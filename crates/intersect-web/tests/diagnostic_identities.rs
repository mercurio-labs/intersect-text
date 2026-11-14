use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::SourceLimits;
use serde_json::{Value, json};
const ECORE: &str = "import \"http://www.eclipse.org/emf/2002/Ecore\" as ecore";
fn source(uri: &str, text: &str) -> DefinitionSource {
    DefinitionSource::new(uri, SourceKind::Xtext, text)
}
fn request(sources: Vec<DefinitionSource>, entry: &DefinitionSource) -> Value {
    json!({"version":1,"bundle":DefinitionBundle{version:1,sources,namespaces:vec![]},"grammar":entry.id})
}
fn invoke(method: &str, inputs: &Value) -> Value {
    intersect_web::dispatch(&json!({"method":method,"inference":inputs}).to_string()).unwrap()
}
fn failure(inputs: &Value) -> Value {
    let old = invoke("compile_inferred", inputs);
    let detailed = invoke("compile_inferred_with_origins", inputs);
    assert_eq!(old["ok"], false, "{old}");
    assert_eq!(old["diagnostics"], detailed["diagnostics"]);
    assert!(old.get("diagnostic_origins").is_none());
    assert_eq!(
        detailed["diagnostics"].as_array().unwrap().len(),
        detailed["diagnostic_origins"].as_array().unwrap().len()
    );
    if let Ok(directory) = std::env::var("INTERSECT_DIAGNOSTIC_OBSERVATIONS") {
        let path = std::path::Path::new(&directory);
        std::fs::create_dir_all(path).unwrap();
        let name = inputs["grammar"]["sha256"].as_str().unwrap();
        std::fs::write(
            path.join(format!("{name}.json")),
            serde_json::to_vec_pretty(
                &json!({"inference":inputs,"legacy":old,"detailed":detailed}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    detailed
}
fn exact(result: &Value, source: &DefinitionSource) {
    assert_eq!(
        result["diagnostic_origins"][0]["kind"], "source",
        "{result}"
    );
    assert_eq!(
        result["diagnostic_origins"][0]["source"],
        json!(source.id),
        "{result}"
    );
}
#[test]
fn selected_syntax_revision_is_owned_natively_and_forged_host_origins_are_ignored() {
    let good = source(
        "memory:shared.xtext",
        &format!(
            "grammar diag.Good\ngenerate model \"urn:diag:good\"\n{ECORE}\nModel: name=ID;\nterminal ID: ('a'..'z')+;"
        ),
    );
    let bad = source(&good.id.uri, "grammar broken ???");
    let inputs = request(vec![good.clone(), bad.clone()], &bad);
    exact(&failure(&inputs), &bad);
    let forged=intersect_web::dispatch(&json!({"method":"compile_inferred_with_origins","inference":inputs,"diagnostic_origins":[{"kind":"source","source":good.id}]}).to_string()).unwrap();
    exact(&forged, &bad);
}
#[test]
fn parent_syntax_revision_is_not_reconstructed_from_its_shared_uri() {
    let good = source(
        "memory:parent.xtext",
        &format!(
            "grammar diag.Parent\ngenerate parent \"urn:diag:parent\"\n{ECORE}\nRoot: name=ID;\nterminal ID: ('a'..'z')+;"
        ),
    );
    let bad = source(&good.id.uri, &format!("{}\n???", good.text));
    let mut child = source(
        "memory:child.xtext",
        &format!(
            "grammar diag.Child with diag.Parent\ngenerate child \"urn:diag:child\"\n{ECORE}\nModel: Root;"
        ),
    );
    child.dependencies.push(bad.id.clone());
    exact(
        &failure(&request(vec![good, bad.clone(), child.clone()], &child)),
        &bad,
    );
}
#[test]
fn semantic_type_error_retains_owned_authored_span() {
    let bad = source(
        "memory:type.xtext",
        &format!(
            "grammar diag.Type\ngenerate model \"urn:diag:type\"\n{ECORE}\nModel: missing=Unknown;"
        ),
    );
    let result = failure(&request(vec![bad.clone()], &bad));
    exact(&result, &bad);
    let span = &result["diagnostic_origins"][0]["span"];
    let text =
        &bad.text[span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize];
    assert!(text.contains("Unknown"), "{text}: {result}");
}
#[test]
fn inherited_fragment_conflict_retains_parent_identity_despite_legacy_child_uri() {
    let parent = source(
        "memory:fragment-parent.xtext",
        &format!(
            "grammar diag.Parent\ngenerate parent \"urn:diag:parent\"\n{ECORE}\nRoot: Shared;\nfragment Shared*: value=ID;\nterminal ID: ('a'..'z')+;"
        ),
    );
    let mut child = source(
        "memory:fragment-child.xtext",
        &format!(
            "grammar diag.Child with diag.Parent\ngenerate child \"urn:diag:child\"\n{ECORE}\nModel: value=INT Shared;\nterminal INT returns ecore::EInt: ('0'..'9')+;"
        ),
    );
    child.dependencies.push(parent.id.clone());
    let result = failure(&request(vec![parent.clone(), child.clone()], &child));
    exact(&result, &parent);
    assert_eq!(result["diagnostics"][0]["source"], child.id.uri);
    let span = &result["diagnostic_origins"][0]["span"];
    assert!(
        parent.text
            [span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize]
            .contains("value=ID")
    );
}
#[test]
fn executable_compiler_error_retains_input_after_successful_inference() {
    let bad = source(
        "memory:executable.xtext",
        &format!(
            "grammar diag.Executable\ngenerate model \"urn:diag:executable\"\n{ECORE}\nModel: name=ID;\nterminal ID: ('a'..'z')+;\nterminal EMPTY: '';"
        ),
    );
    let inputs = request(vec![bad.clone()], &bad);
    let bundle: DefinitionBundle = serde_json::from_value(inputs["bundle"].clone()).unwrap();
    assert!(
        intersect_text::inference::infer_metamodels(&bundle, &bad.id, SourceLimits::default())
            .is_ok()
    );
    let result = failure(&inputs);
    exact(&result, &bad);
    assert_eq!(result["diagnostics"][0]["start"], 0);
    assert_eq!(result["diagnostics"][0]["end"], 1);
    let span = &result["diagnostic_origins"][0]["span"];
    assert!(
        bad.text[span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize]
            .contains("terminal EMPTY")
    );
}
#[test]
fn unsupported_request_version_identifies_context_without_fabricating_a_culprit() {
    let source = source("memory:context.xtext", "grammar diag.Context");
    let mut inputs = request(vec![source.clone()], &source);
    inputs["version"] = json!(99);
    let result = failure(&inputs);
    assert_eq!(
        result["diagnostic_origins"][0],
        json!({"kind":"context","source":source.id})
    );
}
#[test]
fn generated_cycle_failure_retains_authored_correspondence() {
    let source = source(
        "memory:cycle.xtext",
        &format!(
            "grammar diag.Cycle\ngenerate model \"urn:diag:cycle\"\n{ECORE}\nModel: {{Model}} A;\nA: {{A}} B;\nB: {{B}} A;"
        ),
    );
    let result = failure(&request(vec![source.clone()], &source));
    assert_eq!(
        result["diagnostic_origins"][0]["kind"], "generated",
        "{result}"
    );
    let originals = result["diagnostic_origins"][0]["originals"]
        .as_array()
        .unwrap();
    assert!(!originals.is_empty(), "{result}");
    assert!(
        originals.iter().all(|s| s["source"] == json!(source.id)),
        "{result}"
    );
}

#[test]
fn generated_binding_failure_identifies_request_context_without_an_authored_culprit() {
    use intersect_text::grammar::GeneratedRuleTypes;
    use intersect_text::grammar::Profile;
    use intersect_text::grammar::compile_generated_sources;
    let source = source(
        "memory:valid.xtext",
        &format!(
            "grammar diag.Valid\ngenerate model \"urn:diag:valid\"\n{ECORE}\nModel: name=ID; terminal ID: ('a'..'z')+;"
        ),
    );
    let input = request(vec![source.clone()], &source);
    let bundle: DefinitionBundle = serde_json::from_value(input["bundle"].clone()).unwrap();
    let mut definitions =
        intersect_text::inference::infer_metamodels(&bundle, &source.id, SourceLimits::default())
            .unwrap();
    let missing = DefinitionSource::new(
        "memory:missing.xtext",
        SourceKind::Xtext,
        "missing binding source",
    );
    let error = compile_generated_sources(
        &definitions.bundle,
        &source.id,
        &mut definitions.schema,
        Profile::NativeV56,
        SourceLimits::default(),
        &[GeneratedRuleTypes {
            source: missing.id,
            types: std::collections::BTreeMap::new(),
        }],
    )
    .unwrap_err();
    assert_eq!(
        json!(error.origin),
        json!({"kind":"context","source":source.id})
    );
    assert_eq!(error.source, source.id.uri);
}

#[test]
fn inherited_terminal_failure_retains_the_exact_producer_revision_and_owned_span() {
    let parent = source(
        "memory:tokens.xtext",
        &format!("grammar diag.Tokens\n{ECORE}\nterminal ID: ('a'..'z')+; terminal EMPTY: '';"),
    );
    let other = source(
        &parent.id.uri,
        &format!("{}\n// another retained revision", parent.text),
    );
    let mut child = source(
        "memory:terminal-child.xtext",
        "grammar diag.Child with diag.Tokens\ngenerate child \"urn:diag:terminal-child\"\nModel: name=ID;",
    );
    child.dependencies.push(parent.id.clone());
    let inputs = request(vec![other, parent.clone(), child.clone()], &child);
    let bundle: DefinitionBundle = serde_json::from_value(inputs["bundle"].clone()).unwrap();
    assert!(
        intersect_text::inference::infer_metamodels(&bundle, &child.id, SourceLimits::default())
            .is_ok()
    );
    let result = failure(&inputs);
    exact(&result, &parent);
    let span = &result["diagnostic_origins"][0]["span"];
    assert!(
        parent.text
            [span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize]
            .contains("terminal EMPTY")
    );
}

#[test]
fn legacy_archive_failure_does_not_serialize_native_metadata_through_debug_messages() {
    let source = source("memory:archive-invalid.xtext", "grammar broken ???");
    let input = request(vec![source.clone()], &source);
    let request = json!({"method":"package/export","inference":input,"metadata":{"package_id":"diag.origin","release":"1","display_name":"Diagnostic","description":""}});
    let error = intersect_web::dispatch(&request.to_string()).unwrap_err();
    assert_eq!(
        error,
        r#"Cannot compile inferred archive sources: [Diagnostic { source: "memory:archive-invalid.xtext", start: 16, end: 17, message: "Expected syntax identifier" }]"#
    );
    if let Ok(directory) = std::env::var("INTERSECT_DIAGNOSTIC_OBSERVATIONS") {
        let path = std::path::Path::new(&directory)
            .parent()
            .unwrap()
            .join("diagnostic-identities-legacy-transport.json");
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&json!({"request":request,"error":error})).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn parameter_binding_errors_keep_exact_same_uri_revision_and_authored_spans() {
    let text = format!(
        "grammar diag.Parameters\ngenerate model \"urn:diag:parameters\"\n{ECORE}\nModel: child=Node<true>; Node<Flag>: <Flag> a=ID | <!Flag> b=ID;\nterminal ID: ('a'..'z')+;"
    );
    let good = source("memory:parameters.xtext", &text);
    for (bad_text, expected_span, message) in [
        (
            text.replace("Node<true>", "Node"),
            "Node",
            "Expected 1 rule arguments",
        ),
        (
            text.replace("<Flag> a=ID", "<Unknown> a=ID"),
            "Unknown",
            "Unknown rule parameter Unknown",
        ),
    ] {
        let bad = source(&good.id.uri, &bad_text);
        let result = failure(&request(vec![good.clone(), bad.clone()], &bad));
        exact(&result, &bad);
        assert!(
            result["diagnostics"][0]["message"]
                .as_str()
                .unwrap()
                .contains(message),
            "{result}"
        );
        assert_eq!(result["diagnostics"][0].as_object().unwrap().len(), 4);
        let span = &result["diagnostic_origins"][0]["span"];
        assert_eq!(
            &bad.text
                [span["start"].as_u64().unwrap() as usize..span["end"].as_u64().unwrap() as usize],
            expected_span
        );
    }
    assert_eq!(
        invoke("compile_inferred", &request(vec![good.clone()], &good))["ok"],
        true
    );
}
