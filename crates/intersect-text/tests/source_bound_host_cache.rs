//! Handwritten host-cache control (language-pack step 4): a source-bound
//! native draft restored from its saved form, without recompiling or replaying
//! migration, parses real documents exactly as the freshly bound definition.
//! This is draft construction equivalence, not original language admission.
use intersect_text::{
    EvaluationLimits, Source,
    document::AnalysisLimits,
    draft::{DraftLanguage, GrammarPolicy, OpenDraftCapability},
    ordinary,
};
use serde_json::Value;
use std::sync::Arc;
use {
    intersect_model::core::frozen::FrozenStore, intersect_model::core::query::QueryBudget,
    intersect_model::definitions::bundle::DefinitionBundle,
    intersect_model::definitions::bundle::DefinitionSource,
    intersect_model::definitions::bundle::SourceKind, intersect_text::grammar::Profile,
    intersect_text::grammar::RuntimeAdmissionLoadMode,
    intersect_text::grammar::RuntimeGuardedPolicy, intersect_text::package::BundleCompilation,
    intersect_text::package::CompilationSourcePolicy,
    intersect_text::preparation::SavedSourceBoundDefinition,
    intersect_text::preparation::SharedDraftRuntimeLanguageDefinition,
    intersect_text::preparation::SourceBoundDraftRuntimeDefinition,
};

const DOCUMENTS: [&str; 5] = ["x", "x x", "x x x x", "x\nx\tx", "y x"];

fn bound() -> Result<SourceBoundDraftRuntimeDefinition, String> {
    let model = DefinitionSource::new(
        "memory:bound-cache/GateModel.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/xtext-whole-iteration-v76/GateModel.ecore"),
    );
    let mut syntax = DefinitionSource::new(
        "memory:bound-cache/IterationX.xtext",
        SourceKind::Xtext,
        include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationX.xtext"),
    );
    syntax.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:bound-cache/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package emptyRules version \"1\";",
    );
    rules.dependencies.push(model.id.clone());
    let original = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:intersect:oracle:WholeIterationGate".into(),
        xtext: Some(syntax.id.clone()),
        rcore_behavior: rules.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), syntax, rules],
            namespaces: vec![],
        },
    };
    let completed = intersect_model::migration::migrate_ecore_bundle_completed_with_limits(
        original.bundle.clone(),
        &[(model.id, "memory:bound-cache/GateModel.rcore".into())].into(),
        intersect_model::definitions::ecore::ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    let policy = CompilationSourcePolicy::default();
    let owner = ordinary::prepare_with_runtime_guarded_policy(
        intersect_text::migration::remap_compilation(completed.report(), &original)?,
        Profile::NativeV80,
        &[],
        None,
        policy,
        policy,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        None,
    )
    .map_err(|e| format!("{e:?}"))?;
    SharedDraftRuntimeLanguageDefinition::new(Arc::new(owner))
        .bind_completed_origin(completed, policy)
        .map_err(|e| format!("{e:?}"))
}

// Parse every document into its own store; record diagnostics, references,
// consumed runtime decisions and the constructed model.
fn parse_all(bound: &SourceBoundDraftRuntimeDefinition) -> Result<Vec<Value>, String> {
    let definition = bound
        .prepare(CompilationSourcePolicy::default())
        .map_err(|e| format!("{e:?}"))?;
    let inner = GrammarPolicy::new(&definition);
    let capability = OpenDraftCapability::new(&definition, &inner)?;
    let language = DraftLanguage::bind(&definition, &capability, AnalysisLimits::workbench())?;
    let mut rows = Vec::new();
    for (index, text) in DOCUMENTS.into_iter().enumerate() {
        let source = Source {
            uri: format!("memory:bound-cache/doc{index}.x"),
            text: text.into(),
        };
        let mut store = FrozenStore::for_schema(definition.schema()).map_err(|e| e.to_string())?;
        let row = match language.parse_runtime_into(
            &source,
            EvaluationLimits::native(),
            &mut store,
            &mut QueryBudget::new(usize::MAX),
        ) {
            Ok((evaluation, references, consumed)) => serde_json::json!({
                "evaluation": serde_json::to_value(&evaluation).map_err(|e| e.to_string())?,
                "references": references,
                "consumed": consumed,
                "model": serde_json::to_value(store.into_model().map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            }),
            Err(failure) => serde_json::json!({ "failure": failure }),
        };
        rows.push(row);
    }
    Ok(rows)
}

#[test]
fn loaded_source_bound_definition_parses_documents_like_the_fresh_one() -> Result<(), String> {
    let fresh = bound()?;
    let expected = parse_all(&fresh)?;
    // The fixture is a real control: valid documents construct and consume
    // runtime decisions; the invalid one reports diagnostics.
    assert!(
        expected
            .iter()
            .take(4)
            .all(|row| row["consumed"].as_u64().is_some_and(|n| n > 0)
                && row["evaluation"]["diagnostics"]
                    .as_array()
                    .is_some_and(Vec::is_empty)),
        "{expected:#?}"
    );
    assert!(
        expected[4].get("failure").is_some()
            || expected[4]["evaluation"]["diagnostics"]
                .as_array()
                .is_some_and(|d| !d.is_empty()),
        "{:#?}",
        expected[4]
    );
    let bytes = serde_json::to_vec(&fresh.save()?).map_err(|e| e.to_string())?;
    for mode in [
        RuntimeAdmissionLoadMode::HostCache,
        RuntimeAdmissionLoadMode::Revalidate,
    ] {
        let saved: SavedSourceBoundDefinition =
            serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let loaded = SourceBoundDraftRuntimeDefinition::load_host_cache(
            saved,
            CompilationSourcePolicy::default(),
            mode,
        )
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!(parse_all(&loaded)?, expected, "{mode:?}");
    }
    Ok(())
}
