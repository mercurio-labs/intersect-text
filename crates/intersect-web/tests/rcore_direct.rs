use intersect_web::dispatch;
use serde_json::json;

#[test]
fn explicit_migration_preserves_originals_before_native_rcore_compilation() {
    let original = r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" xmlns:m="http://www.omg.org/XMI" name="migration" nsURI="urn:migration"><eClassifiers x:type="e:EClass" name="Item" m:id="item-id"/></e:EPackage>"#;
    let migrated = dispatch(&json!({"method":"ecore/migrate", "uri":"memory:input.ecore", "target_uri":"memory:input.rcore", "text":original}).to_string()).unwrap();
    assert_eq!(migrated["ok"], true, "{migrated}");
    assert_eq!(migrated["report"]["original"]["text"], original);
    let text = migrated["report"]["converted"]["text"].as_str().unwrap();
    assert!(text.contains("identity \"item-id\""));
    let compiled = dispatch(&json!({"method":"compile_rcore", "rcore":text}).to_string()).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let sources = compiled["package"]["compilation"]["bundle"]["sources"]
        .as_array()
        .unwrap();
    assert!(sources.iter().all(|s| s["kind"] == "rcore"));
}

#[test]
fn toolkit_compiles_and_replays_authored_structure_without_generated_ecore() {
    let text = "rcore 1; package test version \"1\"; metamodel \"urn:test\" prefix t { class Item { attr name: String [0,1]; } }";
    let compiled = dispatch(&json!({"method":"compile_rcore", "rcore":text}).to_string()).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    assert!(package["grammar"].is_null());
    assert!(package["compilation"]["xtext"].is_null());
    let sources = package["compilation"]["bundle"]["sources"]
        .as_array()
        .unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["kind"], "rcore");
    assert_eq!(sources[0]["text"], text);
    let parsed = serde_json::from_value(package.clone()).unwrap();
    let replay = intersect_text::compiler::recompile(&parsed).unwrap();
    assert_eq!(serde_json::to_value(replay).unwrap(), *package);
}

#[test]
fn toolkit_text_and_operation_inputs_keep_authored_declaration_identities() {
    let text = include_str!("../../../fixtures/rcore/web/specialization-project.rcore");
    let inspected = dispatch(
        &json!({"method":"rcore/inspectStructure", "uri":"memory:model.rcore", "text":text})
            .to_string(),
    )
    .unwrap();
    assert_eq!(inspected["ok"], true, "{inspected}");
    assert_eq!(
        inspected["operations"][0]["declaration"]["source"],
        inspected["source"]["id"]
    );
    let compiled = dispatch(
        &json!({"method":"compile_rcore", "rcore":text,
        "xtext":include_str!("../../../fixtures/rcore/web/specialization.xtext")})
        .to_string(),
    )
    .unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let sources = compiled["package"]["compilation"]["bundle"]["sources"]
        .as_array()
        .unwrap();
    assert_eq!(sources.len(), 2);
    assert!(sources.iter().all(|source| source["kind"] != "ecore"));
    let evaluated = dispatch(&json!({"method":"evaluate", "package":compiled["package"],
        "sources":[{"uri":"memory:types.demo", "text":include_str!("../../../fixtures/rcore/web/types.demo")},
                   {"uri":"memory:relations.demo", "text":include_str!("../../../fixtures/rcore/web/relations.demo")}] }).to_string()).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]), "{evaluated}");
}
