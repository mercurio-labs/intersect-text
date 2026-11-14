//! Unexecuted NativeV77 productive nullable slice controls; original sources exact.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::package::BundleCompilation;
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
    evaluate_with_limits, ordinary,
};
use serde_json::{Value, json};
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
    intersect_text::migration::remap_compilation(&migrated, &input).unwrap()
}

const NULLABLE: &str =
    include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationNullable.xtext");
#[test]
fn nullable_full_original_three_cases_preserve_genuine_recovered_graphs_and_invalid_model() {
    for combined_profile_value in [
        Profile::NativeV77,
        Profile::NativeV78,
        Profile::NativeV79,
        Profile::NativeV80,
    ] {
        println!(
            "COMBINED_NULLABLE_PROFILE {}",
            combined_profile_value.as_str()
        );
        run_nullable_full_original_three_cases_preserve_genuine_recovered_graphs_and_invalid_model(
            combined_profile_value,
        );
    }
}
fn run_nullable_full_original_three_cases_preserve_genuine_recovered_graphs_and_invalid_model(
    combined_profile_value: Profile,
) {
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/oracle-observations.json"
    ))
    .unwrap();
    let compiled = ordinary::compile(input("IterationNullable", NULLABLE), combined_profile_value);
    println!(
        "V77_NULLABLE_COMPILE_PREASSERT {}",
        serde_json::to_string(&compiled).unwrap()
    );
    let package = compiled.unwrap();
    assert_eq!(package.schema.uri, NS);
    assert_eq!(package.grammar.as_ref().unwrap().entry, "Root");
    // The genuine importer retains the authored group, with '*' on its
    // outer singleton Sequence. Bind that exact canonical node and inner Call.
    let grammar = package.grammar.as_ref().unwrap();
    let intersect_text::grammar::Kind::Sequence(root) = &grammar.rules["Root"].body.kind else {
        panic!("Expected original Root sequence")
    };
    let grouped = &root[1];
    assert_eq!(grouped.cardinality, '*');
    let intersect_text::grammar::Kind::Sequence(body) = &grouped.kind else {
        panic!("Original authored group was not retained")
    };
    assert_eq!(body.len(), 1);
    assert!(matches!(
        body[0].kind,
        intersect_text::grammar::Kind::SyntacticPredicate(_)
    ));
    let mut source_work = 10000;
    let proof = grammar
        .qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut source_work)
        .unwrap()
        .unwrap();
    assert!(std::ptr::eq(proof.expression(), grouped));
    assert_eq!(proof.callee(), "Target");
    assert_eq!(10000 - source_work, 11);
    println!("V77_IMPORTED_SOURCE_PROOF {}", serde_json::to_string(&json!({"owner":proof.owner(),"callee":proof.callee(),"source":grammar.declaration_sources["Root"].source,"outer_range":grouped.original_range,"call_range":proof.call_expression().original_range,"charged_work":10000-source_work})).unwrap());
    let language =
        DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
            .unwrap();
    let mut outcomes = Vec::new();
    let mut captured = Vec::new();
    for row in rows
        .iter()
        .filter(|row| row["case"]["grammar"] == "IterationNullable")
    {
        let index = row["case"]["index"].as_u64().unwrap();
        let raw = &row["raw_complete_dto"];
        let source = Source {
            uri: format!("memory:whole-iteration/IterationNullable-{index}.model"),
            text: raw["text"].as_str().unwrap().into(),
        };
        let ordinary = evaluate_with_limits(
            &package,
            std::slice::from_ref(&source),
            EvaluationLimits::workbench(),
        );
        let document_result = language.analyze(&source, None, EvaluationLimits::workbench());
        println!("V77_NULLABLE_CASE_PREASSERT {}", serde_json::to_string(&json!({"index":index,"source":&source,"ordinary":&ordinary,"document_result":&document_result,"oracle":row})).unwrap());
        captured.push((row, source, ordinary, document_result));
    }
    // Capture each original case exactly once before assertion failure can
    // prevent the other genuine cases' full DTOs from being retained.
    for (row, source, ordinary, document_result) in captured {
        let index = row["case"]["index"].as_u64().unwrap();
        let raw = &row["raw_complete_dto"];
        assert_eq!(
            serde_json::to_value(&ordinary.diagnostics).unwrap(),
            json!([{"source":source.uri,"start":source.text.len(),"end":source.text.len(),"message":"Expected 'x'"}])
        );
        assert!(
            ordinary.model.elements.is_empty(),
            "Rejected resource remains transactional"
        );
        let actual = document_result.unwrap();
        assert!(actual.valid_model.is_none());
        let recovered = actual.recovered.as_ref().unwrap();
        let roots: Vec<_> = recovered
            .objects
            .iter()
            .filter(|object| object.class() == "Root")
            .collect();
        assert_eq!(roots.len(), 1);
        let root = roots[0];
        let children = root.values()["children"].as_array().unwrap();
        assert_eq!(
            children.len(),
            index as usize,
            "No EOF empty iteration object, exact0/1/2 consuming children"
        );
        let mut objects = vec![root];
        for child in children {
            let object = recovered
                .objects
                .iter()
                .find(|object| object.id() == child.as_str().unwrap())
                .unwrap();
            assert_eq!(object.class(), "Target");
            objects.push(object);
        }
        assert_eq!(objects.len(), recovered.objects.len());
        let graph:Vec<_>=objects.iter().enumerate().map(|(ordinal,object)| {
            assert_eq!(object.type_identity(),package.schema.identity(object.class()));let mut values=serde_json::to_value(object.values()).unwrap();if ordinal==0 {values["children"]=json!((1..objects.len()).collect::<Vec<_>>());}
            json!({"class":format!("{NS}#//{}",object.class()),"values":values,"span_bytes":[object.start(),object.end()]})
        }).collect();
        let expected:Vec<_>=raw["graph"].as_array().unwrap().iter().zip(row["offset_comparison_only"]["graph"].as_array().unwrap()).map(|(object,offset)|{
            let values:serde_json::Map<String,Value>=object["values"].as_object().unwrap().iter().map(|(name,value)|(name.clone(),value["eGet"].clone())).collect();json!({"class":object["class"],"values":values,"span_bytes":offset["semantic"]["utf8_bytes"]})
        }).collect();
        assert_eq!(
            graph, expected,
            "Original genuine nullable class/value/containment/object span mismatch"
        );
        let diagnostics: Vec<_> = actual
            .diagnostics
            .iter()
            .map(|issue| json!({"message":issue.message,"start":issue.start,"end":issue.end}))
            .collect();
        let expected:Vec<_>=row["offset_comparison_only"]["diagnostics"].as_array().unwrap().iter().map(|issue|json!({"message":issue["raw"]["message"],"start":issue["span"]["utf8_bytes"][0],"end":issue["span"]["utf8_bytes"][1]})).collect();
        assert_eq!(diagnostics, expected);
        let ids: Vec<_> = objects.iter().map(|object| object.id()).collect();
        let mut features = Vec::new();
        let mut expected_features = Vec::new();
        for feature in actual.snapshot.feature_nodes() {
            let owner = &actual.snapshot.semantic_objects()[feature.owner_object];
            let ordinal = ids.iter().position(|id| *id == owner.id.as_str()).unwrap();
            let node = &actual.snapshot.document().nodes()[feature.node.0];
            assert_eq!(
                &owner.classifier,
                package.schema.declarations.get(&owner.class).unwrap()
            );
            assert_eq!(
                feature.identity,
                package
                    .schema
                    .effective_feature(&owner.class, &feature.feature)
                    .unwrap()
                    .identity
            );
            features.push(json!([
                ordinal,
                feature.feature,
                [node.range.start, node.range.end],
                [node.total_range.start, node.total_range.end]
            ]));
        }
        for object in raw["graph"].as_array().unwrap() {
            for field in object["feature_nodes"].as_array().unwrap() {
                for node in field["nodes"].as_array().unwrap() {
                    expected_features.push(json!([
                        object["id"],
                        field["member"]["feature"],
                        [node["start"], node["end"]],
                        [node["total_start"], node["total_end"]]
                    ]));
                }
            }
        }
        features.sort_by_key(|row| serde_json::to_string(row).unwrap());
        expected_features.sort_by_key(|row| serde_json::to_string(row).unwrap());
        assert_eq!(features, expected_features);
        let node = &actual.snapshot.document().nodes()[0];
        assert_eq!(
            (node.total_range.start, node.total_range.end),
            (0, source.text.encode_utf16().count())
        );
        outcomes.push(json!({"index":index,"source":{"uri":source.uri,"text":source.text},"ordinary":ordinary,"document":actual,"oracle_complete":raw,"oracle_offsets":row["offset_comparison_only"]}));
    }
    assert_eq!(outcomes.len(), 3);
    println!(
        "V77_NULLABLE_GENUINE3 {}",
        serde_json::to_string(&outcomes).unwrap()
    );
}

#[test]
fn productive_profile_does_not_change_any_old_nullable_rejection_or_default() {
    for profile in [
        Profile::NativeV68,
        Profile::NativeV69,
        Profile::NativeV70,
        Profile::NativeV71,
        Profile::NativeV72,
        Profile::NativeV73,
        Profile::NativeV74,
        Profile::NativeV75,
        Profile::NativeV76,
    ] {
        let errors = ordinary::compile(input("IterationNullable", NULLABLE), profile)
            .err()
            .unwrap();
        assert_eq!(
            serde_json::to_value(errors).unwrap(),
            json!([{"source":"memory:whole-iteration/IterationNullable.xtext","start":177,"end":242,"message":"Nullable repetitions are unsupported"}])
        );
    }
    assert_eq!(
        serde_json::to_value(Profile::NativeV77).unwrap(),
        "intersect.xtext.native.v77"
    );
    assert_eq!(std::mem::size_of::<Profile>(), 1);
}
#[test]
fn new_profile_other_existing_oracle_grammars_keep_whole_native_runtime_dtos() {
    for combined_profile_value in [
        Profile::NativeV77,
        Profile::NativeV78,
        Profile::NativeV79,
        Profile::NativeV80,
    ] {
        println!(
            "COMBINED_NULLABLE_PROFILE {}",
            combined_profile_value.as_str()
        );
        run_new_profile_other_existing_oracle_grammars_keep_whole_native_runtime_dtos(
            combined_profile_value,
        );
    }
}
fn run_new_profile_other_existing_oracle_grammars_keep_whole_native_runtime_dtos(
    combined_profile_value: Profile,
) {
    let baseline: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/negative-native-baseline-v76.json"
    ))
    .unwrap();
    // Five negative contracts are separately fully pinned; their behavior under
    //77 must also stay exact. New surrounding caller grammars are not invented
    //independent oracles by this cross-profile control.
    let mut complete = Vec::new();
    let mut positives = 0;
    let mut captured = Vec::new();
    for (name, text) in [
        (
            "IterationX",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationX.xtext"),
        ),
        (
            "IterationXY",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationXY.xtext"),
        ),
        (
            "IterationNested",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationNested.xtext"),
        ),
        (
            "IterationCallerTail",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationCallerTail.xtext"),
        ),
        (
            "IterationHidden",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationHidden.xtext"),
        ),
    ] {
        let old_compiled = ordinary::compile(input(name, text), Profile::NativeV76);
        let new_compiled = ordinary::compile(input(name, text), combined_profile_value);
        println!("V77_SURROUNDING_COMPILE_PREASSERT {}",serde_json::to_string(&json!({"grammar":name,"baseline_native_v76":&old_compiled,"native_v77":&new_compiled})).unwrap());
        let old = old_compiled.unwrap();
        let new = new_compiled.unwrap();
        let rows: Vec<Value> = serde_json::from_str(include_str!(
            "../../../fixtures/xtext-whole-iteration-v76/oracle-observations.json"
        ))
        .unwrap();
        let old_language =
            DocumentLanguage::prepare(&old, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        let new_language =
            DocumentLanguage::prepare(&new, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        for row in rows.iter().filter(|row| row["case"]["grammar"] == name) {
            let index = row["case"]["index"].as_u64().unwrap();
            let source = Source {
                uri: format!("memory:whole-iteration/{name}-{index}.model"),
                text: row["raw_complete_dto"]["text"].as_str().unwrap().into(),
            };
            let before = evaluate_with_limits(
                &old,
                std::slice::from_ref(&source),
                EvaluationLimits::workbench(),
            );
            let after = evaluate_with_limits(
                &new,
                std::slice::from_ref(&source),
                EvaluationLimits::workbench(),
            );
            let old_document = old_language.analyze(&source, None, EvaluationLimits::workbench());
            let new_document = new_language.analyze(&source, None, EvaluationLimits::workbench());
            println!("V77_SURROUNDING_CASE_PREASSERT {}",serde_json::to_string(&json!({"grammar":name,"index":index,"source":&source,"baseline_native_v76":&before,"native_v77":&after,"baseline_document_v76":&old_document,"document_v77":&new_document,"oracle":row})).unwrap());
            captured.push((
                name,
                index,
                row.clone(),
                before,
                after,
                old_document,
                new_document,
            ));
        }
    }
    for (name, index, row, before, after, old_document, new_document) in captured {
        assert_eq!(
            serde_json::to_value(&before).unwrap(),
            serde_json::to_value(&after).unwrap()
        );
        let old_document = old_document.unwrap();
        let new_document = new_document.unwrap();
        assert_eq!(
            serde_json::to_value(&old_document).unwrap(),
            serde_json::to_value(&new_document).unwrap()
        );
        if row["raw_complete_dto"]["errors"]
            .as_array()
            .unwrap()
            .is_empty()
        {
            positives += 1;
            assert!(after.diagnostics.is_empty());
            assert!(new_document.valid_model.is_some());
        }
        complete.push(json!({"grammar":name,"index":index,"cached_oracle":row,"baseline_native_v76":&before,"native_v77":&after,"baseline_document_v76":&old_document,"document_v77":&new_document}));
        if let Some(prior) = baseline["outcomes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|prior| prior["grammar"] == name && prior["index"] == index)
        {
            assert_eq!(serde_json::to_value(&after).unwrap(), prior["native"]);
        }
    }
    assert_eq!(complete.len(), 25);
    assert_eq!(positives, 20);
    println!(
        "V77_SURROUNDING25_COMPLETE {}",
        serde_json::to_string(&complete).unwrap()
    );
}

#[test]
fn nullable_progress_does_not_count_hidden_trivia_or_lexical_recovery_surrogate_units() {
    for combined_profile_value in [
        Profile::NativeV77,
        Profile::NativeV78,
        Profile::NativeV79,
        Profile::NativeV80,
    ] {
        println!(
            "COMBINED_NULLABLE_PROFILE {}",
            combined_profile_value.as_str()
        );
        run_nullable_progress_does_not_count_hidden_trivia_or_lexical_recovery_surrogate_units(
            combined_profile_value,
        );
    }
}
fn run_nullable_progress_does_not_count_hidden_trivia_or_lexical_recovery_surrogate_units(
    combined_profile_value: Profile,
) {
    let package =
        ordinary::compile(input("IterationNullable", NULLABLE), combined_profile_value).unwrap();
    let language =
        DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
            .unwrap();
    // Authored progress/source-boundary controls, not additional independent
    // Java observations. The original common-source grammar stays byte-exact.
    for text in [" ", "\t\r\n", " \tx", "😀", "😀 x"] {
        let source = Source {
            uri: "memory:v77/progress.model".into(),
            text: text.into(),
        };
        let actual = language
            .analyze(&source, None, EvaluationLimits::workbench())
            .unwrap();
        assert!(actual.valid_model.is_none());
        let recovered = actual.recovered.as_ref().unwrap();
        let root = recovered
            .objects
            .iter()
            .find(|object| object.class() == "Root")
            .unwrap();
        let children = root.values()["children"].as_array().unwrap();
        let count = usize::from(text.contains('x'));
        assert_eq!(children.len(), count);
        assert_eq!(recovered.objects.len(), count + 1);
        if let Some(at) = text.find('x') {
            let target = recovered
                .objects
                .iter()
                .find(|object| object.class() == "Target")
                .unwrap();
            assert_eq!((target.start(), target.end()), (at, at + 1));
            let expected = text[..at].encode_utf16().count();
            let feature = actual
                .snapshot
                .feature_nodes()
                .iter()
                .find(|feature| feature.feature == "tag")
                .unwrap();
            let node = &actual.snapshot.document().nodes()[feature.node.0];
            assert_eq!((node.range.start, node.range.end), (expected, expected + 1));
        }
        let node = &actual.snapshot.document().nodes()[0];
        assert_eq!(
            (node.total_range.start, node.total_range.end),
            (0, text.encode_utf16().count())
        );
        assert_eq!(
            actual.snapshot.document().source().sha256,
            intersect_model::definitions::bundle::SourceId::for_text(&source.uri, text).sha256
        );
        if text.contains('😀') {
            assert!(!actual.snapshot.lexical_issues().is_empty());
        }
    }
}

#[test]
fn qualified_target_other_required_caller_keeps_real_empty_object_and_consuming_choice() {
    for combined_profile_value in [
        Profile::NativeV77,
        Profile::NativeV78,
        Profile::NativeV79,
        Profile::NativeV80,
    ] {
        println!(
            "COMBINED_NULLABLE_PROFILE {}",
            combined_profile_value.as_str()
        );
        run_qualified_target_other_required_caller_keeps_real_empty_object_and_consuming_choice(
            combined_profile_value,
        );
    }
}
fn run_qualified_target_other_required_caller_keeps_real_empty_object_and_consuming_choice(
    combined_profile_value: Profile,
) {
    const OTHER: &str = "grammar intersect.iteration.controls.OtherCaller hidden(WS)\nimport \"urn:intersect:oracle:WholeIterationGate\" as gate\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as Ecore\nRoot returns gate::Root: {gate::Root} children+=Target 'z';\nTarget returns gate::Target: {gate::Target} tag='x'?;\nterminal WS: (' ' | '\\t' | '\\r' | '\\n')+;\n";
    let old = ordinary::compile(input("OtherCaller", OTHER), Profile::NativeV76).unwrap();
    let new = ordinary::compile(input("OtherCaller", OTHER), combined_profile_value).unwrap();
    let mut outcomes = Vec::new();
    for (text, tag, span) in [("x z", json!("x"), (0, 1)), ("z", Value::Null, (0, 0))] {
        let source = Source {
            uri: "memory:v77/other-caller.model".into(),
            text: text.into(),
        };
        let before = evaluate_with_limits(
            &old,
            std::slice::from_ref(&source),
            EvaluationLimits::workbench(),
        );
        let actual = evaluate_with_limits(
            &new,
            std::slice::from_ref(&source),
            EvaluationLimits::workbench(),
        );
        assert!(actual.diagnostics.is_empty());
        assert_eq!(
            serde_json::to_value(&before).unwrap(),
            serde_json::to_value(&actual).unwrap()
        );
        let root = actual
            .model
            .elements
            .iter()
            .find(|object| object.class == "Root")
            .unwrap();
        let children = root.values["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        let target = actual
            .model
            .elements
            .iter()
            .find(|object| object.id == children[0].as_str().unwrap())
            .unwrap();
        assert_eq!(target.values["tag"], tag);
        assert_eq!((target.start, target.end), span);
        outcomes.push(json!({"source":{"uri":source.uri,"text":source.text},"actual":actual}));
    }
    println!("V77_OTHER_CALLER_AUTHORED {}",serde_json::to_string(&json!({"scope":"Authored construction and old-profile parity control; generated Target local-choice source is the independent local basis. No independent whole OtherCaller Java oracle or general other-caller language award.","outcomes":outcomes})).unwrap());
}

#[path = "pool_attribution/controls.rs"]
mod pool_attribution_controls;
#[test]
fn compiler_pool_genuine_nullable_source_and_complete_recovery_dtos_match_none() {
    let (plain, observed) =
        pool_attribution_controls::compile_pair(input("IterationNullable", NULLABLE));
    pool_attribution_controls::whole_outcomes(&plain, &observed, &["", "x", "x x"]);
}

#[path = "completed_components/controls.rs"]
mod completed_component_controls;
#[test]
fn composed_ports_genuine_nullable_recovery_matches_legacy() {
    completed_component_controls::compare(input("IterationNullable", NULLABLE), &["", "x", "x x"]);
}

#[path = "shared_analysis_calls/controls.rs"]
mod shared_analysis_call_controls;
#[test]
fn shared_call_ports_genuine_nullable_recovery_matches_legacy() {
    shared_analysis_call_controls::compare(input("IterationNullable", NULLABLE), &["", "x", "x x"]);
}

#[path = "first_queue_topology/controls.rs"]
mod first_queue_topology_controls;
#[test]
fn topology_observer_genuine_nullable_whole_results_and_ledger_match_disabled() {
    first_queue_topology_controls::compare(input("IterationNullable", NULLABLE), &["", "x", "x x"]);
}

#[path = "single_borrowed_ports/controls.rs"]
mod single_borrow_port_controls;
#[test]
fn single_borrow_ports_genuine_nullable_recovery_matches_legacy() {
    single_borrow_port_controls::compare(input("IterationNullable", NULLABLE), &["", "x", "x x"]);
}
