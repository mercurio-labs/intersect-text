use intersect_web::dispatch;
use serde_json::{Value, json};
fn call(value: Value) -> Result<Value, String> {
    dispatch(&value.to_string())
}
fn id(state: &Value, name: &str) -> String {
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
    let plan = call(json!({"method":"library/prepare","context":state["context"],"action":action}))
        .unwrap();
    call(json!({"method":"library/commit","context":state["context"],"token":plan["token"]}))
        .unwrap()
}
#[test]
fn host_publishes_complete_pairs_and_undo_redo_keep_exact_sources_and_ids() {
    let start = call(json!({"method":"library/start"})).unwrap();
    let dune = id(&start, "dune");
    let renamed = apply(
        &start,
        json!({"kind":"set","id":dune,"feature":"name","value":"arrakis"}),
    );
    assert!(
        renamed["sources"][1]["text"]
            .as_str()
            .unwrap()
            .contains("copy arrakis")
    );
    assert_eq!(id(&renamed, "arrakis"), dune);
    let undone = apply(&renamed, json!({"kind":"undo"}));
    assert_eq!(undone["sources"], start["sources"]);
    assert_eq!(id(&undone, "dune"), dune);
    let redone = apply(&undone, json!({"kind":"redo"}));
    assert_eq!(redone["sources"], renamed["sources"]);
    assert_ne!(redone["context"], renamed["context"]);
}
#[test]
fn host_rejects_stale_proposals_and_invalid_source_without_replacing_accepted_pair() {
    let start = call(json!({"method":"library/start"})).unwrap();
    let dune = id(&start, "dune");
    let p=call(json!({"method":"library/prepare","context":start["context"],"action":{"kind":"set","id":dune,"feature":"title","value":"New"}})).unwrap();
    let dirty = call(json!({"method":"library/invalidate","context":start["context"]})).unwrap();
    assert!(
        call(json!({"method":"library/commit","context":dirty["context"],"token":p["token"]}))
            .is_err()
    );
    let mut sources = start["sources"].clone();
    sources[0]["text"] = json!("invalid");
    assert!(
        call(json!({"method":"library/load","context":dirty["context"],"sources":sources}))
            .is_err()
    );
    let current = call(json!({"method":"library/inspect"})).unwrap();
    assert_eq!(current["sources"], start["sources"]);
    assert_eq!(current["evaluation"], start["evaluation"]);
    assert_eq!(current["dirty"], true);
    assert!(call(json!({"method":"library/prepare","context":current["context"],"action":{"kind":"delete","id":dune}})).is_err());
}
#[test]
fn host_create_move_delete_undo_uses_one_shared_model() {
    let start = call(json!({"method":"library/start"})).unwrap();
    let added = apply(
        &start,
        json!({"kind":"add_loan","name":"second","copy":id(&start,"earthsea"),"member":id(&start,"ada")}),
    );
    let second = id(&added, "second");
    let first = id(&added, "first");
    let moved = apply(&added, json!({"kind":"move","id":second,"position":0}));
    let text = moved["sources"][1]["text"].as_str().unwrap();
    assert!(text.find("loan second").unwrap() < text.find("loan first").unwrap());
    let removed = apply(&moved, json!({"kind":"delete","id":first}));
    assert!(
        removed["sources"][1]["text"]
            .as_str()
            .unwrap()
            .contains("/* loan note */")
    );
    let restored = apply(&removed, json!({"kind":"undo"}));
    assert_eq!(restored["sources"], moved["sources"]);
    assert_eq!(id(&restored, "first"), first);
}
