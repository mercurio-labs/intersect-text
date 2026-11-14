//! Direct shared-charge controls, with parser-checked authored declaration maps.
//! These are bounded IR walker controls, not admission of a modeling language.
use super::*;
use crate::grammar::AnalysisObservation;
use crate::grammar::Profile;
use crate::grammar::Rule;
use crate::grammar::SourceLimits;
use crate::grammar::admission_observation::*;
use crate::grammar::syntax;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use serde_json::json;

fn expr(kind: Kind) -> Expr {
    Expr {
        kind,
        first_set_predicated: false,
        original_range: None,
        prediction_path: None,
        cardinality: '1',
    }
}
fn fixture(owner: &str) -> (Grammar, DefinitionSource, String, Expr) {
    fixture_sequence(owner, false)
}
fn fixture_sequence(owner: &str, sequence: bool) -> (Grammar, DefinitionSource, String, Expr) {
    let body = if sequence {
        "(Target Target) 'x'"
    } else {
        "child=Target 'x'"
    };
    let text = format!(
        "/* source provenance */\ngrammar intersect.follow.Charge\nimport \"urn:follow\" as m\n{owner} returns m::Node: {body};\nTarget returns m::Node: 'x'?;\n"
    );
    let source = DefinitionSource::new("memory:follow/charge.xtext", SourceKind::Xtext, text);
    let parsed = syntax::parse(
        &source,
        SourceLimits {
            source_bytes: source.text.len(),
            declarations: 2,
        },
    )
    .unwrap();
    assert_eq!(parsed.rules.len(), 2);
    // Check the precise source occurrence represented by the direct walker
    // fixture: a direct assigned call or a parser-mode grouped call sequence,
    // followed by literal'x'. No parser
    // replacement or invented source/owner is used for error metadata.
    let declaration = parsed
        .rules
        .iter()
        .find(|r| r.value.name.value == owner)
        .unwrap();
    let syntax::ElementKind::Sequence(items) = &declaration.value.body.value else {
        panic!("literal source sequence")
    };
    assert_eq!(items.len(), 2);
    let direct = if sequence {
        let syntax::ElementKind::Group(group) = &items[0].value else {
            panic!("literal source group")
        };
        let syntax::ElementKind::Sequence(calls) = &group.value else {
            panic!("literal source call sequence")
        };
        assert_eq!(calls.len(), 2);
        let values: Vec<_> = calls
            .iter()
            .map(|call| {
                let syntax::ElementKind::Call { rule, .. } = &call.value else {
                    panic!("literal grouped call")
                };
                assert_eq!(rule.value, "Target");
                expr(Kind::Call(rule.value.clone()))
            })
            .collect();
        expr(Kind::Sequence(values))
    } else {
        let syntax::ElementKind::Assignment {
            feature,
            operator,
            value,
        } = &items[0].value
        else {
            panic!("literal source assignment")
        };
        assert_eq!(feature.value, "child");
        assert_eq!(operator, "=");
        let syntax::ElementKind::Call { rule, .. } = &value.value else {
            panic!("literal source call")
        };
        assert_eq!(rule.value, "Target");
        expr(Kind::Call(rule.value.clone()))
    };
    let syntax::ElementKind::Keyword(keyword) = &items[1].value else {
        panic!("literal caller continuation")
    };
    assert_eq!(keyword.to_scalar_string().unwrap(), "x");
    let target = parsed
        .rules
        .iter()
        .find(|r| r.value.name.value == "Target")
        .unwrap();
    let syntax::ElementKind::Cardinality { element, operator } = &target.value.body.value else {
        panic!("literal nullable target")
    };
    assert_eq!(*operator, '?');
    assert!(
        matches!(&element.value,syntax::ElementKind::Keyword(k) if k.to_scalar_string().unwrap()=="x")
    );
    let mut grammar: Grammar = serde_json::from_value(
        json!({"profile":Profile::NativeV71,"name":"Charge","entry":owner,"rules":{}}),
    )
    .unwrap();
    let mut optional = expr(Kind::Literal("x".into()));
    optional.cardinality = '?';
    let target: Rule = serde_json::from_value(json!({"body":optional,"class":"Node"})).unwrap();
    grammar.rules.insert("Target".into(), target);
    for r in &parsed.rules {
        grammar.declaration_sources.insert(
            r.value.name.value.clone(),
            SourceSpan {
                source: source.id.clone(),
                start: r.start,
                end: r.end,
            },
        );
    }
    let path = if sequence {
        format!("rule/{owner}/s0")
    } else {
        format!("rule/{owner}/s0/value")
    };
    (grammar, source, path, direct)
}
fn context<'a>(
    observation: &'a PredictionObservation,
    source: &'a DefinitionSource,
) -> AnalysisObservation<'a> {
    AnalysisObservation {
        recursive_work: None,
        adjacent_work: None,
        observation,
        entry: &source.id,
        stage: RunStage::Executable,
    }
}
fn run(
    grammar: &Grammar,
    body: &Expr,
    path: &str,
    source: &DefinitionSource,
    observation: Option<&PredictionObservation>,
    work: &mut usize,
    required: &mut Vec<RequiredObjectCall>,
) -> Result<(), String> {
    let owner = path
        .strip_prefix("rule/")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    walk(
        body,
        &Tokens {
            symbols: ["literal:x".into()].into(),
            caller: false,
        },
        grammar,
        &mut BTreeMap::new(),
        (
            true,
            path,
            observation.map(|o| WalkObservation {
                context: context(o, source),
                owner,
            }),
            Some(owner),
        ),
        work,
        (&mut BTreeSet::new(), Some(required)),
    )
}
fn projected(required: &[RequiredObjectCall]) -> Vec<(&str, &str, &str)> {
    required
        .iter()
        .map(|r| (&*r.owner, &*r.callee, &*r.path))
        .collect()
}
fn no_inferred(failure: PhaseFailure) {
    assert_eq!(failure.syntax_depth, None);
    assert_eq!(failure.call_depth, None);
    assert_eq!(failure.state_count, None);
    assert_eq!(failure.repeated_state_count, None);
}

#[test]
fn follow_charge_denials_keep_literal_work_order_outputs_and_parser_checked_source_identity() {
    let (grammar, source, path, body) = fixture("Root");
    assert_eq!(
        source.id.sha256,
        "c391f71aff4c9e7f420f6319236ecfb09908f96ba00eb384daa468b1b9f9f91e"
    );
    assert_eq!(
        (
            grammar.declaration_sources["Root"].start,
            grammar.declaration_sources["Root"].end
        ),
        (81, 120)
    );
    let before = serde_json::to_value(&grammar).unwrap();
    for (limit, phase) in [
        (0, Phase::FollowWalk),
        (1, Phase::FollowRequiredLookup),
        (2, Phase::FollowRequiredRetention),
    ] {
        let mut plain_work = limit;
        let mut observed_work = limit;
        let mut plain = Vec::new();
        let mut observed = Vec::new();
        let trace = PredictionObservation::new();
        let old = run(
            &grammar,
            &body,
            &path,
            &source,
            None,
            &mut plain_work,
            &mut plain,
        );
        let new = run(
            &grammar,
            &body,
            &path,
            &source,
            Some(&trace),
            &mut observed_work,
            &mut observed,
        );
        assert_eq!(old, Err("Grammar prediction budget exceeded".into()));
        assert_eq!(new, old);
        assert_eq!((plain_work, observed_work), (0, 0));
        assert_eq!(projected(&plain), projected(&observed));
        assert!(plain.is_empty());
        let failure = trace.first_failure().unwrap();
        assert_eq!(failure.phase, phase);
        assert_eq!(failure.stage, RunStage::Executable);
        assert_eq!(failure.owner_rule.unwrap().as_str().unwrap(), "Root");
        assert_eq!(failure.source.unwrap().uri.as_str().unwrap(), source.id.uri);
        assert_eq!(
            failure.source.unwrap().sha256.as_str().unwrap(),
            source.id.sha256
        );
        assert_eq!(failure.declaration_span, Some((81, 120)));
        assert_eq!(
            failure.decision.unwrap().as_str().unwrap(),
            "rule/Root/s0/value"
        );
        assert_eq!(
            failure.charged_work, None,
            "lower walker cannot invent outer FOLLOW accounting"
        );
        no_inferred(failure);
        // Adapter-only attachment of this direct fixture's actually spent work;
        // the production outer method still uses its unchanged10,000 bucket.
        trace.follow_measurements(limit - observed_work);
        assert_eq!(trace.first_failure().unwrap().charged_work, Some(limit));
    }
    for observed in [false, true] {
        let trace = PredictionObservation::new();
        let mut work = 3;
        let mut required = Vec::new();
        assert_eq!(
            run(
                &grammar,
                &body,
                &path,
                &source,
                observed.then_some(&trace),
                &mut work,
                &mut required
            ),
            Ok(())
        );
        assert_eq!(work, 0);
        assert_eq!(
            projected(&required),
            vec![("Root", "Target", "rule/Root/s0/value")]
        );
        assert_eq!(trace.first_failure(), None);
    }
    assert_eq!(serde_json::to_value(grammar).unwrap(), before);
}

#[test]
fn follow_charge_partial_pending_order_and_error_retry_match_none_path() {
    let (grammar, source, path, body) = fixture_sequence("Root", true);
    for _ in 0..2 {
        let mut plain = Vec::new();
        let mut observed = Vec::new();
        let mut old_work = 6;
        let mut new_work = 6;
        let trace = PredictionObservation::new();
        let old = run(
            &grammar,
            &body,
            &path,
            &source,
            None,
            &mut old_work,
            &mut plain,
        );
        let new = run(
            &grammar,
            &body,
            &path,
            &source,
            Some(&trace),
            &mut new_work,
            &mut observed,
        );
        assert_eq!(old, Err("Grammar prediction budget exceeded".into()));
        assert_eq!(new, old);
        assert_eq!((old_work, new_work), (0, 0));
        assert_eq!(projected(&plain), projected(&observed));
        assert_eq!(
            projected(&plain),
            vec![("Root", "Target", "rule/Root/s0/s1")]
        );
        let failure = trace.first_failure().unwrap();
        assert_eq!(failure.phase, Phase::FollowRequiredRetention);
        assert_eq!(
            failure.decision.unwrap().as_str().unwrap(),
            "rule/Root/s0/s0"
        );
        no_inferred(failure);
    }
}

#[test]
fn follow_charge_first_query_failure_remains_distinct_and_outer_fallback_truthful() {
    let (mut grammar, source, path, body) = fixture("Root");
    // Synthetic recursive IR is explicitly unbound: it is a FIRST allowance
    // fault control, never attributed to the retained authored declaration.
    grammar.declaration_sources.clear();
    grammar.rules.get_mut("Target").unwrap().body = expr(Kind::Call("Target".into()));
    let mut plain_work = 4;
    let mut observed_work = 4;
    let trace = PredictionObservation::new();
    let plain = run(
        &grammar,
        &body,
        &path,
        &source,
        None,
        &mut plain_work,
        &mut Vec::new(),
    );
    let observed = run(
        &grammar,
        &body,
        &path,
        &source,
        Some(&trace),
        &mut observed_work,
        &mut Vec::new(),
    );
    assert_eq!(plain, Err("Grammar prediction budget exceeded".into()));
    assert_eq!(observed, plain);
    assert_eq!((plain_work, observed_work), (3, 3));
    assert_eq!(
        trace.first_failure(),
        None,
        "successful FOLLOW entry charge is not a denied charge"
    );
    assert_eq!(
        super::super::observe_phase(
            observed,
            &grammar,
            Some(context(&trace, &source)),
            Phase::Follow,
            None,
            None
        ),
        plain
    );
    let failure = trace.first_failure().unwrap();
    assert_eq!(failure.phase, Phase::Follow);
    assert_eq!(failure.owner_rule, None);
    assert_eq!(failure.decision, None);
    assert_eq!(failure.declaration_span, None);
    assert_eq!(failure.charged_work, None);
    no_inferred(failure);
}

#[test]
fn follow_charge_missing_and_bounded_origin_capture_does_not_fabricate_identity() {
    let (mut grammar, source, path, body) = fixture("Root");
    grammar.declaration_sources.remove("Root");
    let trace = PredictionObservation::new();
    assert!(
        run(
            &grammar,
            &body,
            &path,
            &source,
            Some(&trace),
            &mut 0,
            &mut Vec::new()
        )
        .is_err()
    );
    let failure = trace.first_failure().unwrap();
    assert_eq!(failure.phase, Phase::FollowWalk);
    assert_eq!(failure.owner_rule, None);
    assert_eq!(failure.declaration_span, None);
    assert_eq!(
        failure.source.unwrap().sha256.as_str().unwrap(),
        source.id.sha256
    );
    no_inferred(failure);
    let owner = "R".repeat(260);
    let (grammar, source, path, body) = fixture(&owner);
    let trace = PredictionObservation::new();
    assert!(
        run(
            &grammar,
            &body,
            &path,
            &source,
            Some(&trace),
            &mut 0,
            &mut Vec::new()
        )
        .is_err()
    );
    let failure = trace.first_failure().unwrap();
    assert_eq!(failure.owner_rule, None);
    assert_eq!(failure.decision, None);
    assert!(failure.declaration_span.is_some());
    assert!(failure.capture_unavailable);
    assert_eq!(
        failure.source.unwrap().sha256.as_str().unwrap(),
        source.id.sha256
    );
    no_inferred(failure);
}

#[test]
fn follow_charge_does_not_replace_existing_first_failure_and_debug_host_projection_is_exact() {
    let (grammar, source, path, body) = fixture("Root");
    let trace = PredictionObservation::new();
    trace.returned_error(
        &source.id,
        RunStage::Parent,
        Phase::RuleConstruction,
        None,
        None,
    );
    let before = trace.first_failure();
    assert!(
        run(
            &grammar,
            &body,
            &path,
            &source,
            Some(&trace),
            &mut 0,
            &mut Vec::new()
        )
        .is_err()
    );
    assert_eq!(trace.first_failure(), before);
    assert_eq!(format!("{:?}", Phase::Follow), "Follow");
    assert_eq!(format!("{:?}", Phase::FollowWalk), "FollowWalk");
    assert_eq!(
        format!("{:?}", Phase::FollowRequiredLookup),
        "FollowRequiredLookup"
    );
    assert_eq!(
        format!("{:?}", Phase::FollowRequiredRetention),
        "FollowRequiredRetention"
    );
    assert_eq!(Phase::Follow as usize, 10);
    assert_eq!(
        Phase::ParserRecursiveAnalysis as usize,
        19,
        "old discriminator prefix remains exact"
    );
    // 83e72b9b deliberately inserted the three runtime-candidate phases
    // (RuntimeNullableInventory, RuntimeCandidateBinding,
    // RuntimeCandidateNonnullableFirst) before DeferredProof; FOLLOW charge
    // phases stay appended after it.
    assert_eq!(Phase::DeferredProof as usize, 23);
    assert_eq!(Phase::FollowWalk as usize, 24);
}

#[allow(dead_code)]
enum LegacyPhase {
    DatatypeInference,
    TerminalPreparation,
    TerminalCheck,
    EntryCategory,
    FragmentCycles,
    RuleConstruction,
    LeftRecursion,
    PredictionCheck,
    FragmentCallers,
    FragmentCallScan,
    Follow,
    LexicalProgram,
    LexicalReachability,
    ParserProgram,
    ParserGuardCollection,
    ParserRuleStates,
    ParserEntryStates,
    ParserReachability,
    ParserRuleGraph,
    ParserRecursiveAnalysis,
    // Inserted by 83e72b9b (explicit runtime candidate admission).
    RuntimeNullableInventory,
    RuntimeCandidateBinding,
    RuntimeCandidateNonnullableFirst,
    DeferredProof,
}
#[allow(dead_code)]
struct LegacyPhaseFailure {
    pub stage: RunStage,
    pub phase: LegacyPhase,
    /// A declaration source when proven; otherwise the actual analysis entry.
    pub source: Option<CapturedSource>,
    pub owner_rule: Option<CapturedText<128>>,
    pub declaration_span: Option<(usize, usize)>,
    pub decision: Option<CapturedText<256>>,
    pub capture_unavailable: bool,
    /// This phase-only increment does not fabricate measurements of inner guards.
    pub charged_work: Option<usize>,
    pub syntax_depth: Option<usize>,
    pub call_depth: Option<usize>,
    pub state_count: Option<usize>,
    pub repeated_state_count: Option<usize>,
}
#[allow(dead_code)]
struct LegacyPredictionObservation {
    coverage: std::cell::Cell<Coverage>,
    first: std::cell::Cell<Option<LegacyPhaseFailure>>,
    // Deliberate passive FIRST reuse / FIRST map cost records added by
    // 83e72b9b and b9b150f8; the FOLLOW charge itself adds no field.
    first_reuse: std::cell::Cell<Option<FirstReuseWorkSnapshot>>,
    first_map_cost_enabled: std::cell::Cell<bool>,
    first_map_cost: std::cell::Cell<Option<FirstMapCostSnapshot>>,
}
#[test]
fn follow_charge_observer_native_layout_matches_before_and_has_one_bounded_record() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<Phase>(), size_of::<LegacyPhase>());
    assert_eq!(align_of::<Phase>(), align_of::<LegacyPhase>());
    assert_eq!(size_of::<PhaseFailure>(), size_of::<LegacyPhaseFailure>());
    assert_eq!(align_of::<PhaseFailure>(), align_of::<LegacyPhaseFailure>());
    assert_eq!(
        size_of::<PredictionObservation>(),
        size_of::<LegacyPredictionObservation>()
    );
    assert_eq!(
        align_of::<PredictionObservation>(),
        align_of::<LegacyPredictionObservation>()
    );
    println!(
        "FOLLOW_CHARGE_OBSERVER_LAYOUT Phase={}/{} PhaseFailure={}/{} Observation={}/{}",
        size_of::<Phase>(),
        align_of::<Phase>(),
        size_of::<PhaseFailure>(),
        align_of::<PhaseFailure>(),
        size_of::<PredictionObservation>(),
        align_of::<PredictionObservation>()
    );
}
