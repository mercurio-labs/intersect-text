//! First actual source -> checked rules -> model consumer. Same native/WASM API.
use intersect_model::{api::ModelEdit, compile_rcore_at, rcore::RcorePackage};
use serde_json::{Value, json};
fn require(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.into()) }
}
pub fn verify() -> Result<(), String> {
    verify_diagnostics()?;
    let text = include_str!("../library/library-project.rcore");
    let package =
        compile_rcore_at("memory:library-rules.rcore", text).map_err(|e| e.to_string())?;
    let mut model = package.new_model().map_err(|e| e.to_string())?;
    for (id, class, name) in [
        ("library", "Library", "Catalog"),
        ("a", "BookCopy", "dune"),
        ("b", "BookCopy", "earthsea"),
        ("member", "Member", "ada"),
        ("first", "Loan", "first"),
        ("second", "Loan", "second"),
    ] {
        model.create(id, class, "memory:rule-model")?;
        model.set(id, "name", json!(name))?;
    }
    for (id, field, value) in [
        ("library", "copies", json!(["a", "b"])),
        ("library", "members", json!(["member"])),
        ("library", "loans", json!(["first", "second"])),
        ("first", "copy", json!("a")),
        ("second", "copy", json!("a")),
        ("first", "member", json!("member")),
        ("second", "member", json!("member")),
    ] {
        model.set(id, field, value)?;
    }
    let before = package.save(&model).map_err(|e| e.to_string())?;
    let evaluation = package
        .evaluate_model_rules(&model)
        .map_err(|e| e.to_string())?;
    let observed = serde_json::to_value(&evaluation).map_err(|e| e.to_string())?;
    let finding = json!({"source":"memory:rule-model","start":0,"end":0,
        "message":"A book copy cannot have more than one active loan."});
    let expected = json!({
        "derived":{"library":{"copyCount":{"kind":"int64","value":"2"}},
            "a":{"available":false},"b":{"available":true}},
        "diagnostics":[finding.clone(),finding],
    });
    require(
        observed == expected,
        "Direct checked source rules changed literal results",
    )?;
    require(
        package.save(&model).map_err(|e| e.to_string())? == before,
        "Rule evaluation mutated model",
    )?;
    require(
        model.set("a", "title", json!(42)).is_err(),
        "Wrong-type edit succeeded",
    )?;
    require(
        package.save(&model).map_err(|e| e.to_string())? == before,
        "Rejected edit mutated model",
    )?;
    let (reloaded, restored) = RcorePackage::reload(&before).map_err(|e| e.to_string())?;
    require(
        serde_json::to_value(
            reloaded
                .evaluate_model_rules(&restored)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?
            == expected,
        "Reload failed to rebuild model rules from exact sources",
    )?;
    let foreign =
        compile_rcore_at("memory:foreign-library-rules.rcore", text).map_err(|e| e.to_string())?;
    require(
        foreign.evaluate_model_rules(&model).is_err(),
        "Foreign source identity admitted",
    )?;
    let invalid = text.replace("size(self.copies)", "true");
    require(
        compile_rcore_at("memory:invalid-library-rules.rcore", &invalid).is_err(),
        "Invalid rule source activated",
    )?;
    let last: Value = serde_json::to_value(
        package
            .evaluate_model_rules(&model)
            .map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    require(
        last == expected,
        "Failed compilation changed earlier package",
    )
}

// Exercise both the declaration-specific lowerer error and the existing global
// check_semantics error after alias-distinct derivations select one real field.
// Literal spans are retained, rather than relabelled as the first declaration.
pub fn verify_diagnostics() -> Result<(), String> {
    use intersect_model::rcore::{DefinitionSource, RcoreError, SourceKind};
    let header = r#"rcore 1; package diagnosticRules version "1";
metamodel "urn:diagnostic-rules" prefix d {
 class Item { attr values: Int64 [0,*]; attr count: Int64 [1,1] derived; }
}
behavior {
 import model "urn:diagnostic-rules" as m;
"#;
    let good_rule = " pub derive m.Item.count(self) -> Int = size(self.values);";
    let good = format!("{header}{good_rule}\n}}");
    let package = compile_rcore_at("memory:diagnostic-good.rcore", &good)
        .map_err(|error| error.to_string())?;
    let model = package.new_model().map_err(|error| error.to_string())?;
    let before = package.save(&model).map_err(|error| error.to_string())?;
    let prior = serde_json::to_value(
        package
            .evaluate_model_rules(&model)
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let restriction = "pub derive m.Item.count(self) -> Int = size(self.values) + 1;";
    let restricted = format!("{header} {restriction}\n}}");
    let start = header.len() + 1;
    let duplicate = format!(
        "{header} import model \"urn:diagnostic-rules\" as n;\n{good_rule}\n pub derive n.Item.count(self) -> Int = size(self.values);\n}}"
    );
    let mixed = r#"rcore 1; package mixedRules version "1";
metamodel "urn:mixed-rules" prefix m {
 class Item { attr values: Int64 [0,*]; attr factor: Int64 [1,1];
   attr count: Int64 [1,1] derived; attr externalValue: Int64 [1,1] derived; }
}
behavior {
 import model "urn:mixed-rules" as m;
 extern fn custom(value: Int) -> Int;
 pub derive m.Item.count(self) -> Int = size(self.values);
 pub derive m.Item.externalValue(self) -> Int = custom(self.factor);
}"#
    .to_owned();
    for (uri, source, start, end, detail) in [
        (
            "memory:restricted-rule.rcore",
            restricted.as_str(),
            start,
            start + restriction.len(),
            "Derivation requires a supported builtin or a direct external call",
        ),
        (
            "memory:duplicate-rule.rcore",
            duplicate.as_str(),
            0,
            1,
            "Duplicate derivation",
        ),
        (
            "memory:mixed-external-rule.rcore",
            mixed.as_str(),
            0,
            1,
            "External model-rule activation requires an explicit provider adapter; declaration localization is unavailable",
        ),
    ] {
        let errors = match compile_rcore_at(uri, source) {
            Err(RcoreError::Compilation(errors)) => errors,
            Err(error) => return Err(format!("Expected compilation diagnostics: {error}")),
            Ok(_) => return Err("Unsupported or duplicate rule source activated".into()),
        };
        require(errors.len() == 1, "Rule lowering changed diagnostic count")?;
        let expected_source = DefinitionSource::new(uri, SourceKind::Rcore, source).id;
        let error = &errors[0];
        require(
            error.span.source == expected_source
                && error.span.start == start
                && error.span.end == end
                && error.message.ends_with(detail)
                && error
                    .message
                    .contains("Derived/validation and provider contract adapters are unsupported"),
            "Rule diagnostic lost original source, offsets, or lowerer detail",
        )?;
        require(
            package.save(&model).map_err(|error| error.to_string())? == before,
            "Failed rule activation changed an earlier package/model",
        )?;
        let after = serde_json::to_value(
            package
                .evaluate_model_rules(&model)
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        require(
            after == prior,
            "Rule evaluation failed to recover after rejected source",
        )?;
    }
    Ok(())
}
