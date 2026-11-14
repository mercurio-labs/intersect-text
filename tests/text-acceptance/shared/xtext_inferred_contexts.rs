//! Handwritten native provider-context controls over shared compilation/storage.
//! Native extension contracts, not additional independent JVM observations.
use intersect_model::core::query::DerivedValues;
use intersect_model::core::query::QueryBudget;
use intersect_model::core::storage::ComputedEdit;
use intersect_model::core::storage::ModelRead;
use intersect_model::core::storage::SettingEdit;
use intersect_model::core::storage::SettingValues;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::conversion::ConversionContract;
use intersect_model::definitions::conversion::ProviderId;
use intersect_model::definitions::setting::ComputedSettingContract;
use intersect_model::definitions::setting::SettingOperation;
use intersect_model::query::PackageDerived;
use intersect_model::setting::SourceSettingValues;
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
use intersect_text::package::{CompilationSourcePolicy, Package};
use intersect_text::setting::TextSettings;
use intersect_text::{
    EvaluationLimits, Source,
    entry::EntryLanguage,
    inference::{InferenceRequest, infer_metamodels},
};
use serde_json::{Value, json};
use std::cell::Cell;
fn rows(behavior: bool) -> Vec<Value> {
    let text = if behavior {
        include_str!("../../../fixtures/xtext/inferred-contexts/behavior-entry-explicit-cases.json")
    } else {
        include_str!("../../../fixtures/xtext/inferred-contexts/construction-group-cases.json")
    };
    serde_json::from_str::<Value>(text).unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}
fn request(row: &Value) -> InferenceRequest {
    serde_json::from_value(row["inference"].clone()).unwrap()
}
fn source(row: &Value) -> Source {
    Source {
        uri: "memory:context.model".into(),
        text: row["model_text"].as_str().unwrap().into(),
    }
}
fn rgb(row: &Value) -> ConversionContract {
    let input = request(row);
    let model = &input.bundle.sources[0].id;
    let datatype = intersect_model::definitions::resources::ResourceSet::new(&input.bundle)
        .unwrap()
        .resolve(model, "#//RGB")
        .unwrap();
    ConversionContract {
        declarations: vec![],
        members: vec![],
        datatype,
        provider: ProviderId {
            name: "example.rgb".into(),
            version: "1".into(),
        },
        serializable: true,
    }
}
fn class(package: &Package, namespace: &str, name: &str) -> String {
    package
        .schema
        .declarations
        .iter()
        .find(|(_, d)| d.namespace == namespace && d.name == name)
        .unwrap()
        .0
        .clone()
}
fn setting(row: &Value) -> ComputedSettingContract {
    let request = request(row);
    let mut definitions =
        infer_metamodels(&request.bundle, &request.grammar, SourceLimits::default()).unwrap();
    intersect_text::grammar::prepare_ecore_reference_context(&mut definitions.schema).unwrap();
    let schema = &definitions.schema;
    let class = schema
        .declarations
        .iter()
        .find(|(_, d)| d.namespace == "urn:inferred:settings" && d.name == "Item")
        .unwrap()
        .0
        .clone();
    ComputedSettingContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class_identity: schema.identity(&class),
        member: schema
            .effective_feature(&class, "computed")
            .unwrap()
            .identity,
        class,
        feature: "computed".into(),
        provider: ProviderId {
            name: "test.projection".into(),
            version: "1".into(),
        },
        sources: request
            .bundle
            .sources
            .iter()
            .map(|s| s.id.clone())
            .collect(),
        operations: vec![SettingOperation::Set],
    }
}
fn verify_archive(
    package: &Package,
    settings: &[ComputedSettingContract],
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
) {
    use intersect_text::distribution::{self, PackageMetadata};
    let archive = distribution::build_inferred_with_context(
        package.xtext_inference.clone().unwrap(),
        PackageMetadata {
            package_id: "test.inferred-context".into(),
            release: "1".into(),
            display_name: "Inferred provider context".into(),
            description: "Native provider controls".into(),
        },
        settings,
        conversions,
        package.source_policy.unwrap(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&archive.package).unwrap(),
        serde_json::to_value(package).unwrap()
    );
    let mut files = archive.files.clone();
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&archive.manifest).unwrap(),
    );
    assert!(distribution::import_files(files.clone()).is_err());
    let restored = distribution::import_files_with_context(
        files.clone(),
        settings,
        conversions,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&restored.package).unwrap(),
        serde_json::to_value(package).unwrap()
    );
    assert_eq!(restored.files, archive.files);
    let mut manifest = serde_json::to_value(&archive.manifest).unwrap();
    manifest["compiler"]["name"] = json!("uninstalled");
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&manifest).unwrap(),
    );
    assert!(
        distribution::import_files_with_context(files, settings, conversions, Default::default())
            .is_err()
    );
}
struct Projection<'a> {
    package: &'a Package,
    contract: ComputedSettingContract,
    calls: Cell<usize>,
    invalid: bool,
}
impl SourceSettingValues for Projection<'_> {
    fn contract(&self) -> &ComputedSettingContract {
        &self.contract
    }
}
impl DerivedValues for Projection<'_> {
    fn schema_revision(&self) -> Option<&str> {
        Some(&self.package.schema.revision)
    }
    fn supports(&self, class: &str, feature: &str) -> bool {
        PackageDerived(self.package).supports(class, feature)
    }
    fn read(
        &self,
        store: &dyn ModelRead,
        id: &str,
        feature: &str,
        budget: &mut QueryBudget,
    ) -> Result<Value, String> {
        PackageDerived(self.package).read(store, id, feature, budget)
    }
}
impl SettingValues for Projection<'_> {
    fn plan(
        &self,
        store: &dyn ModelRead,
        id: &str,
        feature: &str,
        edit: &ComputedEdit,
        budget: &mut QueryBudget,
    ) -> Result<Vec<SettingEdit>, String> {
        self.calls.set(self.calls.get() + 1);
        if self.invalid {
            return Ok(vec![SettingEdit::Set {
                id: id.into(),
                feature: "backing".into(),
                value: json!(false),
            }]);
        }
        PackageDerived(self.package).plan(store, id, feature, edit, budget)
    }
}
#[cfg_attr(all(test, not(target_arch = "wasm32")), test)]
pub fn actual_conversion_defaults_replay_and_named_entries_use_the_installed_registry() {
    for row in rows(false)
        .into_iter()
        .filter(|r| r["id"].as_str().unwrap().starts_with("rgb"))
    {
        let input = request(&row);
        assert!(input.compile(SourceLimits::default()).is_err());
        intersect_model::conversion::with_catalog(&input.bundle, &[rgb(&row)], |registry| {
            let language = input
                .compile_with_context(SourceLimits::default(), &[], Some(registry), None)
                .unwrap();
            let package = language.package();
            assert_eq!(
                package.grammar.as_ref().unwrap().profile,
                if row["id"].as_str().unwrap().contains("plain") {
                    Profile::NativeV56
                } else {
                    Profile::NativeV57
                }
            );
            assert_eq!(package.xtext_inference.as_ref().unwrap().behavior, None);
            verify_archive(package, &[], Some(registry));
            let evaluation = intersect_text::evaluate_with_conversions(
                package,
                &[source(&row)],
                EvaluationLimits::native(),
                registry,
            );
            assert!(
                evaluation.diagnostics.is_empty(),
                "{:?}",
                evaluation.diagnostics
            );
            assert_eq!(evaluation.model.elements.len(), 2);
            let paint = class(package, "urn:inferred:paint", "Paint");
            let object = evaluation
                .model
                .elements
                .iter()
                .find(|e| e.class == paint)
                .unwrap();
            assert_eq!(object.values.len(), 1);
            let model = evaluation
                .model
                .elements
                .iter()
                .find(|e| e.id != object.id)
                .unwrap();
            assert_eq!(
                serde_json::to_value(&model.values).unwrap(),
                json!({"child":object.id})
            );
            assert_eq!(
                object.values["color"]["value"],
                json!({"red":18,"green":58,"blue":188})
            );
            let replay = intersect_text::recompile_with_context(
                package,
                &[],
                Some(registry),
                Default::default(),
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(&replay).unwrap(),
                serde_json::to_value(package).unwrap()
            );
            assert!(intersect_text::recompile(package).is_err());
            let entry = EntryLanguage::prepare_with_context(
                package,
                &[],
                Some(registry),
                Default::default(),
            )
            .unwrap();
            let result = entry
                .evaluate_with_context(
                    "Paint",
                    &[source(&row)],
                    EvaluationLimits::native(),
                    Some(registry),
                    None,
                    &PackageDerived(entry.package()),
                )
                .unwrap();
            assert!(
                result.evaluation.diagnostics.is_empty(),
                "{:?}",
                result.evaluation.diagnostics
            );
            assert_eq!(result.evaluation.model.elements.len(), 1);
            assert_eq!(
                result.evaluation.model.elements[0].values["color"],
                object.values["color"]
            );
            let tiny = CompilationSourcePolicy {
                structure_bytes: 1,
                xtext_bytes: 1,
                ..Default::default()
            };
            assert!(
                intersect_text::recompile_with_context(package, &[], Some(registry), tiny).is_err()
            );
            let mut forged = package.clone();
            forged.schema.classes.clear();
            forged.sources.clear();
            assert_eq!(
                serde_json::to_value(
                    intersect_text::recompile_with_context(
                        &forged,
                        &[],
                        Some(registry),
                        Default::default()
                    )
                    .unwrap()
                )
                .unwrap(),
                serde_json::to_value(package).unwrap()
            );
            Ok(())
        })
        .unwrap();
    }
}
#[cfg_attr(all(test, not(target_arch = "wasm32")), test)]
pub fn source_backed_settings_execute_replay_and_preserve_exact_behavior_selection() {
    for row in rows(true) {
        let input = request(&row);
        let behavior: SourceId = serde_json::from_value(row["behavior"].clone()).unwrap();
        let contract = setting(&row);
        assert!(
            input
                .compile_with_context(
                    SourceLimits::default(),
                    std::slice::from_ref(&contract),
                    None,
                    None
                )
                .is_err()
        );
        let language = input
            .compile_with_context(
                SourceLimits::default(),
                std::slice::from_ref(&contract),
                None,
                Some(&behavior),
            )
            .unwrap();
        let package = language.package();
        verify_archive(package, std::slice::from_ref(&contract), None);
        assert_eq!(
            package.xtext_inference.as_ref().unwrap().behavior,
            Some(behavior.clone())
        );
        assert_eq!(
            serde_json::to_value(&package.xtext_inference.as_ref().unwrap().bundle).unwrap(),
            serde_json::to_value(&input.bundle).unwrap()
        );
        let replay = intersect_text::recompile_with_context(
            package,
            std::slice::from_ref(&contract),
            None,
            Default::default(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&replay).unwrap(),
            serde_json::to_value(package).unwrap()
        );
        assert!(intersect_text::recompile(package).is_err());
        let entry = EntryLanguage::prepare_with_context(
            package,
            std::slice::from_ref(&contract),
            None,
            Default::default(),
        )
        .unwrap();
        let package = entry.package();
        let provider = Projection {
            package,
            contract: contract.clone(),
            calls: Cell::new(0),
            invalid: false,
        };
        let bundle = &package.xtext_inference.as_ref().unwrap().bundle;
        let selected = TextSettings::new(package, bundle, &[&provider]).unwrap();
        let result = entry
            .evaluate_with_context(
                "Model",
                &[source(&row)],
                EvaluationLimits::native(),
                None,
                Some(&selected),
                &PackageDerived(package),
            )
            .unwrap();
        assert!(
            result.evaluation.diagnostics.is_empty(),
            "{:?}",
            result.evaluation.diagnostics
        );
        assert_eq!(result.evaluation.model.elements.len(), 2);
        assert_eq!(provider.calls.get(), 1);
        let item = class(package, "urn:inferred:settings", "Item");
        let object = result
            .evaluation
            .model
            .elements
            .iter()
            .find(|e| e.class == item)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&object.values).unwrap(),
            json!({"backing":"alpha"})
        );
        let model = result
            .evaluation
            .model
            .elements
            .iter()
            .find(|e| e.id != object.id)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&model.values).unwrap(),
            json!({"child":object.id})
        );
        assert_eq!(
            result.evaluation.derived[&object.id]["computed"],
            json!("alpha")
        );
        provider.calls.set(0);
        let result = entry
            .evaluate_with_context(
                "Item",
                &[Source {
                    text: row["named_model_text"].as_str().unwrap().into(),
                    ..source(&row)
                }],
                EvaluationLimits::native(),
                None,
                Some(&selected),
                &PackageDerived(package),
            )
            .unwrap();
        assert!(
            result.evaluation.diagnostics.is_empty(),
            "{:?}",
            result.evaluation.diagnostics
        );
        assert_eq!(result.evaluation.model.elements.len(), 1);
        assert_eq!(
            provider.calls.get(),
            usize::from(row["id"].as_str().unwrap().contains("plain"))
        );
        assert!(
            entry
                .evaluate("Model", &[source(&row)], EvaluationLimits::native())
                .is_err()
        );
    }
}
#[cfg_attr(all(test, not(target_arch = "wasm32")), test)]
pub fn settings_failures_reject_stale_sources_contracts_borrows_and_invalid_plans() {
    let row = rows(true).remove(0);
    let input = request(&row);
    let behavior: SourceId = serde_json::from_value(row["behavior"].clone()).unwrap();
    let contract = setting(&row);
    let language = input
        .compile_with_context(
            SourceLimits::default(),
            std::slice::from_ref(&contract),
            None,
            Some(&behavior),
        )
        .unwrap();
    let package = language.package();
    let mut changed = contract.clone();
    changed.provider.version = "stale".into();
    assert!(
        intersect_text::recompile_with_context(package, &[changed], None, Default::default())
            .is_err()
    );
    assert!(
        input
            .compile_with_context(
                SourceLimits::default(),
                std::slice::from_ref(&contract),
                None,
                Some(&input.grammar)
            )
            .is_err()
    );
    let mut stale = behavior.clone();
    stale.sha256 = "0".repeat(64);
    assert!(
        input
            .compile_with_context(
                SourceLimits::default(),
                std::slice::from_ref(&contract),
                None,
                Some(&stale)
            )
            .is_err()
    );
    let entry = EntryLanguage::prepare_with_context(
        package,
        std::slice::from_ref(&contract),
        None,
        Default::default(),
    )
    .unwrap();
    let provider = Projection {
        package,
        contract: contract.clone(),
        calls: Cell::new(0),
        invalid: false,
    };
    let selected = TextSettings::new(package, &input.bundle, &[&provider]).unwrap();
    assert!(
        entry
            .evaluate_with_context(
                "Model",
                &[source(&row)],
                EvaluationLimits::native(),
                None,
                Some(&selected),
                &PackageDerived(entry.package())
            )
            .unwrap_err()
            .contains("another package")
    );
    let package = entry.package();
    let provider = Projection {
        package,
        contract: contract.clone(),
        calls: Cell::new(0),
        invalid: true,
    };
    let selected = TextSettings::new(package, &input.bundle, &[&provider]).unwrap();
    let failed = entry
        .evaluate_with_context(
            "Model",
            &[source(&row)],
            EvaluationLimits::native(),
            None,
            Some(&selected),
            &PackageDerived(package),
        )
        .unwrap_err();
    assert!(failed.contains("Invalid EString"), "{failed}");
    assert_eq!(provider.calls.get(), 1);
    let provider = Projection {
        package,
        contract: contract.clone(),
        calls: Cell::new(0),
        invalid: false,
    };
    let selected = TextSettings::new(package, &input.bundle, &[&provider]).unwrap();
    let retry = entry
        .evaluate_with_context(
            "Model",
            &[source(&row)],
            EvaluationLimits::native(),
            None,
            Some(&selected),
            &PackageDerived(package),
        )
        .unwrap();
    assert!(
        retry.evaluation.diagnostics.is_empty(),
        "{:?}",
        retry.evaluation.diagnostics
    );
    assert_eq!(retry.evaluation.model.elements.len(), 2);
    let mut forged = package.clone();
    forged.xtext_inference.as_mut().unwrap().bundle.sources[0]
        .text
        .push(' ');
    assert!(
        intersect_text::recompile_with_context(&forged, &[contract], None, Default::default())
            .is_err()
    );
}
#[cfg_attr(all(test, not(target_arch = "wasm32")), test)]
pub fn provider_context_admission_keeps_source_and_budget_failures_atomic() {
    for row in rows(false) {
        let input = request(&row);
        let mut stale = rgb(&row);
        stale.provider.version = "missing".into();
        assert!(
            intersect_model::conversion::with_catalog(&input.bundle, &[stale], |_| Ok(())).is_err()
        );
        let mut stale = rgb(&row);
        stale.datatype.source.sha256 = "0".repeat(64);
        assert!(
            intersect_model::conversion::with_catalog(&input.bundle, &[stale], |_| Ok(())).is_err()
        );
        intersect_model::conversion::with_catalog(&input.bundle, &[rgb(&row)], |registry| {
            assert!(
                input
                    .compile_with_context(
                        SourceLimits {
                            source_bytes: 1,
                            ..Default::default()
                        },
                        &[],
                        Some(registry),
                        None
                    )
                    .is_err()
            );
            let language = input
                .compile_with_context(Default::default(), &[], Some(registry), None)
                .unwrap();
            let failed = intersect_text::evaluate_with_conversions(
                language.package(),
                &[source(&row)],
                EvaluationLimits {
                    elements_per_resource: Some(1),
                    ..EvaluationLimits::native()
                },
                registry,
            );
            assert!(!failed.diagnostics.is_empty());
            assert!(failed.model.elements.is_empty());
            Ok(())
        })
        .unwrap();
    }
    let original: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-contexts/behavior-cases.json"
    ))
    .unwrap();
    let original = &original["cases"][0];
    let behavior: SourceId = serde_json::from_value(original["behavior"].clone()).unwrap();
    let original_contract = setting(original);
    let language = request(original)
        .compile_with_context(
            Default::default(),
            std::slice::from_ref(&original_contract),
            None,
            Some(&behavior),
        )
        .unwrap();
    let entry = EntryLanguage::prepare_with_context(
        language.package(),
        &[original_contract],
        None,
        Default::default(),
    )
    .unwrap();
    assert!(!entry.entries().contains_key("Item"));
    let missing = entry
        .evaluate("Item", &[source(original)], EvaluationLimits::native())
        .unwrap();
    assert_eq!(
        missing.evaluation.diagnostics[0].message,
        "No emitted no-argument parser entry for Item"
    );
    assert!(missing.evaluation.model.elements.is_empty());
    let row = rows(true).remove(0);
    let mut input = request(&row);
    let behavior: SourceId = serde_json::from_value(row["behavior"].clone()).unwrap();
    let contract = setting(&row);
    // Keep the behavior in the source inventory but remove its graph admission.
    let grammar = input
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == input.grammar)
        .unwrap();
    grammar.dependencies.retain(|id| id != &behavior);
    let error = input
        .compile_with_context(Default::default(), &[contract], None, Some(&behavior))
        .err()
        .unwrap();
    assert!(error[0].message.contains("not an exact grammar dependency"));
}
pub fn verify() -> usize {
    actual_conversion_defaults_replay_and_named_entries_use_the_installed_registry();
    source_backed_settings_execute_replay_and_preserve_exact_behavior_selection();
    settings_failures_reject_stale_sources_contracts_borrows_and_invalid_plans();
    provider_context_admission_keeps_source_and_budget_failures_atomic();
    4
}
