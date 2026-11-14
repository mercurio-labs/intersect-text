//! Handwritten JSON transport over the source-authoritative entry interpreter.
use intersect_model::definitions::conversion::ConversionRegistry;
use intersect_text::{EvaluationLimits, Source, entry::EntryLanguage};
use serde_json::{Value, json};

fn with_language<T>(
    request: &Value,
    callback: impl FnOnce(&EntryLanguage, Option<&ConversionRegistry<'_>>) -> Result<T, String>,
) -> Result<T, String> {
    // Use the same untrusted-package/source/provider reconstruction as the other
    // JSON consumers. Cached grammar programs and entry inventories are not authority.
    let package = crate::scripting::package(request)?;
    crate::conversions::with_package(&package, |registry| {
        let language = EntryLanguage::prepare_with_context(
            &package,
            crate::inferred_context::contracts(&package),
            Some(registry),
            Default::default(),
        )
        .map_err(|errors| format!("Invalid entry language: {errors:?}"))?;
        callback(&language, Some(registry))
    })
}

pub fn catalog(request: &Value) -> Result<Value, String> {
    with_language(request, |language, _| {
        let default_entry = &language
            .package()
            .grammar
            .as_ref()
            .ok_or("Package has no text grammar")?
            .entry;
        let entries = language
            .entries()
            .iter()
            .map(|(name, rule)| {
                (
                    name.clone(),
                    json!({"declaration":rule.declaration,"datatype":rule.datatype}),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        Ok(json!({"default_entry":default_entry,"entries":entries}))
    })
}

pub fn evaluate(request: &Value) -> Result<Value, String> {
    let entry = request["entry_rule"]
        .as_str()
        .ok_or("entry_rule must be a parser entry name")?;
    let sources: Vec<Source> = serde_json::from_value(
        request
            .get("sources")
            .cloned()
            .ok_or("Missing model sources")?,
    )
    .map_err(|error| format!("Invalid model sources: {error}"))?;
    with_language(request, |language, registry| {
        let result = crate::inferred_context::with_text_settings(language.package(), |settings| {
            language.evaluate_with_context(
                entry,
                &sources,
                EvaluationLimits::workbench(),
                registry,
                settings,
                &crate::semantic_bindings::Derived::new(language.package()),
            )
        })?;
        let mut value =
            serde_json::to_value(result.evaluation).map_err(|error| error.to_string())?;
        value["entry_values"] =
            serde_json::to_value(result.values).map_err(|error| error.to_string())?;
        Ok(value)
    })
}
