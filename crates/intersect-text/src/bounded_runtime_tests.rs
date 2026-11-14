//! Actual imported Ecore/Xtext -> typed native IR -> SAME live Machine controls.
use super::*;
use crate::grammar::Profile;
use crate::package::BundleCompilation;
use intersect_model::core::query::QueryBudget;
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use serde_json::json;
const NS: &str = "urn:intersect:oracle:WholeIterationGate";
const ECORE: &str = include_str!("../../../fixtures/xtext-whole-iteration-v76/GateModel.ecore");
fn input(name: &str, text: &str) -> BundleCompilation {
    let model = DefinitionSource::new(
        "memory:whole-iteration/GateModel.ecore",
        SourceKind::Ecore,
        ECORE,
    );
    let mut grammar = DefinitionSource::new(
        format!("memory:whole-iteration/{name}.xtext"),
        SourceKind::Xtext,
        text,
    );
    grammar.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:whole-iteration/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package emptyRules version \"1\";",
    );
    rules.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: NS.into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: rules.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), grammar, rules],
            namespaces: vec![],
        },
    };
    let migrated = intersect_model::migration::migrate_ecore_bundle_with_limits(
        input.bundle.clone(),
        &[(
            model.id.clone(),
            "memory:whole-iteration/GateModel.rcore".into(),
        )]
        .into(),
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migrated.reports[0].original, model);
    crate::migration::remap_compilation(&migrated, &input).unwrap()
}

fn graph_projection<'a>(
    package: impl Into<crate::package::TextPackageView<'a>>,
    evaluation: &Evaluation,
) -> Result<(Value, Vec<String>), String> {
    let package = package.into();
    let roots: Vec<_> = evaluation
        .model
        .elements
        .iter()
        .filter(|object| object.class == "Root")
        .collect();
    if roots.len() != 1 {
        return Err(format!("Expected exactly one Root, found {}", roots.len()));
    }
    let root = roots[0];
    let children = root
        .values
        .get("children")
        .and_then(Value::as_array)
        .ok_or("Missing actual children collection")?;
    let mut objects = vec![root];
    for child in children {
        let identity = child
            .as_str()
            .ok_or("Child has no native object identity")?;
        let object = evaluation
            .model
            .elements
            .iter()
            .find(|object| object.id == identity)
            .ok_or("Actual containment target missing")?;
        if object.class != "Target" {
            return Err("Actual child has the wrong class".into());
        }
        objects.push(object);
    }
    if objects.len() != evaluation.model.elements.len() {
        return Err("Extra/unowned native objects are not normalized away".into());
    }
    let identities: Vec<_> = objects.iter().map(|object| object.id.clone()).collect();
    let rows: Result<Vec<_>, String> = objects.iter().enumerate().map(|(index, object)| {
        if object.type_identity != package.schema.identity(&object.class) { return Err("Native object classifier identity mismatch".into()); }
        let mut values = serde_json::to_value(&object.values).map_err(|e|e.to_string())?;
        if index == 0 { values["children"] = json!((1..objects.len()).collect::<Vec<_>>()); }
        Ok(json!({"class":format!("{NS}#//{}",object.class),"values":values,"span_bytes":[object.start,object.end]}))
    }).collect();
    Ok((json!(rows?), identities))
}

fn expected_graph(row: &Value) -> Value {
    let raw = row["raw_complete_dto"]["graph"].as_array().unwrap();
    let offsets = row["offset_comparison_only"]["graph"].as_array().unwrap();
    json!(raw.iter().zip(offsets).map(|(object, offset)| {
        let values: serde_json::Map<String, Value> = object["values"].as_object().unwrap().iter().map(|(name, field)|(name.clone(),field["eGet"].clone())).collect();
        json!({"class":object["class"],"values":values,"span_bytes":offset["semantic"]["utf8_bytes"]})
    }).collect::<Vec<_>>())
}

const X: &str = include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationX.xtext");
const CALLER: &str =
    include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationCallerTail.xtext");
fn compiled(name: &str, source: &str) -> Result<compiler::RuntimeGuardedCompilation, String> {
    ordinary::compile_with_runtime_guarded_decisions(
        input(name, source),
        Profile::NativeV80,
        &[],
        None,
        Default::default(),
        Default::default(),
    )
    .map_err(|errors| format!("{errors:?}"))
}
fn run(
    compiled: &compiler::RuntimeGuardedCompilation,
    text: &str,
    allowance: usize,
    spent: usize,
) -> (
    Result<bounded_runtime::RuntimeGuardedEvaluation, String>,
    usize,
) {
    let mut budget = QueryBudget::new(allowance);
    for _ in 0..spent {
        budget.charge().unwrap();
    }
    let result = bounded_runtime::evaluate(
        compiled,
        &[Source {
            uri: "memory:runtime-obligation.input".into(),
            text: text.into(),
        }],
        EvaluationLimits::workbench(),
        &mut budget,
    );
    (result, budget.reads)
}
fn assert_failed(result: Result<bounded_runtime::RuntimeGuardedEvaluation, String>) {
    match result {
        Err(_) => {}
        Ok(actual) => assert!(
            !actual.evaluation.diagnostics.is_empty(),
            "bounded failure became success"
        ),
    }
}
#[test]
fn bounded_runtime_original_iteration_and_caller_full_oracle_outputs_consume_typed_obligations()
-> Result<(), String> {
    let observations: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/oracle-observations.json"
    ))
    .map_err(|error| error.to_string())?;
    let mut count = 0;
    for (name, source) in [("IterationX", X), ("IterationCallerTail", CALLER)] {
        let compiled = compiled(name, source)?;
        let (package, admission) = compiled.parts();
        let grammar = package.grammar.as_ref().ok_or("Missing actual grammar")?;
        let program = grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing native IR")?;
        assert_eq!(admission.decision_count(), 1);
        assert_eq!(program.runtime_obligation_count(), 1);
        assert!(program.requires_runtime_capability());
        assert!(
            package
                .compilation
                .as_ref()
                .ok_or("Missing original compilation")?
                .bundle
                .sources
                .iter()
                .any(|s| s.kind == SourceKind::Xtext && s.text == source)
        );
        for row in observations
            .iter()
            .filter(|row| row["case"]["grammar"] == name)
        {
            let text = row["case"]["text"].as_str().ok_or("Missing oracle input")?;
            let expected = &row["raw_complete_dto"];
            let (actual, used) = run(&compiled, text, 100_000, 0);
            let actual = actual?;
            println!(
                "BOUNDED_RUNTIME_IMPORTED_ORACLE {}",
                json!({
                    "grammar":name,"oracle":row,"actual":actual.evaluation,
                    "native_runtime_obligations":program.runtime_obligation_count(),
                    "actually_consumed":actual.consumed_runtime_decisions(),"caller_reads":used,
                    "prepared_frontiers_awarded":false,"original_sysml_award":false
                })
            );
            let accepted = expected["accepted"]
                .as_bool()
                .ok_or("Missing oracle truth")?;
            assert_eq!(
                actual.evaluation.diagnostics.is_empty(),
                accepted,
                "{name}/{text}"
            );
            assert!(actual.consumed_runtime_decisions() > 0);
            assert!(used > 0 && used <= 100_000);
            if accepted {
                assert_eq!(
                    graph_projection(package, &actual.evaluation)?.0,
                    expected_graph(row)
                );
                let roots = actual
                    .evaluation
                    .model
                    .elements
                    .iter()
                    .filter(|o| o.class == "Root")
                    .collect::<Vec<_>>();
                assert_eq!(roots.len(), 1);
                let expected_children = text.split_whitespace().count() - 1;
                assert_eq!(
                    roots[0].values["children"]
                        .as_array()
                        .ok_or("Missing ordered children")?
                        .len(),
                    expected_children
                );
            }
            count += 1;
        }
    }
    assert_eq!(count, 10);
    Ok(())
}
#[test]
fn bounded_runtime_actual_pipeline_zero_exact_short_preconsumption_and_fresh_owner()
-> Result<(), String> {
    let compiled = compiled("IterationCallerTail", CALLER)?;
    let (baseline, used) = run(&compiled, "x x x", 100_000, 0);
    let baseline = baseline?;
    assert!(baseline.evaluation.diagnostics.is_empty());
    assert!(baseline.consumed_runtime_decisions() > 0);
    assert!(used > 1);
    let before = serde_json::to_value(&baseline.evaluation).map_err(|e| e.to_string())?;
    let (exact, exact_used) = run(&compiled, "x x x", used, 0);
    let exact = exact?;
    assert_eq!(exact_used, used);
    assert_eq!(
        serde_json::to_value(&exact.evaluation).map_err(|e| e.to_string())?,
        before
    );
    let (short, short_used) = run(&compiled, "x x x", used - 1, 0);
    println!(
        "BOUNDED_RUNTIME_EXACT_SHORT {}",
        json!({"exact":exact.evaluation,"used":used,"short_used":short_used,
        "short":short.as_ref().map(|r| &r.evaluation)})
    );
    assert_failed(short);
    assert_eq!(short_used, used - 1);
    assert_failed(run(&compiled, "x x x", 0, 0).0);
    let (preconsumed, pre_used) = run(&compiled, "x x x", used + 7, 7);
    assert_eq!(
        serde_json::to_value(&preconsumed?.evaluation).map_err(|e| e.to_string())?,
        before
    );
    assert_eq!(pre_used, used + 7);
    assert_eq!(
        serde_json::to_value(&run(&compiled, "x x x", used, 0).0?.evaluation)
            .map_err(|e| e.to_string())?,
        before
    );
    Ok(())
}
#[test]
fn bounded_runtime_default_and_deserialized_ir_cannot_install_live_authority() -> Result<(), String>
{
    let compiled = compiled("IterationX", X)?;
    let (package, admission) = compiled.parts();
    let result = evaluate_with_limits(
        &package.to_owned(),
        &[Source {
            uri: "memory:default.input".into(),
            text: "x".into(),
        }],
        EvaluationLimits::workbench(),
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("typed bounded live runtime capability"))
    );
    let grammar = package.grammar.as_ref().ok_or("Missing grammar")?;
    let mut wire: Value = serde_json::to_value(grammar).map_err(|e| e.to_string())?;
    let source =
        intersect_model::definitions::bundle::SourceId::for_text("memory:scope.input", "x");
    // An equal-content reloaded graph still needs the native companion; old constructor rejects.
    let loaded: grammar::Grammar =
        serde_json::from_value(wire.clone()).map_err(|e| e.to_string())?;
    assert!(grammar::LiveCallerPrediction::new(&loaded, &source, &mut Some(100_000)).is_err());
    assert!(
        grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing typed IR")?
            .probe_rule_completion(
                "Root",
                &[grammar::PredictionSymbol::End],
                &mut Some(100_000)
            )
            .is_err()
    );
    let mut native67_marker = wire.clone();
    native67_marker["profile"] =
        serde_json::to_value(Profile::NativeV67).map_err(|e| e.to_string())?;
    let native67_marker: grammar::Grammar =
        serde_json::from_value(native67_marker).map_err(|e| e.to_string())?;
    assert_eq!(native67_marker.profile, Profile::NativeV67);
    assert!(
        grammar::LiveCallerPrediction::new(&native67_marker, &source, &mut Some(100_000)).is_err()
    );
    // Mutate actual serialized source, graph, lexical and obligation dependencies
    // independently. None can obtain live execution from an imported marker.
    let mut mutations = Vec::new();
    let mut graph = wire.clone();
    let old = graph["parser_prediction"]["states"][0]["accepting"]
        .as_bool()
        .ok_or("Missing actual state")?;
    graph["parser_prediction"]["states"][0]["accepting"] = json!(!old);
    mutations.push(("graph", graph));
    let mut source_changed = wire.clone();
    source_changed["declaration_sources"]["Root"]["source"]["uri"] = json!("memory:foreign.xtext");
    mutations.push(("source", source_changed));
    let mut lexical = wire.clone();
    lexical["hidden"] = json!([]);
    mutations.push(("hidden", lexical));
    let mut requirement = wire.clone();
    requirement["parser_prediction"]["runtime_requirement"]["LinearFullRepeats"]["decisions"] =
        json!(["rule/forged/repeat"]);
    mutations.push(("requirement", requirement));
    for (kind, changed) in mutations {
        let changed: grammar::Grammar =
            serde_json::from_value(changed).map_err(|e| e.to_string())?;
        let mut work = Some(100_000);
        assert!(
            grammar::LiveCallerPrediction::new_runtime_guarded(
                &changed, &source, admission, &mut work
            )
            .is_err(),
            "{kind}"
        );
        assert!(work.is_some_and(|left| left < 100_000), "{kind}");
    }
    // Legacy raw IR is an existing trusted input, not a tamper-proof format.
    // Record its actual stripped-marker outcome without inventing rejection.
    let mut stripped_wire = serde_json::to_value(package).map_err(|e| e.to_string())?;
    stripped_wire["grammar"]["parser_prediction"]
        .as_object_mut()
        .ok_or("Missing actual program object")?
        .remove("runtime_requirement");
    let stripped: Package = serde_json::from_value(stripped_wire).map_err(|e| e.to_string())?;
    let legacy = evaluate_with_limits(
        &stripped,
        &[Source {
            uri: "memory:stripped-legacy.input".into(),
            text: "x".into(),
        }],
        EvaluationLimits::workbench(),
    );
    println!(
        "BOUNDED_RUNTIME_STRIPPED_TRUSTED_IR {}",
        json!({
            "legacy_actual": &legacy,
            "trusted_raw_ir_only_not_native_admission": true,
            "typed_companion_manufactured": false,
        })
    );
    let stripped_grammar = stripped
        .grammar
        .as_ref()
        .ok_or("Missing stripped grammar")?;
    assert!(
        !stripped_grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing stripped program")?
            .requires_runtime_capability()
    );
    assert!(
        grammar::LiveCallerPrediction::new_runtime_guarded(
            stripped_grammar,
            &source,
            admission,
            &mut Some(100_000),
        )
        .is_err()
    );
    // Ordinary reconstruction consumes the original retained sources and does
    // not inherit runtime admission from a serialized marker or stripped graph.
    let reconstructed = package.compilation.ok_or("Missing retained compilation")?;
    assert!(
        reconstructed
            .bundle
            .sources
            .iter()
            .any(|s| s.kind == SourceKind::Xtext && s.text == X)
    );
    let original_compilation = serde_json::to_value(reconstructed).map_err(|e| e.to_string())?;
    let ordinary_recompiled = ordinary::compile(reconstructed.to_owned(), Profile::NativeV80);
    println!(
        "BOUNDED_RUNTIME_ORDINARY_SOURCE_RECONSTRUCTION {}",
        json!({
            "actual": &ordinary_recompiled, "explicit_runtime_policy": false,
        })
    );
    // This tiny unchanged fixture is also supported by the existing static route.
    // Recompilation must preserve its source and must not inherit typed authority.
    let ordinary_package = ordinary_recompiled.map_err(|errors| format!("{errors:?}"))?;
    assert_eq!(
        serde_json::to_value(
            ordinary_package
                .compilation
                .as_ref()
                .ok_or("Missing ordinary retained compilation")?
        )
        .map_err(|e| e.to_string())?,
        original_compilation
    );
    let ordinary_grammar = ordinary_package
        .grammar
        .as_ref()
        .ok_or("Missing ordinary grammar")?;
    assert_eq!(ordinary_grammar.profile, Profile::NativeV80);
    let ordinary_program = ordinary_grammar
        .parser_prediction
        .as_ref()
        .ok_or("Missing ordinary native program")?;
    assert!(!ordinary_program.requires_runtime_capability());
    assert_eq!(ordinary_program.runtime_obligation_count(), 0);
    let companion_attempt = grammar::LiveCallerPrediction::new_runtime_guarded(
        ordinary_grammar,
        &source,
        admission,
        &mut Some(100_000),
    );
    println!(
        "BOUNDED_RUNTIME_ORDINARY_COMPANION_ATTEMPT {}",
        json!({"actual":companion_attempt.as_ref().map(|_| "actual owner created")})
    );
    assert!(companion_attempt.is_err());
    let ordinary_evaluation = evaluate_with_limits(
        &ordinary_package,
        &[Source {
            uri: "memory:ordinary-recompiled.input".into(),
            text: "x".into(),
        }],
        EvaluationLimits::workbench(),
    );
    let observations: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/oracle-observations.json"
    ))
    .map_err(|e| e.to_string())?;
    let oracle = observations
        .iter()
        .find(|row| row["case"]["grammar"] == "IterationX" && row["case"]["text"] == "x")
        .ok_or("Missing unchanged ordinary x oracle")?;
    println!(
        "BOUNDED_RUNTIME_ORDINARY_STATIC_ORACLE {}",
        json!({"actual":&ordinary_evaluation,"oracle":oracle,
            "explicit_runtime_policy":false,"typed_obligations":0,
            "new_language_coverage_awarded":false})
    );
    assert_eq!(oracle["raw_complete_dto"]["accepted"], json!(true));
    assert!(ordinary_evaluation.diagnostics.is_empty());
    assert_eq!(
        graph_projection(&ordinary_package, &ordinary_evaluation)?.0,
        expected_graph(oracle)
    );
    // The real companion also rejects a changed source/graph; no imported marker grants proof.
    wire["name"] = json!("foreign-effective-grammar");
    let foreign: grammar::Grammar = serde_json::from_value(wire).map_err(|e| e.to_string())?;
    assert!(
        grammar::LiveCallerPrediction::new_runtime_guarded(
            &foreign,
            &source,
            admission,
            &mut Some(100_000)
        )
        .is_err()
    );
    Ok(())
}

struct TraceInput {
    tokens: Vec<grammar::PredictionSymbol>,
    position: usize,
    furthest: usize,
    rewinds: Vec<(usize, usize)>,
    denied_after_progress: bool,
}
impl TraceInput {
    fn new(text: &str) -> Self {
        let mut tokens = text
            .split_whitespace()
            .map(|s| grammar::PredictionSymbol::Keyword(s.into()))
            .collect::<Vec<_>>();
        tokens.push(grammar::PredictionSymbol::End);
        Self {
            tokens,
            position: 0,
            furthest: 0,
            rewinds: vec![],
            denied_after_progress: false,
        }
    }
}
impl grammar::PredictionInput for TraceInput {
    fn replay_position(&self) -> Option<usize> {
        Some(self.position)
    }
    fn checkpoint(&self) -> Result<usize, String> {
        Ok(self.position)
    }
    fn rewind(&mut self, at: usize) -> Result<(), String> {
        self.rewinds.push((self.position, at));
        self.position = at;
        Ok(())
    }
    fn read(
        &mut self,
        work: &mut Option<usize>,
    ) -> Result<Option<grammar::PredictionSymbol>, String> {
        let left = work.as_mut().ok_or("Missing finite shared input budget")?;
        if *left == 0 {
            self.denied_after_progress |= self.position > 0;
            return Err("Prediction budget exceeded during actual input read".into());
        }
        *left -= 1;
        let value = self.tokens.get(self.position).cloned();
        if value.is_some() {
            self.position += 1;
            self.furthest = self.furthest.max(self.position);
        }
        Ok(value)
    }
}
fn repeat_expr(grammar: &grammar::Grammar) -> &Expr {
    let body = &grammar.rules["Root"].body;
    let Kind::Sequence(items) = &body.kind else {
        panic!("actual imported root sequence")
    };
    &items[1]
}
#[test]
fn bounded_runtime_real_full_probe_rewind_foreign_source_and_sticky_failure() -> Result<(), String>
{
    let compiled = compiled("IterationX", X)?;
    let (package, admission) = compiled.parts();
    let grammar = package.grammar.as_ref().ok_or("Missing actual grammar")?;
    let source =
        intersect_model::definitions::bundle::SourceId::for_text("memory:probe.input", "x");
    let path = format!(
        "{}/repeat",
        repeat_expr(grammar)
            .prediction_path
            .as_deref()
            .ok_or("Missing actual repeat source path")?
    );
    assert!(admission.decision_is_runtime(&path));
    let actual_program = serde_json::to_value(
        grammar
            .parser_prediction
            .as_ref()
            .ok_or("Missing actual prediction program")?,
    )
    .map_err(|e| e.to_string())?;
    assert_eq!(actual_program["decisions"][path.as_str()]["exit"], json!(1));
    let mut work = Some(100_000);
    let mut live =
        grammar::LiveCallerPrediction::new_runtime_guarded(grammar, &source, admission, &mut work)?;
    let mut input = TraceInput::new("x");
    let result = live.predict_shared_returns(
        &source,
        &[],
        repeat_expr(grammar),
        "repeat",
        &mut input,
        &mut work,
    );
    println!(
        "BOUNDED_RUNTIME_REPLAY {}",
        json!({"result":result,"remaining":work,"position":input.position,
        "furthest":input.furthest,"rewinds":input.rewinds})
    );
    // The existing public API normalizes the ordered exit index1 to None.
    // The complete-model oracle separately proves this sole x is the suffix.
    assert_eq!(result?, None);
    assert!(input.rewinds.iter().any(|(from, to)| *from > 0 && *to == 0));
    let foreign =
        intersect_model::definitions::bundle::SourceId::for_text("memory:foreign.input", "x");
    assert!(
        live.predict_shared_returns(
            &foreign,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut input,
            &mut work
        )
        .is_err()
    );
    assert!(live.is_poisoned());
    assert!(
        live.predict_shared_returns(
            &source,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut input,
            &mut work
        )
        .is_err()
    );
    let mut historical_work = Some(100_000);
    let mut historical = grammar::LiveCallerPrediction::new_runtime_guarded(
        grammar,
        &source,
        admission,
        &mut historical_work,
    )?;
    assert!(
        historical
            .predict_historical(
                &source,
                &[],
                repeat_expr(grammar),
                "repeat",
                &mut TraceInput::new("x"),
                &mut historical_work,
            )
            .is_err()
    );
    assert!(historical.is_poisoned());
    let mut fresh_work = Some(100_000);
    let mut fresh = grammar::LiveCallerPrediction::new_runtime_guarded(
        grammar,
        &source,
        admission,
        &mut fresh_work,
    )?;
    assert_eq!(
        fresh.predict_shared_returns(
            &source,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut TraceInput::new("x x"),
            &mut fresh_work
        )?,
        Some(0)
    );
    Ok(())
}
#[test]
fn bounded_runtime_branching_callee_is_outside_the_first_slice_not_silently_deferred() {
    let changed = X.replace("tag='x'", "tag=('x'|'y')");
    assert!(compiled("BranchingUnsupported", &changed).is_err());
    let nullable = X.replace("tag='x'", "tag='x'?");
    assert!(compiled("NullableUnsupported", &nullable).is_err());
    // Only source-transparent unit groups may expose the whole Full guard.
    // These retain a modifier INSIDE the outer star, so unwrapping is unsound.
    for (name, group) in [
        ("InnerOptionalUnsupported", "((=> children+=Target)?)*"),
        ("InnerFirstUnsupported", "(->(=> children+=Target))*"),
    ] {
        let changed = X.replace("(=> children+=Target)*", group);
        assert_ne!(changed, X);
        assert!(compiled(name, &changed).is_err());
    }
}

#[test]
fn bounded_runtime_full_operand_partial_input_and_denied_read_restore_original_cursor()
-> Result<(), String> {
    let source_text = X.replace("tag='x'", "tag='x' 'y'");
    let compiled = compiled("LinearTwoToken", &source_text)?;
    let (package, admission) = compiled.parts();
    let grammar = package.grammar.as_ref().ok_or("Missing grammar")?;
    let source =
        intersect_model::definitions::bundle::SourceId::for_text("memory:two-token.input", "x y x");
    let (positive, _) = run(&compiled, "x y x", 100_000, 0);
    let positive = positive?;
    assert!(positive.evaluation.diagnostics.is_empty());
    assert!(positive.consumed_runtime_decisions() > 0);
    // A prefix alone is NOT a true Full operand. The real suffix keeps its x.
    let partial = run(&compiled, "x", 100_000, 0).0?;
    assert!(partial.evaluation.diagnostics.is_empty());
    let root = partial
        .evaluation
        .model
        .elements
        .iter()
        .find(|o| o.class == "Root")
        .ok_or("Missing root")?;
    assert!(
        root.values["children"]
            .as_array()
            .ok_or("Missing children")?
            .is_empty()
    );
    let mut baseline_work = Some(100_000);
    let mut baseline_scope = grammar::LiveCallerPrediction::new_runtime_guarded(
        grammar,
        &source,
        admission,
        &mut baseline_work,
    )?;
    let constructor_used = 100_000 - baseline_work.ok_or("Missing original pool")?;
    let mut baseline_input = TraceInput::new("x y x");
    assert_eq!(
        baseline_scope.predict_shared_returns(
            &source,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut baseline_input,
            &mut baseline_work
        )?,
        Some(0)
    );
    let prediction_used =
        100_000 - constructor_used - baseline_work.ok_or("Missing shared pool")?;
    let mut witnessed = false;
    for allowance in 1..prediction_used {
        let mut work = Some(constructor_used + allowance);
        let mut scope = grammar::LiveCallerPrediction::new_runtime_guarded(
            grammar, &source, admission, &mut work,
        )?;
        let mut input = TraceInput::new("x y x");
        let result = scope.predict_shared_returns(
            &source,
            &[],
            repeat_expr(grammar),
            "repeat",
            &mut input,
            &mut work,
        );
        if input.denied_after_progress {
            println!(
                "BOUNDED_RUNTIME_REAL_DENIED_READ {}",
                json!({"allowance":allowance,"result":result,"position":input.position,"rewinds":input.rewinds,"remaining":work})
            );
            assert!(result.is_err());
            assert_eq!(work, Some(0));
            assert_eq!(input.position, 0);
            assert!(scope.is_poisoned());
            witnessed = true;
            break;
        }
    }
    assert!(witnessed, "no actual denied consumed probe was exercised");
    Ok(())
}

#[path = "bounded_runtime_scope_tests.rs"]
mod strict_scope;

#[path = "bounded_runtime_observation_tests.rs"]
mod observation_controls;
