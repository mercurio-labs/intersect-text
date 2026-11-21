//! Real generic language fixtures; also compiled against the immutable old host
//! in a separate executable for exact before/after qualification.
use super::loader;
use intersect_model::{
    core::{
        frozen::{FrozenStore, Metamodel},
        query::QueryBudget,
    },
    definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        ecore::ParseLimits,
        rcore_behavior::ir_v2::Limits,
        rcore_scope::{
            BindingKind, compile_binding_kinds_with_source_limits, source_scope_bindings,
            source_validate_bindings,
        },
    },
    frozen_scopes::RcoreScopes,
};
use intersect_text::{
    EvaluationLimits, Source,
    document::AnalysisLimits,
    draft::{DraftLanguage, GrammarPolicy, OpenDraftCapability},
    grammar::Profile,
    package::{BundleCompilation, CompilationSourcePolicy},
};
use serde_json::{Value, json};
use std::{cell::RefCell, sync::Arc};

const ECORE: &str = include_str!("../../../fixtures/frozen-text-catalog/catalog.ecore");
const GRAMMAR: &str = include_str!("../../../fixtures/frozen-text-catalog/catalog.xtext");
const SCOPES: &str = include_str!("../../../fixtures/frozen-text-catalog/catalog-scoping.rcore");
const TYPES: &str = include_str!("../../../fixtures/frozen-text-catalog/types.catalog");
const SHOP: &str = include_str!("../../../fixtures/frozen-text-catalog/shop.catalog");
const TERMINALS: &str = include_str!(
    "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
);

fn observe(documents: &[(&str, &str)], invalid_structure: bool) -> Result<Value, String> {
    let ecore = DefinitionSource::new("catalog.ecore", SourceKind::Ecore, ECORE);
    let mut terminals = DefinitionSource::new(
        "org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        TERMINALS,
    );
    terminals.dependencies.push(ecore.id.clone());
    let grammar_text = if invalid_structure {
        GRAMMAR.replace("'datatype' name=ID", "{cat::DataType} 'datatype' ID")
    } else {
        GRAMMAR.into()
    };
    let mut grammar = DefinitionSource::new("catalog.xtext", SourceKind::Xtext, grammar_text);
    grammar
        .dependencies
        .extend([ecore.id.clone(), terminals.id.clone()]);
    let mut rules = DefinitionSource::new(
        "rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package emptyRules version \"1\";",
    );
    rules.dependencies.push(ecore.id.clone());
    let input = BundleCompilation {
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), terminals, grammar.clone(), rules.clone()],
            namespaces: vec![],
        },
        ecore: ecore.id.clone(),
        xtext: Some(grammar.id),
        rcore_behavior: rules.id,
        namespace: "urn:intersect:example:catalog:1".into(),
        materialized_types: vec![],
    };
    let migration = intersect_model::migration::migrate_ecore_bundle(
        input.bundle.clone(),
        &[(ecore.id, "catalog.rcore".into())].into(),
    )
    .map_err(|e| e.to_string())?;
    let input = intersect_text::migration::remap_compilation(&migration, &input)?;
    let policy = CompilationSourcePolicy::default();
    let definition = intersect_text::compiler::prepare_bundle_with_borrowed_authored_validation(
        &input,
        &migration,
        Profile::NativeV80,
        &[],
        None,
        policy,
        policy,
        &|_, _, _| Ok(()),
    )
    .map_err(|e| format!("{e:?}"))?;
    let inner = GrammarPolicy::new(&definition);
    let capability = OpenDraftCapability::new(&definition, &inner)?;
    let language = DraftLanguage::bind(&definition, &capability, AnalysisLimits::workbench())?;
    let mm = Arc::new(Metamodel::freeze(definition.schema()).map_err(|e| e.to_string())?);
    let structure = definition
        .sources()
        .sources
        .iter()
        .find(|s| s.kind == SourceKind::Rcore)
        .ok_or("Missing migrated structure")?
        .clone();
    let mut scopes = DefinitionSource::new(
        "scopes.rcore",
        SourceKind::RcoreBehavior,
        format!(
            r#"{SCOPES}
bind validate c.Entity = notices;
fn notices(self:c.Entity)->Sequence<Issue> =
 if self.name == "Bad" {{ [issue_at("error", "fixture.error", "Rejected entity.", "name")] }}
 else {{ if self.name == "Warn" {{ [issue_at("warning", "fixture.warning", "Review entity.", "name")] }} else {{ [] }} }};
"#
        ),
    );
    scopes.dependencies.push(structure.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![structure, scopes.clone()],
        namespaces: vec![],
    };
    let limits = ParseLimits {
        source_bytes: 1_000_000,
    };
    let mut declarations: Vec<_> = source_scope_bindings(&scopes, limits)
        .map_err(|e| format!("{e:?}"))?
        .into_iter()
        .map(|b| (b.declaration, BindingKind::Scope))
        .collect();
    declarations.extend(
        source_validate_bindings(&scopes, limits)
            .map_err(|e| format!("{e:?}"))?
            .into_iter()
            .map(|b| (b.declaration, BindingKind::Validate)),
    );
    let compiled = compile_binding_kinds_with_source_limits(
        &bundle,
        &scopes.id,
        &declarations,
        &Limits::default(),
        limits,
    )
    .map_err(|e| format!("{e:?}"))?;
    let resolvers = RcoreScopes::new(&mm, &bundle, &compiled).map_err(|e| e.to_string())?;
    let sources: Vec<_> = documents
        .iter()
        .map(|(uri, text)| Source {
            uri: (*uri).into(),
            text: (*text).into(),
        })
        .collect();
    let phases = RefCell::new(Vec::new());
    let loaded = loader::load_frozen(
        FrozenStore::new(mm.clone()),
        &sources,
        |source, store| {
            language.parse_runtime_into(
                source,
                EvaluationLimits::native(),
                store,
                &mut QueryBudget::new(10_000_000),
            )
        },
        &resolvers,
        &|phase| phases.borrow_mut().push(phase),
    )?;
    let model = loaded
        .model
        .as_ref()
        .map(|m| m.to_model().map_err(|e| e.to_string()))
        .transpose()?;
    Ok(
        json!({"clean":loaded.is_clean(),"model":model,"diagnostics":loaded.diagnostics,
        "references":loaded.references,"resolved":loaded.resolved,"navigation":loaded.navigation,
        "proxies":loaded.proxies,"unresolvedOccurrences":loaded.unresolved_occurrences,"phases":phases.into_inner()}),
    )
}

#[test]
fn real_loading_preserves_phases_diagnostics_and_multi_resource_navigation() -> Result<(), String> {
    let mut cases = Vec::new();
    for (name, docs, invalid) in [
        (
            "clean",
            vec![("types.catalog", TYPES), ("shop.catalog", SHOP)],
            false,
        ),
        (
            "unresolved",
            vec![("missing.catalog", "entity E { value : Missing }")],
            false,
        ),
        ("parse", vec![("parse.catalog", "datatype @")], false),
        (
            "structure",
            vec![("structure.catalog", "datatype Alpha")],
            true,
        ),
        (
            "error",
            vec![("error.catalog", "// 😀\r\nentity Bad {}")],
            false,
        ),
        (
            "warning",
            vec![("warning.catalog", "// 😀\r\nentity Warn {}")],
            false,
        ),
    ] {
        let value = observe(&docs, invalid)?;
        match name {
            "clean" => {
                assert_eq!(value["clean"], true);
                assert_eq!(value["references"], 9);
                assert_eq!(value["resolved"], 9);
                assert_eq!(value["navigation"].as_array().ok_or("navigation")?.len(), 9);
            }
            "warning" => {
                assert_eq!(value["clean"], true);
                assert_eq!(value["diagnostics"][0]["code"], "fixture.warning");
            }
            "error" => {
                assert_eq!(value["clean"], false);
                assert_eq!(value["diagnostics"][0]["code"], "fixture.error");
            }
            _ => {
                assert_eq!(value["clean"], false, "{name}: {value}");
                assert!(
                    !value["diagnostics"]
                        .as_array()
                        .ok_or("diagnostics")?
                        .is_empty()
                );
            }
        }
        if name == "unresolved" {
            assert_eq!(value["diagnostics"][0]["phase"], "link");
            assert_eq!(value["diagnostics"][0]["resource"], "missing.catalog");
        }
        if name == "parse" {
            assert_eq!(value["diagnostics"][0]["phase"], "parse");
        }
        if name == "structure" {
            assert_eq!(value["diagnostics"][0]["phase"], "validate");
        }
        if matches!(name, "clean" | "warning" | "error") {
            assert_eq!(
                value["phases"],
                json!([
                    "parse_end",
                    "link_end",
                    "implicit_end",
                    "defaults_end",
                    "validate_end",
                    "check_end"
                ])
            );
        }
        cases.push(json!({"case":name,"outcome":value}));
    }
    if let Ok(path) = std::env::var("INTERSECT_LOAD_RECEIPT") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&cases).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
