//! Exact common-source controls against genuine Xtext2.38 generated parsers.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
    evaluate_with_limits, ordinary,
};
use serde_json::{Value, json};

const ECORE: &str = include_str!("../../../fixtures/xtext-full-predicates-v68/imported.ecore");
const NS: &str = "urn:intersect:oracle:FullNested";

fn compile(name: &str, text: &str) -> Package {
    compile_profile(name, text, Profile::NativeV68)
}
fn compile_profile(name: &str, text: &str, profile: Profile) -> Package {
    let model = DefinitionSource::new(
        "memory:full-predicate/imported.ecore",
        SourceKind::Ecore,
        ECORE,
    );
    let mut terminals = DefinitionSource::new(
        "memory:full-predicate/Terminals.xtext",
        SourceKind::Xtext,
        include_str!("../../../fixtures/xtext-full-predicates-v68/Terminals.xtext"),
    );
    terminals.dependencies.push(model.id.clone());
    let mut grammar = DefinitionSource::new(
        format!("memory:full-predicate/{name}.xtext"),
        SourceKind::Xtext,
        text,
    );
    grammar
        .dependencies
        .extend([model.id.clone(), terminals.id.clone()]);
    let mut behavior = DefinitionSource::new(
        "memory:full-predicate/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package emptyRules version \"1\";",
    );
    behavior.dependencies.push(model.id.clone());
    let inputs = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: NS.into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: behavior.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), terminals, grammar.clone(), behavior],
            namespaces: vec![],
        },
    };
    let targets = [(
        model.id.clone(),
        "memory:full-predicate/imported.rcore".into(),
    )]
    .into();
    let migration = intersect_model::migration::migrate_ecore_bundle_with_limits(
        inputs.bundle.clone(),
        &targets,
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migration.reports.len(), 1);
    assert_eq!(migration.reports[0].original, model);
    let converted = migration.reports[0].converted.clone().unwrap();
    let inputs = intersect_text::migration::remap_compilation(&migration, &inputs).unwrap();
    let package = ordinary::compile(inputs, profile).unwrap();
    assert_eq!(package.schema.uri, NS);
    let retained = package.compilation.as_ref().unwrap();
    assert!(
        retained
            .bundle
            .sources
            .iter()
            .any(|source| source == &converted)
    );
    assert!(
        retained
            .bundle
            .sources
            .iter()
            .any(|source| source.id == grammar.id && source.text == text)
    );
    assert_eq!(package.grammar.as_ref().unwrap().entry, "Entry");
    let field = package.schema.effective_feature("Entry", "branch").unwrap();
    assert!(!field.feature.reference && !field.feature.derived && field.feature.changeable);
    assert_eq!(field.feature.upper, 1);
    assert_eq!(
        package
            .schema
            .runtime_attribute_lexical_target("Entry", "branch")
            .unwrap(),
        Some("EString")
    );
    package
}

fn compare_observations(rows: &Value, cases: &[(&str, &str)], expected_count: usize) {
    compare_observations_profile(rows, cases, expected_count, Profile::NativeV68)
}
fn compare_observations_profile(
    rows: &Value,
    cases: &[(&str, &str)],
    expected_count: usize,
    profile: Profile,
) {
    let mut compared = 0;
    for &(name, grammar) in cases {
        let package = if profile == Profile::NativeV68 {
            compile(name, grammar)
        } else {
            compile_profile(name, grammar, profile)
        };
        let language =
            DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        for expected in rows
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["grammar"] == format!("intersect.oracle.{name}"))
        {
            let text = expected["text"].as_str().unwrap();
            let source = Source {
                uri: "memory:full-prefix.model".into(),
                text: text.into(),
            };
            let actual = evaluate_with_limits(
                &package,
                std::slice::from_ref(&source),
                EvaluationLimits::workbench(),
            );
            let accepted = expected["accepted"].as_bool().unwrap();
            assert_eq!(
                actual.diagnostics.is_empty(),
                accepted,
                "{expected}: {:?}",
                actual.diagnostics
            );
            if accepted {
                assert_eq!(actual.model.elements.len(), 1);
                let element = &actual.model.elements[0];
                assert_eq!(element.class, "Entry");
                assert_eq!(element.resource, source.uri);
                assert_eq!(element.id, "memory:full-prefix.model#0");
                assert_eq!(element.type_identity, package.schema.identity("Entry"));
                assert_eq!(
                    json!({"class":format!("{}#//{}",package.schema.uri,element.class),"values":element.values}),
                    expected["model"]
                );
                assert_eq!(element.start, expected["root_start"]);
                assert_eq!(element.end, expected["root_end"]);
                assert_eq!(actual.references.len(), 0);
                assert_eq!(actual.derived.len(), 0);
                assert_eq!(
                    serde_json::to_value(&actual.diagnostics).unwrap(),
                    json!([])
                );
                let analysis = language
                    .analyze(&source, None, EvaluationLimits::workbench())
                    .unwrap();
                assert!(analysis.valid_model.is_some());
                assert!(analysis.snapshot.syntax_issues().is_empty());
                let feature = analysis
                    .snapshot
                    .feature_nodes()
                    .iter()
                    .find(|node| node.feature == "branch")
                    .unwrap();
                assert_eq!(
                    feature.identity,
                    package
                        .schema
                        .effective_feature("Entry", "branch")
                        .unwrap()
                        .identity
                );
                let node = &analysis.snapshot.document().nodes()[feature.node.0];
                assert_eq!(node.range.start, expected["feature_spans"][0]["start"]);
                assert_eq!(node.range.end, expected["feature_spans"][0]["end"]);
                if text == "a b d" {
                    assert_eq!(element.values["branch"], "d");
                    assert_eq!(element.start, 0);
                    assert_eq!(element.end, 5);
                }
            } else {
                // Native whole-resource publication rolls back partial objects;
                // independent Xtext parse recovery may retain a partial EObject.
                assert!(actual.model.elements.is_empty());
                assert!(!actual.diagnostics.is_empty());
            }
            compared += 1;
        }
    }
    assert_eq!(compared, expected_count);
}

#[test]
fn exact_imported_full_prefix_and_adjacent_guards_preserve_independent_branch_models_and_spans() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/observations.json"
    ))
    .unwrap();
    compare_observations(
        &rows,
        &[
            (
                "ImportedFullPrefix",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"
                ),
            ),
            (
                "ImportedFullBoth",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullBoth.xtext"),
            ),
        ],
        14,
    );
}

#[test]
fn direct_full_predicate_cardinality_preserves_independent_models_and_spans() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/cardinality-observations.json"
    ))
    .unwrap();
    compare_observations(
        &rows,
        &[
            (
                "ImportedFullOptional",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullOptional.xtext"
                ),
            ),
            (
                "ImportedFullStar",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullStar.xtext"),
            ),
            (
                "ImportedFullPlus",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPlus.xtext"),
            ),
            (
                "ImportedFullGroupedStar",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullGroupedStar.xtext"
                ),
            ),
        ],
        32,
    );
}

#[test]
fn v74_explicit_completion_policy_preserves_all_original_full_oracle_models_and_spans() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/observations.json"
    ))
    .unwrap();
    compare_observations_profile(
        &rows,
        &[
            (
                "ImportedFullPrefix",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"
                ),
            ),
            (
                "ImportedFullBoth",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullBoth.xtext"),
            ),
        ],
        14,
        Profile::NativeV74,
    );
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/cardinality-observations.json"
    ))
    .unwrap();
    compare_observations_profile(
        &rows,
        &[
            (
                "ImportedFullOptional",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullOptional.xtext"
                ),
            ),
            (
                "ImportedFullStar",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullStar.xtext"),
            ),
            (
                "ImportedFullPlus",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPlus.xtext"),
            ),
            (
                "ImportedFullGroupedStar",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullGroupedStar.xtext"
                ),
            ),
        ],
        32,
        Profile::NativeV74,
    );
}

#[test]
fn v75_call_free_reuse_preserves_all_original_full_oracle_models_and_spans() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/observations.json"
    ))
    .unwrap();
    compare_observations_profile(
        &rows,
        &[
            (
                "ImportedFullPrefix",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"
                ),
            ),
            (
                "ImportedFullBoth",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullBoth.xtext"),
            ),
        ],
        14,
        Profile::NativeV75,
    );
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/cardinality-observations.json"
    ))
    .unwrap();
    compare_observations_profile(
        &rows,
        &[
            (
                "ImportedFullOptional",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullOptional.xtext"
                ),
            ),
            (
                "ImportedFullStar",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullStar.xtext"),
            ),
            (
                "ImportedFullPlus",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPlus.xtext"),
            ),
            (
                "ImportedFullGroupedStar",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullGroupedStar.xtext"
                ),
            ),
        ],
        32,
        Profile::NativeV75,
    );
}

#[test]
fn v76_adjacent_queue_preserves_all_original_full_oracle_models_and_spans() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/observations.json"
    ))
    .unwrap();
    compare_observations_profile(
        &rows,
        &[
            (
                "ImportedFullPrefix",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"
                ),
            ),
            (
                "ImportedFullBoth",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullBoth.xtext"),
            ),
        ],
        14,
        Profile::NativeV76,
    );
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/cardinality-observations.json"
    ))
    .unwrap();
    compare_observations_profile(
        &rows,
        &[
            (
                "ImportedFullOptional",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullOptional.xtext"
                ),
            ),
            (
                "ImportedFullStar",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullStar.xtext"),
            ),
            (
                "ImportedFullPlus",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPlus.xtext"),
            ),
            (
                "ImportedFullGroupedStar",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullGroupedStar.xtext"
                ),
            ),
        ],
        32,
        Profile::NativeV76,
    );
}

#[test]
fn v77_nullable_slice_preserves_all_original_full_oracle_models_and_spans() {
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/observations.json"
    ))
    .unwrap();
    compare_observations_profile(
        &rows,
        &[
            (
                "ImportedFullPrefix",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"
                ),
            ),
            (
                "ImportedFullBoth",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullBoth.xtext"),
            ),
        ],
        14,
        Profile::NativeV77,
    );
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-full-predicates-v68/cardinality-observations.json"
    ))
    .unwrap();
    compare_observations_profile(
        &rows,
        &[
            (
                "ImportedFullOptional",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullOptional.xtext"
                ),
            ),
            (
                "ImportedFullStar",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullStar.xtext"),
            ),
            (
                "ImportedFullPlus",
                include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPlus.xtext"),
            ),
            (
                "ImportedFullGroupedStar",
                include_str!(
                    "../../../fixtures/xtext-full-predicates-v68/ImportedFullGroupedStar.xtext"
                ),
            ),
        ],
        32,
        Profile::NativeV77,
    );
}

#[path = "frontier_order/consumers.rs"]
mod frontier_order_consumers;

#[path = "partial_token/full_controls.rs"]
mod partial_token_probe_full_controls;

#[path = "transparent_chain/full_controls.rs"]
mod transparent_chain_controls;

#[path = "transparent_ports/full_controls.rs"]
mod transparent_port_controls;

#[path = "pool_attribution/controls.rs"]
mod pool_attribution_controls;
#[test]
fn compiler_pool_genuine_full_source_and_complete_fallback_dtos_match_none() {
    let prepared = compile_profile(
        "ImportedFullPrefix",
        include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"),
        Profile::NativeV80,
    );
    let (plain, observed) = pool_attribution_controls::compile_pair(prepared.compilation.unwrap());
    pool_attribution_controls::whole_outcomes(&plain, &observed, &["a b d", "a b c", "a"]);
}

#[path = "completed_components/controls.rs"]
mod completed_component_controls;
#[test]
fn composed_ports_genuine_full_imported_fallback_matches_legacy() {
    let prepared = compile_profile(
        "ImportedFullPrefix",
        include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"),
        Profile::NativeV80,
    );
    completed_component_controls::compare(prepared.compilation.unwrap(), &["a b d", "a b c", "a"]);
}

#[path = "shared_analysis_calls/controls.rs"]
mod shared_analysis_call_controls;
#[test]
fn shared_call_ports_genuine_full_imported_fallback_matches_legacy() {
    let prepared = compile_profile(
        "ImportedFullPrefix",
        include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"),
        Profile::NativeV80,
    );
    shared_analysis_call_controls::compare(prepared.compilation.unwrap(), &["a b d", "a b c", "a"]);
}

#[path = "first_queue_topology/controls.rs"]
mod first_queue_topology_controls;
#[test]
fn topology_observer_genuine_full_whole_results_and_ledger_match_disabled() {
    let prepared = compile_profile(
        "ImportedFullPrefix",
        include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"),
        Profile::NativeV80,
    );
    first_queue_topology_controls::compare(prepared.compilation.unwrap(), &["a b d", "a b c", "a"]);
}

#[path = "single_borrowed_ports/controls.rs"]
mod single_borrow_port_controls;
#[test]
fn single_borrow_ports_genuine_full_imported_fallback_matches_legacy() {
    let prepared = compile_profile(
        "ImportedFullPrefix",
        include_str!("../../../fixtures/xtext-full-predicates-v68/ImportedFullPrefix.xtext"),
        Profile::NativeV80,
    );
    single_borrow_port_controls::compare(prepared.compilation.unwrap(), &["a b d", "a b c", "a"]);
}
