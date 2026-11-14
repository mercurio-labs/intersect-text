use super::*;
use intersect_text::{serialization::CanonicalSerializer, source_edit::SourceEditSession};
use serde_json::json;

pub(super) fn verify() -> Result<(), String> {
    let identity = intersect_text::compiler_identity();
    require(
        identity.len() == 64
            && identity != intersect_text::SOURCE_IDENTITY
            && identity != intersect_model::SOURCE_IDENTITY,
        "Missing installed compiler composition",
    )?;
    require(
        intersect_text::compiler_cache_identity(Profile::NativeV82, &["provider@1"])?
            != intersect_text::compiler_cache_identity(Profile::NativeV82, &["provider@2"])?,
        "Provider implementation change reused a compiler cache key",
    )?;
    let package = intersect_text::ordinary::compile(inputs(GRAMMAR), Profile::NativeV82)
        .map_err(diagnostics)?;
    // The public DTO is intentionally mutable and forward-tolerant. Replay, not
    // Deserialize, is the authority boundary; strict source recipes stay strict.
    let bytes = serde_json::to_string(&package).map_err(|e| e.to_string())?;
    let wire: serde_json::Value = serde_json::from_str(&bytes).map_err(|e| e.to_string())?;
    require(wire["version"] == 2, "Text package format changed")?;
    let mut extended = wire.clone();
    extended["future_metadata"] = json!({"note":"not executable"});
    let mut restored: Package = serde_json::from_value(extended).map_err(|e| e.to_string())?;
    restored.schema.classes.clear();
    let rebuilt = intersect_text::recompile(&restored).map_err(diagnostics)?;
    require(
        rebuilt.schema == package.schema,
        "Cached schema supplied replay authority",
    )?;
    for version in [0, 1, 3, u32::MAX] {
        let mut future = wire.clone();
        future["version"] = version.into();
        let future: Package = serde_json::from_value(future).map_err(|e| e.to_string())?;
        require(
            intersect_text::recompile(&future).is_err(),
            "Unsupported Text package version admitted",
        )?;
    }
    let mut unknown_recipe = wire.clone();
    unknown_recipe["compilation"]["executable"] = json!({});
    require(
        serde_json::from_value::<Package>(unknown_recipe).is_err(),
        "Unknown source recipe field admitted",
    )?;
    let mut unknown_profile = wire;
    unknown_profile["grammar"]["profile"] = json!("future-profile");
    require(
        serde_json::from_value::<Package>(unknown_profile).is_err(),
        "Unknown grammar profile admitted",
    )?;
    let duplicate = format!("{{\"version\":2,{}", &bytes[1..]);
    require(
        serde_json::from_str::<Package>(&duplicate).is_err(),
        "Duplicate Text package version admitted",
    )?;
    let recipe = package
        .compilation
        .as_ref()
        .ok_or("Lost ordinary source recipe")?;
    require(
        recipe.bundle.sources[0].text == STRUCTURE,
        "Changed original Rcore bytes",
    )?;
    require(
        recipe.bundle.sources[1].text == GRAMMAR,
        "Changed original grammar bytes",
    )?;
    let sources = [
        Source { uri: "a.machine".into(), text: "// retained 😀\r\nmachine M { state start label 'Start'; edge next start -> end; }\r\n".into() },
        Source { uri: "b.machine".into(), text: "machine N { state end final; }".into() },
    ];
    let result = intersect_text::evaluate(&package, &sources);
    require(
        result.diagnostics.is_empty(),
        &format!("Ordinary diagnostics: {:?}", result.diagnostics),
    )?;
    require(
        result.model.elements.len() == 5 && result.references.len() == 2,
        "Cross-resource graph/reference count differs",
    )?;
    result.model.validate(&package.schema)?;
    let mut invalid = sources.clone();
    invalid[0].text = invalid[0].text.replace("-> end", "-> missing");
    require(
        !intersect_text::evaluate(&package, &invalid)
            .diagnostics
            .is_empty(),
        "Unresolved cross-resource reference accepted",
    )?;
    invalid[0].text = "machine M { broken syntax }".into();
    require(
        !intersect_text::evaluate(&package, &invalid)
            .diagnostics
            .is_empty(),
        "Invalid syntax accepted",
    )?;
    require(
        intersect_text::evaluate(&package, &sources).model == result.model,
        "Failed evaluation poisoned recovery",
    )?;
    let replay = intersect_text::recompile(&package).map_err(diagnostics)?;
    require(
        replay.schema == package.schema,
        "Ordinary replay changed the model schema",
    )?;

    let mut session =
        SourceEditSession::new(prepare(&package)?, &sources, EvaluationLimits::workbench())?;
    let original = session.checkpoint();
    let id = session
        .model()
        .elements
        .iter()
        .find(|element| element.values.get("name") == Some(&json!("start")))
        .ok_or("Missing authored state")?
        .id
        .clone();
    let mut candidate = session.model().clone();
    candidate.edit(
        &package.schema,
        &[(id.clone(), "name".into(), json!("begin"))],
    )?;
    let first = session.plan(&candidate, Default::default())?;
    let stale = session.plan(&candidate, Default::default())?;
    session.adopt(first)?;
    require(
        session.sources()[0].text == sources[0].text.replace("start", "begin"),
        "Rename did not preserve comments, quotes, CRLF and reference spelling",
    )?;
    require(
        session.sources()[1].text == sources[1].text,
        "Unchanged resource rewritten",
    )?;
    require(session.adopt(stale).is_err(), "Stale proposal published")?;
    let renamed = session.checkpoint();
    let undo = session.plan_restore(&original)?;
    session.adopt(undo)?;
    require(
        session.sources().len() == sources.len()
            && session
                .sources()
                .iter()
                .zip(&sources)
                .all(|(actual, expected)| {
                    actual.uri == expected.uri && actual.text == expected.text
                }),
        "Undo lost exact original source bytes",
    )?;
    let redo = session.plan_restore(&renamed)?;
    session.adopt(redo)?;
    require(
        session
            .model()
            .elements
            .iter()
            .any(|e| e.id == id && e.values.get("name") == Some(&json!("begin"))),
        "Redo lost stable model identity",
    )?;
    require(
        session
            .plan(session.model(), Default::default())?
            .patches()
            .is_empty(),
        "No-op source plan changed bytes",
    )?;
    let generated = CanonicalSerializer::new(
        prepare(&package)?,
        EvaluationLimits::workbench(),
        Default::default(),
    )
    .serialize(session.model())?;
    require(
        generated.sources.len() == 2 && generated.identities.len() == 5,
        "Canonical generation lost a resource or object",
    )?;
    let reparsed = intersect_text::evaluate(&package, &generated.sources);
    require(
        reparsed.diagnostics.is_empty()
            && reparsed.model.elements.len() == 5
            && reparsed.references.len() == 2,
        "Generated text failed independent parse/link/validation",
    )?;
    Ok(())
}
