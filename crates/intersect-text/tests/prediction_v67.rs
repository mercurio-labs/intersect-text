//! Focused new-profile consumers over original shared predictor/document inputs.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
    evaluate, evaluate_with_limits, ordinary,
};
use serde_json::{Value, json};

fn migrated_inputs(ecore: &str, grammar: &str, behavior: &str) -> BundleCompilation {
    // Same explicit fixture conversion/URIs as the maintained acceptance helper.
    let report = intersect_model::migration::migrate_ecore(
        DefinitionSource::new("memory:oracle.ecore", SourceKind::Ecore, ecore),
        "memory:fixture.rcore",
    )
    .unwrap();
    let converted = report.converted.unwrap();
    // The historical facade creates the canonical source envelope. Its old IR
    // is discarded; V63+ admission must replay those sources in the ordinary
    // language frontend with fresh authored-source validation.
    compiler::compile_rcore_with_profile(
        &converted.text,
        Some(grammar),
        Some(behavior),
        Profile::NativeV39,
    )
    .unwrap()
    .compilation
    .unwrap()
}

fn migrated(ecore: &str, grammar: &str, behavior: &str, profile: Profile) -> Package {
    ordinary::compile(migrated_inputs(ecore, grammar, behavior), profile).unwrap()
}

fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "shared.model".into(),
        text: text.into(),
    }]
}

#[test]
fn positive_v67_recursive_commitment_matches_all_original_independent_parser_rows() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut outcomes = Vec::new();
    for profile in [Profile::NativeV66, Profile::NativeV67] {
        let package = migrated(
            include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
            profile,
        );
        assert_eq!(package.grammar.as_ref().unwrap().entry, "Entry");
        let language =
            DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        let mut current = Vec::new();
        for row in rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["parser"] == "RecursiveEntryFirst")
        {
            let actual = evaluate(&package, &source(row["text"].as_str().unwrap()));
            assert_eq!(
                actual.diagnostics.is_empty(),
                row["accepted"].as_bool().unwrap(),
                "{profile:?} {row}"
            );
            let input = source(row["text"].as_str().unwrap());
            let analysis = language
                .analyze(&input[0], None, EvaluationLimits::workbench())
                .unwrap();
            if row["accepted"] == true {
                // Literal-only branches need not construct an EObject. Prove
                // real branch selection and complete consumption in the node
                // tree, then compare the entire old-profile observation below.
                assert!(analysis.valid_model.is_some());
                assert!(analysis.snapshot.syntax_issues().is_empty());
                let nodes = analysis.snapshot.document().nodes();
                assert_eq!(nodes[0].total_range.start, 0);
                assert_eq!(
                    nodes[0].total_range.end,
                    input[0].text.encode_utf16().count()
                );
                let left = &package.grammar.as_ref().unwrap().declaration_sources["Left"];
                assert!(nodes.iter().any(|node| node.kind
                    == intersect_model::core::text_document::NodeKind::Composite
                    && node.range.start == 0
                    && node.range.end == input[0].text.encode_utf16().count()
                    && node.data.origin.is_some_and(|origin| {
                        analysis.snapshot.origins()[origin].target.as_ref() == Some(left)
                    })));
                assert!(
                    analysis
                        .snapshot
                        .prediction_dependencies()
                        .iter()
                        .any(|dependency| dependency.occurrence == "rule/Entry/choice"
                            && dependency.range.start == 0
                            && dependency.range.end > 0)
                );
                assert!(analysis.snapshot.tree_usage().operations > 0);
                actual.model.validate(&package.schema).unwrap();
            } else {
                assert!(actual.model.elements.is_empty());
                assert!(analysis.valid_model.is_none());
            }
            current.push(json!({"evaluation":actual,"document":analysis}));
        }
        assert_eq!(current.len(), 11);
        outcomes.push(current);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[test]
fn original_lowercase_shared_recursion_name_remains_rejected_without_rewriting() {
    let inputs = migrated_inputs(
        include_str!("../../../fixtures/xtext/current-actions/model.ecore"),
        include_str!("../../../fixtures/xtext/shared-recursion/model.xtext"),
        include_str!("../../../fixtures/xtext/current-actions/model-behavior.rcore"),
    );
    let before = ordinary::compile(inputs.clone(), Profile::NativeV66).unwrap_err();
    let after = ordinary::compile(inputs, Profile::NativeV67).unwrap_err();
    assert_eq!(before, after);
    assert!(after.iter().any(|diagnostic| diagnostic.message
        == "The last element should start with an upper case letter."));
}

#[test]
fn positive_v67_original_recursive_value_example_preserves_models_spans_and_rollback() {
    let text = include_str!("../../../fixtures/xtext/recursive-predicates/example.model");
    let mut outcomes = Vec::new();
    for profile in [Profile::NativeV66, Profile::NativeV67] {
        let package = migrated(
            include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
            profile,
        );
        assert_eq!(package.grammar.as_ref().unwrap().entry, "Entry");
        let input = source(text);
        let actual = evaluate(&package, &input);
        assert!(actual.diagnostics.is_empty());
        assert_eq!(actual.model.elements.len(), 2);
        let left = &actual.model.elements[0];
        assert_eq!(left.class, "Left");
        assert_eq!(left.id, "shared.model#0");
        assert_eq!(left.values["number"], 12);
        assert_eq!(left.values["child"], "shared.model#1");
        assert_eq!((left.start, left.end), (0, text.trim_end().len()));
        let inner = &actual.model.elements[1];
        assert_eq!(inner.class, "Inner");
        assert_eq!(inner.values["child"], Value::Null);
        assert_eq!(&text[inner.start..inner.end], "a a b b");
        actual.model.validate(&package.schema).unwrap();
        assert_eq!(
            evaluate(&intersect_text::recompile(&package).unwrap(), &input).model,
            actual.model
        );
        let exact = evaluate_with_limits(
            &package,
            &input,
            EvaluationLimits {
                elements_per_resource: Some(2),
                ..EvaluationLimits::native()
            },
        );
        assert_eq!(exact.model, actual.model);
        for limits in [
            EvaluationLimits {
                elements_per_resource: Some(1),
                ..EvaluationLimits::native()
            },
            EvaluationLimits {
                steps_per_resource: Some(1),
                ..EvaluationLimits::native()
            },
        ] {
            let rejected = evaluate_with_limits(&package, &input, limits);
            assert!(!rejected.diagnostics.is_empty());
            assert!(rejected.model.elements.is_empty());
        }
        let language =
            DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        let analysis = language
            .analyze(&input[0], None, EvaluationLimits::workbench())
            .unwrap();
        assert!(analysis.valid_model.is_some());
        assert!(analysis.snapshot.syntax_issues().is_empty());
        let nodes = analysis.snapshot.document().nodes();
        assert_eq!(nodes[0].total_range.start, 0);
        assert_eq!(nodes[0].total_range.end, text.encode_utf16().count());
        assert_eq!(nodes[0].range.end, text.trim_end().encode_utf16().count());
        assert!(
            analysis
                .snapshot
                .prediction_dependencies()
                .iter()
                .any(|dependency| dependency.occurrence.starts_with("rule/Inner/"))
        );
        assert!(analysis.snapshot.tree_usage().operations > 41);
        outcomes.push(json!({"evaluation":actual,"document":analysis}));
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

fn document_inputs() -> BundleCompilation {
    let cases: Value =
        serde_json::from_str(include_str!("../src/document_tests/cases.json")).unwrap();
    let selected = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "namespace_nested_restore")
        .unwrap();
    serde_json::from_value(selected["compilation"].clone()).unwrap()
}

#[test]
fn v67_full_document_recovery_preserves_all_original_nested_error_models_and_issues() {
    let rows: Value =
        serde_json::from_str(include_str!("../src/document_tests/nested_recovery.json")).unwrap();
    let mut outcomes = Vec::new();
    for profile in [Profile::NativeV66, Profile::NativeV67] {
        let package = ordinary::compile(document_inputs(), profile).unwrap();
        let language =
            DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        let mut current = Vec::new();
        for row in rows["rows"].as_array().unwrap() {
            let source = Source {
                uri: "memory:node-analysis/recovery".into(),
                text: row["text"].as_str().unwrap().into(),
            };
            let analysis = language
                .analyze(&source, None, EvaluationLimits::workbench())
                .unwrap();
            assert!(analysis.valid_model.is_none());
            let recovered = analysis.recovered.as_ref().unwrap();
            let expected = row["resource"]["model"].as_array().unwrap();
            assert_eq!(recovered.objects.len(), expected.len());
            for (actual, expected) in recovered.objects.iter().zip(expected) {
                assert_eq!(json!(actual.values()), expected["values"]);
            }
            let errors: Vec<_> = analysis
                .snapshot
                .syntax_issues()
                .iter()
                .map(|issue| issue.message.as_str())
                .collect();
            let expected: Vec<_> = row["resource"]["errors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|issue| issue["message"].as_str().unwrap())
                .collect();
            assert_eq!(errors, expected, "{profile:?} {}", row["text"]);
            current.push(json!(analysis));
        }
        assert_eq!(current.len(), 8);
        outcomes.push(current);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

fn live_return_inputs() -> BundleCompilation {
    let cases: Value =
        serde_json::from_str(include_str!("../src/document_tests/cases.json")).unwrap();
    let mut input: BundleCompilation =
        serde_json::from_value(cases["cases"][0]["compilation"].clone()).unwrap();
    let old = input.xtext.take().unwrap();
    let index = input
        .bundle
        .sources
        .iter()
        .position(|source| source.id == old)
        .unwrap();
    // Exact original maintained actual-live-return-site control grammar.
    let mut authored = DefinitionSource::new(
        "memory:document-mechanics/ActualReturn.xtext",
        SourceKind::Xtext,
        "grammar document.mechanics.ActualReturn\nimport \"urn:intersect:xtext:mini-kerml:terminal-spellings:1\" as k\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nModel returns k::Model: name=Inner 'm';\nOther returns k::Model: name=Inner 'o';\nInner returns ecore::EString: 'a';\n",
    );
    authored.dependencies = input.bundle.sources[index].dependencies.clone();
    input.xtext = Some(authored.id.clone());
    input.bundle.sources[index] = authored;
    input
}

#[test]
fn v67_caller_specific_document_recovery_keeps_exact_and_one_short_tree_boundaries() {
    let before = ordinary::compile(live_return_inputs(), Profile::NativeV66).unwrap();
    let after = ordinary::compile(live_return_inputs(), Profile::NativeV67).unwrap();
    let old = DocumentLanguage::prepare(&before, Default::default(), AnalysisLimits::workbench())
        .unwrap();
    let new =
        DocumentLanguage::prepare(&after, Default::default(), AnalysisLimits::workbench()).unwrap();
    for text in ["o m", "a o"] {
        let source = Source {
            uri: "memory:v67-live-return/input".into(),
            text: text.into(),
        };
        let expected = old
            .analyze(&source, None, EvaluationLimits::workbench())
            .unwrap();
        let actual = new
            .analyze(&source, None, EvaluationLimits::workbench())
            .unwrap();
        assert!(actual.valid_model.is_none());
        assert!(actual.recovered.is_some());
        assert!(!actual.snapshot.syntax_issues().is_empty());
        assert_eq!(json!(actual), json!(expected), "{text}");
        let usage = actual.snapshot.tree_usage();
        for mode in 0..4 {
            let mut limits = AnalysisLimits::workbench();
            match mode {
                0 => limits.tree.nodes = usage.nodes,
                1 => limits.tree.operations = usage.operations,
                2 => limits.tree.logical_storage_bytes = usage.peak_logical_storage_bytes,
                _ => limits.tree.source_bytes = source.text.len(),
            }
            let exact = DocumentLanguage::prepare(&after, Default::default(), limits)
                .unwrap()
                .analyze(&source, None, EvaluationLimits::workbench())
                .unwrap();
            assert_eq!(json!(exact), json!(actual));
            match mode {
                0 => limits.tree.nodes -= 1,
                1 => limits.tree.operations -= 1,
                2 => limits.tree.logical_storage_bytes -= 1,
                _ => limits.tree.source_bytes -= 1,
            }
            assert!(
                DocumentLanguage::prepare(&after, Default::default(), limits)
                    .unwrap()
                    .analyze(&source, None, EvaluationLimits::workbench())
                    .is_err(),
                "{text}: one-short tree mode {mode}"
            );
        }
    }
}

#[test]
fn positive_v76_recursive_commitment_matches_all_original_independent_parser_rows() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut outcomes = Vec::new();
    for profile in [Profile::NativeV75, Profile::NativeV76] {
        let package = migrated(
            include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
            profile,
        );
        assert_eq!(package.grammar.as_ref().unwrap().entry, "Entry");
        let language =
            DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        let mut current = Vec::new();
        for row in rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["parser"] == "RecursiveEntryFirst")
        {
            let actual = evaluate(&package, &source(row["text"].as_str().unwrap()));
            assert_eq!(
                actual.diagnostics.is_empty(),
                row["accepted"].as_bool().unwrap(),
                "{profile:?} {row}"
            );
            let input = source(row["text"].as_str().unwrap());
            let analysis = language
                .analyze(&input[0], None, EvaluationLimits::workbench())
                .unwrap();
            if row["accepted"] == true {
                // Literal-only branches need not construct an EObject. Prove
                // real branch selection and complete consumption in the node
                // tree, then compare the entire old-profile observation below.
                assert!(analysis.valid_model.is_some());
                assert!(analysis.snapshot.syntax_issues().is_empty());
                let nodes = analysis.snapshot.document().nodes();
                assert_eq!(nodes[0].total_range.start, 0);
                assert_eq!(
                    nodes[0].total_range.end,
                    input[0].text.encode_utf16().count()
                );
                let left = &package.grammar.as_ref().unwrap().declaration_sources["Left"];
                assert!(nodes.iter().any(|node| node.kind
                    == intersect_model::core::text_document::NodeKind::Composite
                    && node.range.start == 0
                    && node.range.end == input[0].text.encode_utf16().count()
                    && node.data.origin.is_some_and(|origin| {
                        analysis.snapshot.origins()[origin].target.as_ref() == Some(left)
                    })));
                assert!(
                    analysis
                        .snapshot
                        .prediction_dependencies()
                        .iter()
                        .any(|dependency| dependency.occurrence == "rule/Entry/choice"
                            && dependency.range.start == 0
                            && dependency.range.end > 0)
                );
                assert!(analysis.snapshot.tree_usage().operations > 0);
                actual.model.validate(&package.schema).unwrap();
            } else {
                assert!(actual.model.elements.is_empty());
                assert!(analysis.valid_model.is_none());
            }
            current.push(json!({"evaluation":actual,"document":analysis}));
        }
        assert_eq!(current.len(), 11);
        outcomes.push(current);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[test]
fn positive_v77_recursive_commitment_matches_all_original_independent_parser_rows() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut outcomes = Vec::new();
    for profile in [Profile::NativeV76, Profile::NativeV77] {
        let package = migrated(
            include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
            profile,
        );
        assert_eq!(package.grammar.as_ref().unwrap().entry, "Entry");
        let language =
            DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        let mut current = Vec::new();
        for row in rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["parser"] == "RecursiveEntryFirst")
        {
            let actual = evaluate(&package, &source(row["text"].as_str().unwrap()));
            println!(
                "V77_FIRST_PREASSERT {}",
                serde_json::to_string(
                    &json!({"profile":profile,"oracle":row,"evaluation":&actual})
                )
                .unwrap()
            );
            assert_eq!(
                actual.diagnostics.is_empty(),
                row["accepted"].as_bool().unwrap(),
                "{profile:?} {row}"
            );
            let input = source(row["text"].as_str().unwrap());
            let analysis = language
                .analyze(&input[0], None, EvaluationLimits::workbench())
                .unwrap();
            if row["accepted"] == true {
                // Literal-only branches need not construct an EObject. Prove
                // real branch selection and complete consumption in the node
                // tree, then compare the entire old-profile observation below.
                assert!(analysis.valid_model.is_some());
                assert!(analysis.snapshot.syntax_issues().is_empty());
                let nodes = analysis.snapshot.document().nodes();
                assert_eq!(nodes[0].total_range.start, 0);
                assert_eq!(
                    nodes[0].total_range.end,
                    input[0].text.encode_utf16().count()
                );
                let left = &package.grammar.as_ref().unwrap().declaration_sources["Left"];
                assert!(nodes.iter().any(|node| node.kind
                    == intersect_model::core::text_document::NodeKind::Composite
                    && node.range.start == 0
                    && node.range.end == input[0].text.encode_utf16().count()
                    && node.data.origin.is_some_and(|origin| {
                        analysis.snapshot.origins()[origin].target.as_ref() == Some(left)
                    })));
                assert!(
                    analysis
                        .snapshot
                        .prediction_dependencies()
                        .iter()
                        .any(|dependency| dependency.occurrence == "rule/Entry/choice"
                            && dependency.range.start == 0
                            && dependency.range.end > 0)
                );
                assert!(analysis.snapshot.tree_usage().operations > 0);
                actual.model.validate(&package.schema).unwrap();
            } else {
                assert!(actual.model.elements.is_empty());
                assert!(analysis.valid_model.is_none());
            }
            current.push(json!({"evaluation":actual,"document":analysis}));
        }
        assert_eq!(current.len(), 11);
        outcomes.push(current);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[test]
fn positive_v80_bounded_witness_reuse_preserves_genuine_recursive_first_whole_results() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut outcomes = Vec::new();
    for profile in [Profile::NativeV79, Profile::NativeV80] {
        let package = migrated(
            include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
            include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
            profile,
        );
        let language =
            DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        let mut current = Vec::new();
        for row in rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["parser"] == "RecursiveEntryFirst")
        {
            let input = source(row["text"].as_str().unwrap());
            let actual = evaluate(&package, &input);
            let document = language
                .analyze(&input[0], None, EvaluationLimits::workbench())
                .unwrap();
            assert_eq!(
                actual.diagnostics.is_empty(),
                row["accepted"].as_bool().unwrap()
            );
            assert_eq!(
                document.valid_model.is_some(),
                row["accepted"].as_bool().unwrap()
            );
            if row["text"] == "a c" {
                assert_eq!(row["branch"], "g");
                assert_eq!(row["syntax_errors"], 1);
                assert!(!actual.diagnostics.is_empty());
                assert!(document.valid_model.is_none());
            }
            current.push(json!({"evaluation":actual,"document":document}));
        }
        assert_eq!(current.len(), 11);
        println!(
            "V80_GENUINE_FIRST_RESULTS {}",
            json!({"profile":profile,"outcomes":current})
        );
        outcomes.push(current);
    }
    assert_eq!(outcomes[0], outcomes[1]);
}

#[path = "frontier_order/first_controls.rs"]
mod frontier_order_first_controls;

#[path = "partial_token/first_controls.rs"]
mod partial_token_probe_first_controls;

#[path = "transparent_chain/first_controls.rs"]
mod transparent_chain_controls;

#[path = "transparent_ports/first_controls.rs"]
mod transparent_port_controls;

#[path = "pool_attribution/controls.rs"]
mod pool_attribution_controls;
#[test]
fn compiler_pool_genuine_first_source_and_complete_error_dtos_match_none() {
    let inputs = migrated_inputs(
        include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
    );
    let (plain, observed) = pool_attribution_controls::compile_pair(inputs);
    pool_attribution_controls::whole_outcomes(&plain, &observed, &["a b", "a b d", "a c", ""]);
}

#[path = "completed_components/controls.rs"]
mod completed_component_controls;
#[test]
fn composed_ports_genuine_first_complete_outcomes_match_legacy() {
    let input = migrated_inputs(
        include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
    );
    completed_component_controls::compare(input, &["a b", "a b d", "a c", ""]);
}

#[path = "shared_analysis_calls/controls.rs"]
mod shared_analysis_call_controls;
#[test]
fn shared_call_ports_genuine_first_complete_outcomes_match_legacy() {
    let input = migrated_inputs(
        include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
    );
    shared_analysis_call_controls::compare(input, &["a b", "a b d", "a c", ""]);
}

#[path = "first_queue_topology/controls.rs"]
mod first_queue_topology_controls;
#[test]
fn topology_observer_genuine_first_whole_results_and_ledger_match_disabled() {
    let input = migrated_inputs(
        include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
    );
    first_queue_topology_controls::compare(input, &["a b", "a b d", "a c", ""]);
}

#[path = "single_borrowed_ports/controls.rs"]
mod single_borrow_port_controls;
#[test]
fn single_borrow_ports_genuine_first_complete_outcomes_match_legacy() {
    let input = migrated_inputs(
        include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
    );
    single_borrow_port_controls::compare(input, &["a b", "a b d", "a c", ""]);
}
