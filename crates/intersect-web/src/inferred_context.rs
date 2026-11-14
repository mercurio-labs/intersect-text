//! Explicit host catalog installation over the shared source-authoritative frontend.
//! This module owns transport/admission only; no conversion, inference or model algorithm.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::conversion::AppliedConversionContract;
use intersect_model::definitions::conversion::ConstructorContract;
use intersect_model::definitions::conversion::ConversionContract;
use intersect_model::definitions::conversion::ConversionRegistry;
use intersect_model::definitions::setting::ComputedSettingContract;
use intersect_text::distribution::{self, CompilerIdentity, XtextInferenceFrontend};
use intersect_text::grammar::Profile;
use intersect_text::grammar::SourceLimits;
use intersect_text::package::{CompilationSourcePolicy, Package, XtextInferenceInputs};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Context {
    version: u32,
    #[serde(default)]
    conversions: Vec<ConversionContract>,
    #[serde(default)]
    applied_conversions: Vec<AppliedConversionContract>,
    #[serde(default)]
    constructors: Vec<ConstructorContract>,
    #[serde(default)]
    behavior: Option<SourceId>,
    #[serde(default)]
    computed_settings: Vec<ComputedSettingContract>,
}
impl Context {
    fn parse(value: Value) -> Result<Self, String> {
        let extended = ["behavior", "computed_settings"]
            .iter()
            .any(|key| value.get(*key).is_some());
        let complete = ["behavior", "computed_settings"]
            .iter()
            .all(|key| value.get(*key).is_some());
        let context: Self =
            serde_json::from_value(value).map_err(|e| format!("Invalid inferred context: {e}"))?;
        if !matches!(context.version, 1 | 2)
            || (context.version == 1 && extended)
            || (context.version == 2 && !complete)
        {
            return Err("Unsupported inferred context version".into());
        }
        if context
            .computed_settings
            .iter()
            .any(|c| c.provider.name != "rcore.projection" || c.provider.version != "1")
        {
            return Err("Unsupported inferred computed setting provider".into());
        }
        Ok(context)
    }
    fn request(request: &Value) -> Result<Self, String> {
        Self::parse(
            request
                .get("inference_context")
                .cloned()
                .ok_or("Missing explicit inference_context")?,
        )
    }
    fn catalog_request(request: &Value) -> Result<Self, String> {
        let context = Self::parse(
            request
                .get("inference_context")
                .cloned()
                .unwrap_or_else(|| json!({"version":1})),
        )?;
        if context.version != 1 {
            return Err(
                "Provider selection accepts only an optional catalog-only version 1 input context"
                    .into(),
            );
        }
        Ok(context)
    }
    fn value(&self) -> Value {
        let mut value = json!({"version":self.version,"behavior":self.behavior,"computed_settings":self.computed_settings});
        for (key, contracts) in [
            ("conversions", json!(self.conversions)),
            ("applied_conversions", json!(self.applied_conversions)),
            ("constructors", json!(self.constructors)),
        ] {
            if contracts.as_array().is_some_and(|rows| !rows.is_empty()) {
                value[key] = contracts;
            }
        }
        value
    }
    fn retained(package: &Value) -> Result<Self, String> {
        let mut value = json!({"version":1,
            "conversions":package["schema"].get("conversion_contracts").cloned().unwrap_or(json!([])),
            "applied_conversions":package["schema"].get("applied_conversion_contracts").cloned().unwrap_or(json!([])),
            "constructors":package["schema"].get("constructor_contracts").cloned().unwrap_or(json!([]))});
        let behavior = &package["xtext_inference"]["behavior"];
        let settings = package["grammar"]
            .get("computed_settings")
            .cloned()
            .unwrap_or(json!([]));
        if !behavior.is_null() || settings.as_array().is_some_and(|v| !v.is_empty()) {
            value["version"] = json!(2);
            value["behavior"] = behavior.clone();
            value["computed_settings"] = settings;
        }
        Self::parse(value)
    }

    fn with_registry<T>(
        &self,
        bundle: &DefinitionBundle,
        callback: impl FnOnce(&ConversionRegistry<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        if bundle.sources.iter().any(|s| s.text.len() > 256_000) {
            return Err("Definition size budget exceeded".into());
        }
        intersect_model::conversion::with_catalog_policies(
            bundle,
            &self.conversions,
            &self.applied_conversions,
            &self.constructors,
            callback,
        )
    }
    fn used(&self, package: &Package) -> Result<(), String> {
        if package.schema.conversion_contracts.len() != self.conversions.len()
            || package.schema.applied_conversion_contracts.len() != self.applied_conversions.len()
            || package.schema.constructor_contracts.len() != self.constructors.len()
        {
            return Err("Unused conversion provider selection".into());
        }
        if package
            .grammar
            .as_ref()
            .map_or(&[][..], |g| g.computed_settings.as_slice())
            != self.computed_settings
        {
            return Err("Unused or conflicting computed setting provider selection".into());
        }
        with_text_settings(package, |_| Ok(()))
    }
    fn rebuild(
        &self,
        inputs: &XtextInferenceInputs,
        requested: CompilationSourcePolicy,
        host: CompilationSourcePolicy,
        registry: &ConversionRegistry<'_>,
    ) -> Result<Package, String> {
        if inputs.behavior != self.behavior {
            return Err("Inferred behavior selection differs from explicit context".into());
        }
        let package = intersect_text::compile_inference_inputs_with_context(
            inputs,
            requested,
            host,
            &self.computed_settings,
            Some(registry),
        )
        .map_err(|e| format!("Cannot rebuild inferred context: {e:?}"))?;
        self.used(&package)?;
        Ok(package)
    }
}
pub fn compile(request: &Value) -> Result<Value, String> {
    let inputs = crate::inferred::source_inputs(request)?;
    let context = Context::request(request)?;
    context.with_registry(&inputs.bundle, |registry| {
        let result = inputs.compile_with_context(
            SourceLimits::default(),
            &context.computed_settings,
            Some(registry),
            context.behavior.as_ref(),
        );
        if let Ok(language) = &result {
            context.used(language.package())?;
        }
        Ok(crate::inferred::compiled_response(result, true))
    })
}
pub fn with_rebuilt<T>(
    retained: &Value,
    callback: impl FnOnce(&Package, &ConversionRegistry<'_>) -> Result<T, String>,
) -> Result<T, String> {
    if retained["version"].as_u64() != Some(2) || !retained["compilation"].is_null() {
        return Err("Unsupported or conflicting inferred package inputs".into());
    }
    let inputs: XtextInferenceInputs = serde_json::from_value(retained["xtext_inference"].clone())
        .map_err(|e| format!("Invalid original inference recipe: {e}"))?;
    let requested = serde_json::from_value(
        retained
            .get("source_policy")
            .cloned()
            .ok_or("Missing inference source policy")?,
    )
    .map_err(|e| format!("Invalid inference source policy: {e}"))?;
    let context = Context::retained(retained)?;
    context.with_registry(&inputs.bundle, |registry| {
        let package = context.rebuild(&inputs, requested, Default::default(), registry)?;
        callback(&package, registry)
    })
}
pub fn evaluate(request: &Value) -> Result<Value, String> {
    with_rebuilt(&request["package"], |package, registry| {
        let sources: Vec<intersect_text::Source> =
            serde_json::from_value(request["sources"].clone())
                .map_err(|e| format!("Invalid model sources: {e}"))?;
        with_text_settings(package, |settings| {
            Ok(json!(intersect_text::evaluate_with_context(
                package,
                &sources,
                intersect_text::EvaluationLimits::workbench(),
                Some(registry),
                settings,
                &crate::semantic_bindings::Derived::new(package),
            )))
        })
    })
}
pub fn export(request: &Value) -> Result<Value, String> {
    let context = Context::request(request)?;
    // A retained recipe may contain behavior; it must agree with the explicit host selection.
    let mut original = request.clone();
    if let Some(recipe) = original["inference"].as_object_mut()
        && let Some(selected) = recipe.remove("behavior")
    {
        let selected: Option<SourceId> = serde_json::from_value(selected)
            .map_err(|e| format!("Invalid behavior source: {e}"))?;
        if selected != context.behavior {
            return Err("Archive recipe behavior differs from explicit context".into());
        }
    }
    let input = crate::inferred::source_inputs(&original)?;
    let metadata = serde_json::from_value(request["metadata"].clone())
        .map_err(|e| format!("Invalid package metadata: {e}"))?;
    let inputs = XtextInferenceInputs {
        profile: None,
        version: input.version,
        bundle: input.bundle,
        grammar: input.grammar,
        behavior: context.behavior.clone(),
    };
    context.with_registry(&inputs.bundle, |registry| {
        let archive = distribution::build_inferred_with_context(
            inputs.clone(),
            metadata,
            &context.computed_settings,
            Some(registry),
            Default::default(),
            Default::default(),
        )?;
        context.used(&archive.package)?;
        serde_json::to_value(archive).map_err(|e| e.to_string())
    })
}
struct Frontend<'a>(&'a Context);
impl XtextInferenceFrontend for Frontend<'_> {
    fn identity(&self) -> CompilerIdentity {
        distribution::inference_compiler_identity()
    }
    fn profile(&self) -> Profile {
        Profile::NativeV56
    }
    fn supports_profile(&self, profile: Profile) -> bool {
        matches!(profile, Profile::NativeV56 | Profile::NativeV57)
    }
    fn compile(
        &self,
        inputs: &XtextInferenceInputs,
        requested: CompilationSourcePolicy,
        host: CompilationSourcePolicy,
    ) -> Result<Package, String> {
        self.0.with_registry(&inputs.bundle, |registry| {
            self.0.rebuild(inputs, requested, host, registry)
        })
    }
}
pub fn import(request: &Value) -> Result<Value, String> {
    let context = Context::request(request)?;
    let files = serde_json::from_value(request["files"].clone())
        .map_err(|e| format!("Invalid archive files: {e}"))?;
    // The ordinary inventory/digest/IR verifier admits original inputs before
    // this installed frontend constructs a catalog. No archived code selects a factory.
    let archive = distribution::import_files_with_inference_frontend(
        files,
        &context.computed_settings,
        Default::default(),
        &Frontend(&context),
    )?;
    if archive.package.xtext_inference.is_none() {
        return Err("Explicit inferred context requires an inferred source archive".into());
    }
    let keywords = intersect_text::keywords(&archive.package);
    let mut result = serde_json::to_value(archive).map_err(|e| e.to_string())?;
    result["keywords"] = json!(keywords);
    Ok(result)
}

/// Retained requirements are selectors; callers first rebuild their original source recipe.
pub fn contracts(package: &Package) -> &[ComputedSettingContract] {
    package
        .grammar
        .as_ref()
        .map_or(&[], |g| g.computed_settings.as_slice())
}
pub fn with_text_settings<T>(
    package: &Package,
    callback: impl FnOnce(Option<&intersect_text::setting::TextSettings<'_>>) -> Result<T, String>,
) -> Result<T, String> {
    use intersect_text::setting::{RcoreProjection, SourceSettingValues, TextSettings};
    let required = contracts(package);
    if required.is_empty() {
        return callback(None);
    }
    let bundle = crate::conversions::retained_bundle(package)?;
    let providers = required
        .iter()
        .map(|c| RcoreProjection::new(package, &bundle, c))
        .collect::<Result<Vec<_>, _>>()?;
    let selected = providers
        .iter()
        .map(|p| p as &dyn SourceSettingValues)
        .collect::<Vec<_>>();
    let settings = TextSettings::new(package, &bundle, &selected)?;
    callback(Some(&settings))
}

/// Apply a script operation through an exact source-bound inferred contract.
/// The caller has already rebuilt the package from its original source recipe.
pub fn edit_computed(
    package: &Package,
    model: &mut intersect_model::core::model::Model,
    id: &str,
    feature: &str,
    edit: intersect_model::core::storage::ComputedEdit,
    conversions: &ConversionRegistry<'_>,
) -> Result<(), String> {
    use intersect_model::core::{frozen::FrozenStore, query::QueryBudget, storage::export};
    use intersect_text::setting::{RcoreProjection, SourceBoundSettings};
    let class = &model
        .elements
        .iter()
        .find(|object| object.id == id)
        .ok_or("Missing element")?
        .class;
    let expected = contracts(package)
        .iter()
        .find(|contract| contract.class == *class && contract.feature == feature)
        .ok_or("Missing source-bound computed setting contract")?;
    let bundle = crate::conversions::retained_bundle(package)?;
    let provider = RcoreProjection::new(package, &bundle, expected)?;
    let mut store = FrozenStore::from_materialized(&package.schema, model)?;
    SourceBoundSettings {
        schema: &package.schema,
        bundle: &bundle,
        expected,
        provider: &provider,
        conversions: Some(conversions),
    }
    .apply(&mut store, id, edit, &mut QueryBudget::new(100_000))?;
    *model = export(&store)?;
    Ok(())
}

/// Structural authored choices only; this does not activate a language/provider.
pub fn provider_candidates(request: &Value) -> Result<Value, String> {
    let inputs = crate::inferred::source_inputs(request)?;
    let catalog = Context::catalog_request(request)?;
    catalog.with_registry(&inputs.bundle, |registry| {
        match intersect_text::inference::providers::candidates(
            &inputs,
            SourceLimits::default(),
            Some(registry),
        ) {
            Ok(report) => {
                let mut response = serde_json::to_value(report).map_err(|e| e.to_string())?;
                response["ok"] = json!(true);
                Ok(response)
            }
            Err(diagnostics) => Ok(crate::inferred::compiled_response(Err(diagnostics), true)),
        }
    })
}
/// Native authored selection builds complete checked context; no retained package
/// or JavaScript-computed class/member/revision identity participates.
pub fn select_providers(request: &Value) -> Result<Value, String> {
    let inputs = crate::inferred::source_inputs(request)?;
    let selection: intersect_text::inference::providers::ProviderSelection =
        serde_json::from_value(
            request
                .get("provider_selection")
                .cloned()
                .ok_or("Missing explicit provider_selection")?,
        )
        .map_err(|e| format!("Invalid authored provider selection: {e}"))?;
    let catalog = Context::catalog_request(request)?;
    catalog.with_registry(&inputs.bundle, |registry| {
        let selected = match intersect_text::inference::providers::compile_selected(
            &inputs,
            &selection,
            SourceLimits::default(),
            Some(registry),
        ) {
            Ok(selected) => selected,
            Err(diagnostics) => {
                return Ok(crate::inferred::compiled_response(Err(diagnostics), true));
            }
        };
        let context = Context {
            version: 2,
            conversions: catalog.conversions.clone(),
            applied_conversions: catalog.applied_conversions.clone(),
            constructors: catalog.constructors.clone(),
            behavior: Some(selected.selection().behavior.clone()),
            computed_settings: selected.computed_settings().to_vec(),
        };
        context.used(selected.language().package())?;
        let mut response = crate::inferred::compiled_response(Ok(selected.into_language()), true);
        response["provider_selection"] = json!(selection);
        response["inference_context"] = context.value();
        Ok(response)
    })
}
/// Explicit contextual preview preserves the legacy context-free rejection route.
pub fn structure_preview(request: &Value) -> Result<Value, String> {
    let inputs = crate::inferred::source_inputs(request)?;
    let context = Context::request(request)?;
    context.with_registry(&inputs.bundle, |registry| {
        let language = inputs
            .compile_with_context(
                SourceLimits::default(),
                &context.computed_settings,
                Some(registry),
                context.behavior.as_ref(),
            )
            .map_err(|diagnostics| format!("Cannot rebuild inferred preview: {diagnostics:?}"))?;
        context.used(language.package())?;
        crate::inferred::structure_preview_language(request, &language)
    })
}
