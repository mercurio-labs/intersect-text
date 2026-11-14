//! Direct native program bridge controls: executable callbacks are not certificates.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::schema::Diagnostic;
use intersect_text::grammar;
use intersect_text::grammar::AuthoredSourceProgramAdapter;
use intersect_text::grammar::GeneratedRuleTypes;
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
use intersect_text::grammar::SourceProgram;
use intersect_text::package::BundleCompilation;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
};
fn input(negative: bool) -> BundleCompilation {
    let f: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/contextual-first-rule/native/parameter-cases.json"
    ))
    .unwrap();
    let id = if negative {
        "terminal_first_parameter"
    } else {
        "canonical_parameter"
    };
    serde_json::from_value(
        f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap()["compilation"]
            .clone(),
    )
    .unwrap()
}
fn derived(
    mut input: BundleCompilation,
) -> (
    BundleCompilation,
    intersect_model::definitions::bundle::SourceId,
) {
    let original = input.xtext.clone().unwrap();
    let source = input.bundle.source(&original).unwrap();
    let end = [
        source.text.find("terminal VALUE").unwrap(),
        source.text.find("Model returns").unwrap(),
    ]
    .into_iter()
    .min()
    .unwrap();
    let mut generated = DefinitionSource::new(
        "memory:contextual-entry/synthetic61.xtext",
        SourceKind::Xtext,
        format!(
            "{}Model returns k::Model: name=VALUE;\nterminal VALUE:'a';\n",
            &source.text[..end]
        ),
    );
    generated.dependencies = source.dependencies.clone();
    generated.dependencies.push(original.clone());
    input.xtext = Some(generated.id.clone());
    input.bundle.sources.push(generated);
    (input, original)
}
fn schema() -> intersect_model::definitions::schema::Schema {
    intersect_text::ordinary::compile(input(false), Profile::NativeV60)
        .unwrap()
        .schema
}
#[test]
fn legacy_public_program_contract_rejects_new_profile_without_fresh_adapter() {
    let (comp, original) = derived(input(false));
    let entry = comp.xtext.as_ref().unwrap();
    let bindings = [GeneratedRuleTypes {
        source: entry.clone(),
        types: BTreeMap::new(),
    }];
    let producers = BTreeMap::from([(entry.clone(), original.clone())]);
    let program = SourceProgram {
        bindings: &bindings,
        producers: &producers,
    };
    let error = grammar::compile_source_program(
        &comp.bundle,
        entry,
        &mut schema(),
        Profile::NativeV61,
        SourceLimits::default(),
        &[],
        &program,
    )
    .unwrap_err();
    assert!(error.message.contains("fresh authored-source validator"));
    assert_eq!(error.source, original.uri);
    assert!(
        grammar::compile_source_program(
            &comp.bundle,
            entry,
            &mut schema(),
            Profile::NativeV60,
            SourceLimits::default(),
            &[],
            &program
        )
        .is_ok()
    );
}
#[test]
fn original_invalid_first_rule_cannot_hide_behind_valid_synthetic_first_parser() {
    let (comp, original) = derived(input(true));
    let entry = comp.xtext.as_ref().unwrap();
    let bindings = [GeneratedRuleTypes {
        source: entry.clone(),
        types: BTreeMap::new(),
    }];
    let producers = BTreeMap::from([(entry.clone(), original.clone())]);
    let program = SourceProgram {
        bindings: &bindings,
        producers: &producers,
    };
    let error = grammar::compile_source_program_with_authored_validation(
        &comp.bundle,
        entry,
        &mut schema(),
        Profile::NativeV61,
        SourceLimits::default(),
        &[],
        &AuthoredSourceProgramAdapter {
            program: &program,
            validate_authored: intersect_text::ordinary::validate_authored_program,
        },
    )
    .unwrap_err();
    assert_eq!(error.source, original.uri);
    assert_eq!(error.message, "The first rule must be a parser rule.");
    assert!(format!("{:?}", error.origin).contains(&original.sha256));
}
static CALLS: AtomicUsize = AtomicUsize::new(0);
fn failing_adapter(
    _: &intersect_model::definitions::bundle::DefinitionBundle,
    entry: &intersect_model::definitions::bundle::SourceId,
    _: SourceLimits,
) -> Result<(), Diagnostic> {
    CALLS.fetch_add(1, Ordering::SeqCst);
    Err(Diagnostic::from_span(
        intersect_model::definitions::bundle::SourceSpan {
            source: entry.clone(),
            start: 12,
            end: 17,
        },
        "Fresh native adapter rejected original source",
    ))
}
#[test]
fn fresh_callback_invocation_and_original_failure_identity_survive_each_call() {
    let (comp, original) = derived(input(false));
    let entry = comp.xtext.as_ref().unwrap();
    let bindings = [GeneratedRuleTypes {
        source: entry.clone(),
        types: BTreeMap::new(),
    }];
    let producers = BTreeMap::from([(entry.clone(), original.clone())]);
    let program = SourceProgram {
        bindings: &bindings,
        producers: &producers,
    };
    let adapter = AuthoredSourceProgramAdapter {
        program: &program,
        validate_authored: failing_adapter,
    };
    let before = CALLS.load(Ordering::SeqCst);
    for _ in 0..2 {
        let error = grammar::compile_source_program_with_authored_validation(
            &comp.bundle,
            entry,
            &mut schema(),
            Profile::NativeV61,
            SourceLimits::default(),
            &[],
            &adapter,
        )
        .unwrap_err();
        assert_eq!((error.start, error.end), (12, 17));
        assert_eq!(error.source, original.uri);
        assert!(format!("{:?}", error.origin).contains(&original.sha256));
    }
    assert_eq!(CALLS.load(Ordering::SeqCst), before + 2);
}
