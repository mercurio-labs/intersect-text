use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
use intersect_text::package::BundleCompilation;
use intersect_web::dispatch;
use serde_json::{Value, json};
fn call(value: Value) -> Result<Value, String> {
    dispatch(&value.to_string())
}
fn compilation(name: &str) -> BundleCompilation {
    let (structure, grammar) = match name {
        "machine" => (
            include_str!("../../../fixtures/xtext/model-to-text/machine.rcore"),
            include_str!("../../../fixtures/xtext/model-to-text/machine.xtext"),
        ),
        "paint" => (
            include_str!("../../../fixtures/xtext/model-to-text/paint.rcore"),
            include_str!("../../../fixtures/xtext/model-to-text/paint.xtext"),
        ),
        _ => (
            include_str!("../../../fixtures/xtext/model-to-text/expressions.rcore"),
            include_str!("../../../fixtures/xtext/model-to-text/expressions.xtext"),
        ),
    };
    let model = DefinitionSource::new("memory:general.rcore", SourceKind::Rcore, structure);
    let mut text = DefinitionSource::new("memory:general.xtext", SourceKind::Xtext, grammar);
    let mut common = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        include_str!(
            "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
        ),
    );
    common.dependencies.push(model.id.clone());
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
fn start(name: &str, text: &str) -> Value {
    call(json!({"method":"modelText/start","compilation":compilation(name),"sources":[{"uri":"example.test","text":text}]})).unwrap()
}
fn named(state: &Value, name: &str) -> String {
    state["evaluation"]["model"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["values"]["name"] == name)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn apply(state: &Value, action: Value) -> Value {
    let proposal =
        call(json!({"method":"modelText/prepare","context":state["context"],"action":action}))
            .unwrap();
    call(json!({"method":"modelText/commit","context":state["context"],"token":proposal["token"]}))
        .unwrap()
}
#[test]
fn generic_host_preserves_renames_and_structural_transactions_without_language_actions() {
    let state = start(
        "machine",
        "// keep\nmachine M { state start label 'Start'; edge step start -> end; state end final; }\n",
    );
    let id = named(&state, "start");
    let changed = apply(
        &state,
        json!({"kind":"transaction","changes":[[id,"name","begin"]]}),
    );
    assert_eq!(named(&changed, "begin"), id);
    assert!(
        changed["sources"][0]["text"]
            .as_str()
            .unwrap()
            .contains("edge step begin -> end")
    );
    let root = named(&changed, "M");
    let added = apply(
        &changed,
        json!({"kind":"transaction","creates":[{"id":"region","class":"Region","resource":"example.test"},{"id":"nested","class":"State","resource":"example.test"}],"changes":[["region","name","R"],["nested","name","nested"],["region","states",["nested"]],[root,"regions",["region"]]]}),
    );
    assert!(
        added["sources"][0]["text"]
            .as_str()
            .unwrap()
            .contains("region R")
    );
    let removed = apply(&added, json!({"kind":"transaction","deletes":["region"]}));
    super::assert_store_abi(
        &removed,
        "ea6babe0712ae9bc22534011a92a8df558a8e7e64ebfcb75d323ea2903b22087",
    );
    assert!(
        !removed["evaluation"]["model"]["elements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == "nested")
    );
    assert!(
        removed["sources"][0]["text"]
            .as_str()
            .unwrap()
            .contains("// keep")
    );
    let undone = apply(&removed, json!({"kind":"undo"}));
    assert_eq!(undone["sources"], added["sources"]);
}
#[test]
fn generic_host_switches_definitions_and_generates_verified_canonical_expression_text() {
    let state = start("expressions", "calc 01 /* keep */ + (2 + 3) // final\n");
    let number = state["evaluation"]["model"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["values"]["value"] == 2)
        .unwrap()["id"]
        .clone();
    let changed = apply(
        &state,
        json!({"kind":"transaction","changes":[[number,"value",20]]}),
    );
    assert_eq!(
        changed["sources"][0]["text"],
        "calc 01 /* keep */ + (20 + 3) // final\n"
    );
    let generated =
        call(json!({"method":"modelText/serialize","context":changed["context"]})).unwrap();
    assert!(
        generated["sources"][0]["text"]
            .as_str()
            .unwrap()
            .contains("20")
    );
    let observed = call(json!({"method":"modelText/inspect"})).unwrap();
    assert_eq!(observed["sources"], changed["sources"]);
    assert!(
        call(json!({"method":"modelText/commit","context":state["context"],"token":"1"})).is_err()
    );
}

#[test]
fn installed_conversion_catalog_survives_edits_generation_reload_and_history() {
    use intersect_model::definitions::resources::ResourceSet;
    let inputs = compilation("paint");
    let datatype = ResourceSet::new(&inputs.bundle)
        .unwrap()
        .resolve(&inputs.ecore, "#//RGB")
        .unwrap();
    let contract = json!({"datatype":datatype,"provider":{"name":"example.rgb","version":"1"},"serializable":true});
    let text = "paint P #123abc; // authored\r\n";
    let state=call(json!({"method":"modelText/start","compilation":inputs,"conversions":[contract],"sources":[{"uri":"example.test","text":text}]})).unwrap();
    let object = &state["evaluation"]["model"]["elements"][0];
    let mut value = object["values"]["color"].clone();
    value["value"] = json!({"red":171,"green":205,"blue":239});
    let changed = apply(
        &state,
        json!({"kind":"transaction","changes":[[object["id"],"color",value]]}),
    );
    assert_eq!(
        changed["sources"][0]["text"],
        text.replace("#123abc", "#ABCDEF")
    );
    let generated =
        call(json!({"method":"modelText/serialize","context":changed["context"]})).unwrap();
    assert!(
        generated["sources"][0]["text"]
            .as_str()
            .unwrap()
            .contains("#ABCDEF")
    );
    let restored = apply(&changed, json!({"kind":"undo"}));
    assert_eq!(restored["sources"], state["sources"]);
    let loaded=call(json!({"method":"modelText/load","context":restored["context"],"sources":[{"uri":"example.test","text":"paint P #abcdef;"}]})).unwrap();
    assert!(call(json!({"method":"modelText/serialize","context":loaded["context"]})).is_ok());
    let mut unknown = contract.clone();
    unknown["provider"]["name"] = json!("unknown");
    assert!(call(json!({"method":"modelText/start","compilation":compilation("paint"),"conversions":[unknown],"sources":loaded["sources"]})).is_err());
}
