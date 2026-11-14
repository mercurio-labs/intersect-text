#[path = "support/mod.rs"]
mod support;
use intersect_web::dispatch;
use serde_json::{Value, json};

fn call(method: &str, mut args: Value) -> Result<Value, String> {
    args["method"] = json!(method);
    if args.get("revision").is_none() {
        args["revision"] = json!("test:1");
    }
    dispatch(&args.to_string())
}
fn compile_migrated_fixture(mut args: Value) -> Result<Value, String> {
    args["rcore"] = json!(support::migrate_text(args["ecore"].as_str().unwrap()));
    args.as_object_mut().unwrap().remove("ecore");
    call("compile_rcore", args)
}
fn fixture() -> (Value, Value) {
    let compiled = compile_migrated_fixture(json!({
        "ecore":include_str!("../../../fixtures/library/library.ecore"),
        "xtext":include_str!("../../../fixtures/library/library.xtext"),
        "rcore_behavior":include_str!("../../../fixtures/library/library-behavior.rcore")
    }))
    .expect("fixture compiles");
    let package = compiled["package"].clone();
    let result = call("evaluate", json!({"package":package,"sources":[
        {"uri":"catalog.library","text":include_str!("../../../fixtures/library/catalog.library")},
        {"uri":"loans.library","text":include_str!("../../../fixtures/library/loans.library")}
    ]})).expect("fixture evaluates");
    (package, result["model"].clone())
}
fn begin(package: &Value, model: &Value) {
    call("script/begin", json!({"package":package,"model":model})).expect("begin");
}
fn find(class: &str, name: &str) -> Value {
    call("script/find", json!({"class":class,"name":name})).expect("find")["id"].clone()
}

#[test]
fn loan_uses_native_derivation_validation_and_containment() {
    let (package, model) = fixture();
    begin(&package, &model);
    let copy = find("BookCopy", "earthsea");
    let member = find("Member", "ada");
    let library = find("Library", "Circulation");
    assert_eq!(
        call("script/get", json!({"id":copy,"feature":"available"})).expect("derived"),
        json!(true)
    );
    let loan = call(
        "script/create",
        json!({"class":"Loan","values":{"name":"second","copy":copy,"member":member}}),
    )
    .expect("create");
    let old = call("script/get", json!({"id":library,"feature":"loans"})).expect("loans");
    call(
        "script/set",
        json!({"id":library,"feature":"loans","value":[old[0]["id"],loan["id"]]}),
    )
    .expect("containment");
    assert_eq!(
        call("script/get", json!({"id":copy,"feature":"available"})).expect("derived"),
        json!(false)
    );
    assert_eq!(
        call("script/validate", json!({})).expect("validate"),
        json!([])
    );
    let committed = call("script/commit", json!({})).expect("commit");
    assert_eq!(
        committed["model"]["elements"]
            .as_array()
            .expect("elements")
            .len(),
        7
    );
    assert!(call("script/get", json!({"id":copy,"feature":"available"})).is_err());
    // The publication boundary recomputes the same native result from the model.
    let checked = call(
        "script/check",
        json!({"package":package,"model":committed["model"]}),
    )
    .expect("check");
    assert_eq!(checked["model"], committed["model"]);
    assert_eq!(checked["derived"], committed["derived"]);
    assert_eq!(committed["transaction"]["status"], "prepared");
    assert_eq!(
        committed["transaction"]["derived_changes"][0]["feature"],
        "available"
    );
    let published = call("script/publish", json!({"package":package,"before":model,"current":model,"model":committed["model"],"context":committed["transaction"]["context"],"generation":0})).expect("publish");
    assert_eq!(published["transaction"]["status"], "committed");
    assert_eq!(published["model"], committed["model"]);
    assert!(
        call(
            "script/finish",
            json!({"transaction":published["transaction"],"status":"committed"})
        )
        .is_err()
    );
    assert!(call("script/publish", json!({"package":package,"before":model,"current":model,"model":committed["model"],"context":committed["transaction"]["context"],"generation":1})).is_err());
    // A later transaction can construct another element without reusing a handle.
    begin(&package, &committed["model"]);
    call(
        "script/create",
        json!({"class":"Member","values":{"name":"grace"}}),
    )
    .expect("next creation");
    assert!(call("script/commit", json!({})).is_ok());
}

#[test]
fn errors_poison_transactions_and_invalid_models_cannot_publish() {
    let (package, model) = fixture();
    for (field, value, message) in [
        ("available", json!(false), "derived"),
        ("unknown", json!(1), "Unknown feature"),
    ] {
        begin(&package, &model);
        let id = find("BookCopy", "earthsea");
        assert!(
            call("script/set", json!({"id":id,"feature":field,"value":value}))
                .expect_err("reject")
                .contains(message)
        );
        assert!(call("script/commit", json!({})).is_err());
    }
    begin(&package, &model);
    let id = find("BookCopy", "earthsea");
    call(
        "script/set",
        json!({"id":id,"feature":"edition","value":"bad"}),
    )
    .expect("staged");
    assert!(
        call("script/commit", json!({}))
            .expect_err("wrong type")
            .contains("Invalid EInt")
    );
    begin(&package, &model);
    let copy = find("BookCopy", "dune");
    let member = find("Member", "ada");
    call(
        "script/create",
        json!({"class":"Loan","values":{"name":"duplicate","copy":copy,"member":member}}),
    )
    .expect("staged");
    assert!(
        !call("script/validate", json!({}))
            .expect("validation")
            .as_array()
            .expect("diagnostics")
            .is_empty()
    );
    assert!(
        call("script/commit", json!({}))
            .expect_err("duplicate loan")
            .contains("Semantic validation")
    );
    begin(&package, &model);
    assert!(
        call("script/all", json!({"class":"BookCopy","revision":"old"}))
            .expect_err("revision")
            .contains("Stale")
    );
    assert!(call("script/commit", json!({})).is_err());
    let mut foreign = model.clone();
    foreign["schema_revision"] = json!("foreign");
    assert!(call("script/check", json!({"package":package,"model":foreign})).is_err());
    // Rollback is snapshot discard; the input remains a valid six-element model.
    let original =
        call("script/check", json!({"package":package,"model":model})).expect("original");
    assert_eq!(
        original["model"]["elements"]
            .as_array()
            .expect("elements")
            .len(),
        6
    );
}

#[test]
fn package_setting_declarations_drive_shared_script_set_reset_and_reference_reads() {
    let ecore = include_str!("../../../fixtures/ecore-settings/settings.ecore");
    let xtext = include_str!("../../../fixtures/ecore-settings/settings.xtext");
    let rcore_behavior = include_str!("../../../fixtures/ecore-settings/settings-behavior.rcore");
    let compiled = compile_migrated_fixture(
        json!({"ecore":ecore,"xtext":xtext,"rcore_behavior":rcore_behavior}),
    )
    .unwrap();
    assert!(compiled["package"].is_object(), "{compiled}");
    let package = &compiled["package"];
    let evaluated = call("evaluate", json!({"package":package,"sources":[{"uri":"example.settings","text":"model Demo { item a item b }"}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    begin(package, model);
    let a = find("Item", "a");
    let b = find("Item", "b");
    assert_eq!(
        call("script/get", json!({"id":a,"feature":"aliasTitle"})).unwrap(),
        json!("seed")
    );
    call(
        "script/set",
        json!({"id":a,"feature":"aliasTitle","value":"seed"}),
    )
    .unwrap();
    assert_eq!(
        call("script/is_set", json!({"id":a,"feature":"aliasTitle"})).unwrap(),
        json!(true)
    );
    assert_eq!(
        call("script/is_set", json!({"id":a,"feature":"readonlyTitle"})).unwrap(),
        json!(false)
    );
    call(
        "script/set",
        json!({"id":a,"feature":"aliasTitle","value":"changed"}),
    )
    .unwrap();
    assert_eq!(
        call("script/get", json!({"id":a,"feature":"readonlyTitle"})).unwrap(),
        json!("changed")
    );
    assert_eq!(
        call("script/is_set", json!({"id":a,"feature":"title"})).unwrap(),
        json!(true)
    );
    call(
        "script/set",
        json!({"id":a,"feature":"aliasTags","value":["one","two"]}),
    )
    .unwrap();
    call(
        "script/set",
        json!({"id":a,"feature":"aliasPeer","value":b}),
    )
    .unwrap();
    assert_eq!(
        call("script/get", json!({"id":a,"feature":"aliasPeer"})).unwrap()["id"],
        b
    );
    for name in ["aliasTitle", "aliasTags", "aliasPeer"] {
        call("script/unset", json!({"id":a,"feature":name})).unwrap();
        assert_eq!(
            call("script/is_set", json!({"id":a,"feature":name})).unwrap(),
            json!(false)
        );
    }
    let committed = call("script/commit", json!({})).unwrap();
    let mut expected = model.clone();
    let record = &mut expected["resources"]["example.settings"];
    let current = &committed["model"]["resources"]["example.settings"];
    assert!(
        current["content_revision"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > record["content_revision"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
    );
    assert!(record["source"].is_object());
    record["content_revision"] = current["content_revision"].clone();
    record["source"] = Value::Null;
    assert_eq!(committed["model"], expected);
    for (feature, value) in [
        ("readonlyTitle", json!("forbidden")),
        ("aliasTags", json!(["duplicate", "duplicate"])),
        ("aliasPeer", json!("missing")),
    ] {
        begin(package, model);
        call(
            "script/set",
            json!({"id":a,"feature":"aliasTitle","value":"rollback"}),
        )
        .unwrap();
        assert!(
            call(
                "script/set",
                json!({"id":a,"feature":feature,"value":value})
            )
            .is_err()
        );
        assert!(call("script/commit", json!({})).is_err());
    }
    for changed in [
        ecore.replace(
            "name=\"title\" unsettable=\"true\"",
            "name=\"title\" changeable=\"false\"",
        ),
        ecore.replace(
            "name=\"aliasTags\" derived=\"true\" unsettable=\"true\" upperBound=\"2\"",
            "name=\"aliasTags\" derived=\"true\" unsettable=\"true\" upperBound=\"3\"",
        ),
    ] {
        let result = compile_migrated_fixture(
            json!({"ecore":changed,"xtext":xtext,"rcore_behavior":rcore_behavior}),
        )
        .unwrap();
        assert!(result["package"].is_null());
        assert!(!result["diagnostics"].as_array().unwrap().is_empty());
    }
}

#[test]
fn stored_projection_contracts_reject_mismatches_and_replay_ignores_tampered_ir() {
    let ecore = include_str!("../../../fixtures/ecore-settings/settings.ecore");
    let xtext = include_str!("../../../fixtures/ecore-settings/settings.xtext");
    let rcore_behavior = include_str!("../../../fixtures/ecore-settings/settings-behavior.rcore");
    for (from, to) in [
        (
            "name=\"title\" unsettable=\"true\"",
            "name=\"title\" unsettable=\"false\"",
        ),
        (
            "name=\"aliasTitle\" derived=\"true\"",
            "name=\"aliasTitle\" derived=\"true\" changeable=\"false\"",
        ),
        (
            "name=\"aliasTags\" derived=\"true\"",
            "name=\"aliasTags\" derived=\"true\" unique=\"false\"",
        ),
        (
            "name=\"aliasTags\" derived=\"true\"",
            "name=\"aliasTags\" derived=\"true\" ordered=\"false\"",
        ),
        (
            "name=\"aliasPeer\" derived=\"true\"",
            "name=\"aliasPeer\" derived=\"true\" containment=\"true\"",
        ),
        (
            "name=\"title\" unsettable=\"true\"",
            "name=\"title\" derived=\"true\"",
        ),
    ] {
        let result = compile_migrated_fixture(
            json!({"ecore":ecore.replace(from,to),"xtext":xtext,"rcore_behavior":rcore_behavior}),
        )
        .unwrap();
        assert!(result["package"].is_null(), "{from} => {to}");
        assert!(
            result["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["message"]
                    .as_str()
                    .unwrap()
                    .contains("projection")),
            "{result}"
        );
    }
    let container_backing = ecore.replace(
        "<eStructuralFeatures x:type=\"e:EReference\" name=\"peer\"",
        "<eStructuralFeatures x:type=\"e:EReference\" name=\"children\" containment=\"true\" upperBound=\"-1\" eType=\"#//Item\" eOpposite=\"#//Item/peer\"/><eStructuralFeatures x:type=\"e:EReference\" name=\"peer\" eOpposite=\"#//Item/children\"",
    );
    let rejected = compile_migrated_fixture(
        json!({"ecore":container_backing,"xtext":xtext,"rcore_behavior":rcore_behavior}),
    )
    .unwrap();
    assert!(rejected["package"].is_null(), "{rejected}");
    let compiled = compile_migrated_fixture(
        json!({"ecore":ecore,"xtext":xtext,"rcore_behavior":rcore_behavior}),
    )
    .unwrap();
    let mut tampered = compiled["package"].clone();
    tampered["semantics"]["derived"][0]["expression"]["feature"] = json!("name");
    let sources = json!([{"uri":"example.settings","text":"model Demo { item a }"}]);
    let authoritative = call(
        "evaluate",
        json!({"package":compiled["package"],"sources":sources}),
    )
    .unwrap();
    let replayed = call("evaluate", json!({"package":tampered,"sources":sources})).unwrap();
    assert_eq!(replayed, authoritative);
    assert!(
        replayed["derived"]
            .as_object()
            .unwrap()
            .values()
            .any(|fields| fields["aliasTitle"] == "seed")
    );
}

#[test]
fn declared_constructor_dependencies_cannot_be_bypassed_by_script_creation() {
    let semantics = format!(
        "{}\ninitialize m.Item using \"required\" version \"1\" {{ title; }}",
        include_str!("../../../fixtures/ecore-settings/settings-behavior.rcore")
    );
    let compiled = compile_migrated_fixture(json!({
        "ecore":include_str!("../../../fixtures/ecore-settings/settings.ecore"),
        "xtext":include_str!("../../../fixtures/ecore-settings/settings.xtext"),
        "rcore_behavior":semantics
    }))
    .unwrap();
    let package = compiled["package"].clone();
    let evaluated = call(
        "evaluate",
        json!({"package":package,"sources":[{"uri":"empty.settings","text":"model Empty { }"}]}),
    )
    .unwrap();
    begin(&package, &evaluated["model"]);
    let error = call(
        "script/create",
        json!({"class":"Item","values":{"name":"bad"}}),
    )
    .unwrap_err();
    assert!(
        error.contains("requires initializer provider dispatch"),
        "{error}"
    );
    // Script errors abort the enclosing transaction. A fresh transaction must
    // still reject the missing provider rather than reuse partial construction.
    begin(&package, &evaluated["model"]);
    let retry = call(
        "script/create",
        json!({"class":"Item","values":{"name":"retry"}}),
    )
    .unwrap_err();
    assert!(
        retry.contains("requires initializer provider dispatch"),
        "{retry}"
    );
    super::assert_store_abi(
        &json!([error, retry]),
        "0e8ce2a5e8d0cd4ca9f9b72af84530a8df9ee82da8a646b5dc0c246838f857a1",
    );
}

#[test]
fn structural_scripts_move_delete_and_never_reuse_a_deleted_handle() {
    let (package, model) = fixture();
    begin(&package, &model);
    let library = find("Library", "Catalog");
    let branch = call(
        "script/create",
        json!({"class":"Library","values":{"name":"Branch"}}),
    )
    .expect("branch")["id"]
        .clone();
    let child = call(
        "script/create",
        json!({"class":"Member","values":{"name":"temporary"}}),
    )
    .expect("child")["id"]
        .clone();
    assert_eq!(
        call(
            "script/reparent",
            json!({"id":child,"owner":library,"feature":"members","position":0})
        )
        .expect("attach"),
        json!(true)
    );
    assert_eq!(
        call(
            "script/reparent",
            json!({"id":child,"owner":branch,"feature":"members"})
        )
        .expect("transfer"),
        json!(true)
    );
    assert_eq!(
        call(
            "script/reparent",
            json!({"id":child,"owner":branch,"feature":"members"})
        )
        .expect("no-op"),
        json!(false)
    );
    let removed = call(
        "script/delete",
        json!({"ids":[branch],"selection":"subtrees","incoming":"reject"}),
    )
    .expect("delete subtree");
    assert_eq!(removed.as_array().expect("IDs").len(), 2);
    assert!(removed.as_array().expect("IDs").contains(&child));
    let replacement = call(
        "script/create",
        json!({"class":"Member","values":{"name":"replacement"}}),
    )
    .expect("replacement")["id"]
        .clone();
    assert_ne!(replacement, child);
    assert_ne!(replacement, branch);
    let result = call("script/commit", json!({})).expect("commit");
    super::assert_store_abi(
        &json!([removed, result]),
        "c209891d03c3e53a0f7e524f78f4a5aa439a457fa79b2b405d046a1e5f74ffcc",
    );
    let elements = result["model"]["elements"].as_array().expect("elements");
    assert!(elements.iter().any(|e| e["id"] == replacement));
    assert!(
        !elements
            .iter()
            .any(|e| e["id"] == child || e["id"] == branch)
    );
    begin(&package, &result["model"]);
    assert!(call("script/get", json!({"id":child,"feature":"name"})).is_err());
    assert!(call("script/commit", json!({})).is_err());
}

#[test]
fn structural_script_errors_abort_the_whole_transaction() {
    let (package, model) = fixture();
    for request in [
        json!({"method":"script/delete","ids":[],"selection":"unknown","incoming":"reject"}),
        json!({"method":"script/delete","ids":[1],"selection":"objects","incoming":"detach"}),
        json!({"method":"script/delete","ids":[],"selection":"objects","incoming":"unknown"}),
        json!({"method":"script/reparent","id":"missing","owner":null,"position":0}),
        json!({"method":"script/reparent","id":"missing","owner":"missing","feature":"members","position":-1}),
        json!({"method":"script/reparent","id":"missing","owner":"missing","feature":"members","position":1.5}),
    ] {
        begin(&package, &model);
        call(
            "script/create",
            json!({"class":"Member","values":{"name":"discarded"}}),
        )
        .expect("prior edit");
        assert!(call(request["method"].as_str().expect("method"), request.clone()).is_err());
        assert!(call("script/commit", json!({})).is_err());
        assert_eq!(
            call("script/transaction", json!({})).expect("report")["status"],
            "rolled_back"
        );
    }
}

#[test]
fn script_delete_rejects_incoming_and_detach_still_checks_publication() {
    let (package, model) = fixture();
    begin(&package, &model);
    let member = find("Member", "ada");
    assert!(
        call(
            "script/delete",
            json!({"ids":[member],"selection":"objects","incoming":"reject"})
        )
        .is_err()
    );
    assert!(call("script/commit", json!({})).is_err());
    begin(&package, &model);
    call(
        "script/delete",
        json!({"ids":[member],"selection":"objects","incoming":"detach"}),
    )
    .expect("draft cleanup");
    // A surviving Loan requires its member. Draft cleanup cannot bypass publication.
    assert!(call("script/commit", json!({})).is_err());
}
