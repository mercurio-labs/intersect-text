//! Focused new-profile consumers over original shared predictor/document inputs.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
    evaluate_with_limits, ordinary,
};
use serde_json::Value;

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

#[allow(dead_code)]
fn migrated(ecore: &str, grammar: &str, behavior: &str, profile: Profile) -> Package {
    ordinary::compile(migrated_inputs(ecore, grammar, behavior), profile).unwrap()
}

fn source(text: &str) -> [Source; 1] {
    [Source {
        uri: "shared.model".into(),
        text: text.into(),
    }]
}

fn full_inputs(grammar: &str) -> BundleCompilation {
    let mut inputs = migrated_inputs(
        include_str!("../../../fixtures/xtext/recursive-predicates/model.ecore"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext"),
        include_str!("../../../fixtures/xtext/recursive-predicates/model-behavior.rcore"),
    );
    // Only the source envelope is reused; the old executable IR is discarded.
    // Retain this test's exact authored full-predicate bytes and new identity.
    let old = inputs.xtext.clone().unwrap();
    let slot = inputs
        .bundle
        .sources
        .iter_mut()
        .find(|source| source.id == old)
        .unwrap();
    let mut authored = DefinitionSource::new(old.uri.clone(), SourceKind::Xtext, grammar);
    authored.dependencies = slot.dependencies.clone();
    *slot = authored.clone();
    inputs.xtext = Some(authored.id.clone());
    for source in &mut inputs.bundle.sources {
        for dependency in &mut source.dependencies {
            if *dependency == old {
                *dependency = authored.id.clone();
            }
        }
    }
    inputs
}

#[test]
fn v68_full_authored_predicate_uses_real_parser_model_and_retained_original_source() {
    let original = include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext");
    let authored = original.replace("->Left", "=>Left");
    let inputs = full_inputs(&authored);
    let old = ordinary::compile(inputs.clone(), Profile::NativeV67).unwrap_err();
    assert!(
        old.iter()
            .any(|error| error.message.contains("Unsupported Xtext construct '='"))
    );
    let package = ordinary::compile(inputs, Profile::NativeV68).unwrap();
    assert_eq!(
        package
            .compilation
            .as_ref()
            .unwrap()
            .bundle
            .sources
            .iter()
            .find(|source| source.kind == SourceKind::Xtext)
            .unwrap()
            .text,
        authored
    );
    let language =
        DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
            .unwrap();
    let rows: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/recursive-predicates/observations.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["parser"] == "RecursiveEntryFull")
    {
        let input = source(row["text"].as_str().unwrap());
        let evaluation = evaluate_with_limits(&package, &input, EvaluationLimits::workbench());
        let accepted = row["accepted"].as_bool().unwrap();
        assert_eq!(
            evaluation.diagnostics.is_empty(),
            accepted,
            "{row}: {:?}",
            evaluation.diagnostics
        );
        let analysis = language
            .analyze(&input[0], None, EvaluationLimits::workbench())
            .unwrap();
        assert_eq!(analysis.valid_model.is_some(), accepted, "{row}");
        if accepted {
            assert_eq!(
                analysis.snapshot.document().nodes()[0].total_range.end,
                input[0].text.encode_utf16().count()
            );
        }
        count += 1;
    }
    assert_eq!(count, 11);
    let input = source(include_str!(
        "../../../fixtures/xtext/recursive-predicates/example.model"
    ));
    let actual = evaluate_with_limits(&package, &input, EvaluationLimits::workbench());
    assert!(actual.diagnostics.is_empty(), "{:?}", actual.diagnostics);
    assert_eq!(actual.model.elements.len(), 2);
    assert_eq!(actual.model.elements[0].values["number"], 12);
    actual.model.validate(&package.schema).unwrap();
}

#[test]
fn v68_quoted_arrow_is_an_ordinary_keyword_and_outer_cardinality_retains_operand() {
    let original = include_str!("../../../fixtures/xtext/recursive-predicates/model.xtext");
    let authored = original.replace(
        "Entry returns r::Entry: ->Left | Right;",
        "Entry returns r::Left: (=> ('a' 'b'))* '=>';",
    );
    let package = ordinary::compile(full_inputs(&authored), Profile::NativeV68).unwrap();
    let body = &package.grammar.as_ref().unwrap().rules["Entry"].body;
    let json = serde_json::to_value(body).unwrap();
    assert!(json.to_string().contains("SyntacticPredicate"));
    let evaluation =
        evaluate_with_limits(&package, &source("a b =>"), EvaluationLimits::workbench());
    assert!(
        evaluation.diagnostics.is_empty(),
        "{:?}",
        evaluation.diagnostics
    );
}
