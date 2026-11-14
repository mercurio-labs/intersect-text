//! Small public-API admission and execution controls, shared by native and WASM.
use super::*;
use serde_json::json;

fn compilation(structure: &str, grammar: &str) -> BundleCompilation {
    let model = DefinitionSource::new("memory:profiles.rcore", SourceKind::Rcore, structure);
    let mut common = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        TERMINALS,
    );
    common.dependencies.push(model.id.clone());
    let mut text = DefinitionSource::new("memory:profiles.xtext", SourceKind::Xtext, grammar);
    text.dependencies = vec![model.id.clone(), common.id.clone()];
    BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:general-text".into(),
        xtext: Some(text.id.clone()),
        rcore_behavior: model.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, text, common],
            namespaces: vec![],
        },
    }
}

fn analyze(package: &Package, text: &str) -> intersect_text::Evaluation {
    intersect_text::evaluate(
        package,
        &[Source {
            uri: "memory:profile.input".into(),
            text: text.into(),
        }],
    )
}

pub(super) fn verify() -> Result<(), String> {
    require(
        Profile::default() == Profile::WorkbenchV1,
        "Default profile changed",
    )?;
    for label in std::iter::once("intersect.workbench.v1".to_owned())
        .chain((1..=82).map(|n| format!("intersect.xtext.native.v{n}")))
    {
        let profile: Profile = serde_json::from_value(json!(label)).map_err(diagnostics)?;
        require(
            serde_json::to_value(profile).map_err(diagnostics)? == json!(label),
            "Retained profile identity changed",
        )?;
    }
    for label in [
        "intersect.xtext.native.v0",
        "intersect.xtext.native.v83",
        "NativeV82",
        "",
    ] {
        require(
            serde_json::from_value::<Profile>(json!(label)).is_err(),
            "Unknown profile silently accepted",
        )?;
    }
    let structure = r#"rcore 1; package example version "1"; metamodel "urn:general-text" prefix m {
        class Item { attr name: String [1,1]; attr ready: Bool [0,1]; }
    }"#;
    let simple = r#"grammar independent.Items with org.eclipse.xtext.common.Terminals
        import "urn:general-text" as m
        Item returns m::Item: 'item' name=ID ';';"#;
    let default =
        intersect_text::compiler::compile_rcore_project(structure, simple).map_err(diagnostics)?;
    require(
        default
            .grammar
            .as_ref()
            .is_some_and(|g| g.profile == Profile::WorkbenchV1),
        "Convenience compiler changed profile",
    )?;
    for package in [
        default,
        intersect_text::ordinary::compile(compilation(structure, simple), Profile::NativeV80)
            .map_err(diagnostics)?,
    ] {
        let valid = analyze(&package, "item first;");
        require(
            valid.diagnostics.is_empty()
                && valid
                    .model
                    .elements
                    .first()
                    .and_then(|e| e.values.get("name"))
                    == Some(&json!("first")),
            "Basic profile parse differs",
        )?;
        require(
            !analyze(&package, "item ;").diagnostics.is_empty(),
            "Malformed document accepted",
        )?;
    }
    let scalar = r#"rcore 1; package scalar version "1"; metamodel "urn:general-text" prefix m { class Root { attr value: ELong [1,1]; } }"#;
    let scalar_grammar = r#"grammar independent.Scalar hidden()
        import "urn:general-text" as m
        import "http://www.eclipse.org/emf/2002/Ecore" as ecore
        Root returns m::Root: value=VALUE;
        terminal VALUE returns ecore::ELong: .+;"#;
    require(
        intersect_text::ordinary::compile(compilation(scalar, scalar_grammar), Profile::NativeV80)
            .is_err(),
        "V80 silently admitted V81 scalar returns",
    )?;
    let package =
        intersect_text::ordinary::compile(compilation(scalar, scalar_grammar), Profile::NativeV81)
            .map_err(diagnostics)?;
    // ELong uses tagged decimal transport so JavaScript cannot round large values.
    let parsed = analyze(&package, "9223372036854775807");
    require(
        parsed.diagnostics.is_empty()
            && parsed
                .model
                .elements
                .first()
                .and_then(|e| e.values.get("value"))
                == Some(&json!({"kind": "int64", "value": "9223372036854775807"})),
        "V81 scalar value differs",
    )?;
    require(
        !analyze(&package, "not-a-number").diagnostics.is_empty(),
        "Invalid scalar accepted",
    )?;
    require(
        !analyze(&package, "9223372036854775808")
            .diagnostics
            .is_empty(),
        "Out-of-range scalar accepted",
    )?;
    let unordered = r#"grammar independent.Unordered with org.eclipse.xtext.common.Terminals
        import "urn:general-text" as m
        Item returns m::Item: ready?='ready'? & name=ID;"#;
    require(
        intersect_text::ordinary::compile(compilation(structure, unordered), Profile::NativeV81)
            .is_err(),
        "V81 silently admitted unordered groups",
    )?;
    let package =
        intersect_text::ordinary::compile(compilation(structure, unordered), Profile::NativeV82)
            .map_err(diagnostics)?;
    for input in ["ready first", "first ready"] {
        let parsed = analyze(&package, input);
        require(
            parsed.diagnostics.is_empty()
                && parsed.model.elements.first().is_some_and(|e| {
                    e.values.get("name") == Some(&json!("first"))
                        && e.values.get("ready") == Some(&json!(true))
                }),
            "V82 unordered result differs",
        )?;
    }
    require(
        !analyze(&package, "ready ready first")
            .diagnostics
            .is_empty(),
        "Repeated unordered member accepted",
    )?;
    Ok(())
}
