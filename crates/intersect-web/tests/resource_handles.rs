#[path = "support/mod.rs"]
mod support;
use serde_json::{Value, json};
const RCORE: &str = include_str!("../../../fixtures/resource-handles/handles-structure.rcore");
fn request() -> Value {
    json!({"method":"compile_rcore","rcore":RCORE,"xtext":include_str!("../../../fixtures/resource-handles/handles.xtext"),"rcore_behavior":include_str!("../../../fixtures/resource-handles/handles-behavior.rcore"),"conversions":serde_json::from_str::<Value>(include_str!("../../../fixtures/resource-handles/conversions-rcore.json")).unwrap()})
}
fn dispatch(request: &Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn wire(contract: &Value) -> Value {
    let source = &contract["declarations"][0]["source"];
    json!({"kind":"converted","datatype":contract["datatype"],"provider":contract["provider"],"value":if contract["provider"]["name"]=="ecore.definition-resource" {json!({"resource":source})} else {json!({"resources":[source]})}})
}
#[test]
fn resource_access_model_identity_typing_and_export_use_exact_retained_definitions() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    let source = &input["conversions"][0]["declarations"][0]["source"];
    let op = &package["schema"]["classes"]["Catalog"]["operations"][0];
    assert_eq!(
        op["result"]["target"],
        json!({"kind":"datatype","name":"__standard_EResource"})
    );
    assert_eq!(
        op["parameters"][0]["value"]["target"],
        json!({"kind":"datatype","name":"__standard_EResourceSet"})
    );
    let evaluated=dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"sample.handles","text":"catalog example"}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    let id = &model["elements"][0]["id"];
    dispatch(&json!({"method":"script/begin","package":package,"model":model,"revision":"1"}))
        .unwrap();
    for (index, feature) in ["resource", "localResource", "resources", "localSet"]
        .into_iter()
        .enumerate()
    {
        assert!(model["elements"][0]["values"][feature].is_null());
        let handle = wire(&input["conversions"][index]);
        let read = dispatch(
            &json!({"method":"resource/read","package":package,"handle":handle,"from":source}),
        )
        .unwrap();
        assert_eq!(read["source"]["text"], RCORE);
        assert_eq!(read["source"]["id"], *source);
        let resolved=dispatch(&json!({"method":"resource/resolve","package":package,"handle":handle,"from":source,"href":"#//Catalog"})).unwrap();
        assert_eq!(resolved["declaration"], json!({"source":source,"path":[0]}));
        dispatch(
            &json!({"method":"script/set","revision":"1","id":id,"feature":feature,"value":handle}),
        )
        .unwrap();
    }
    let committed = dispatch(&json!({"method":"script/commit","revision":"1"})).unwrap();
    let saved = &committed["model"];
    for (feature, value) in [
        ("resource", wire(&input["conversions"][1])),
        (
            "resources",
            json!({"kind":"converted","datatype":input["conversions"][2]["datatype"],"provider":input["conversions"][2]["provider"],"value":{"resources":[source,source]}}),
        ),
    ] {
        dispatch(&json!({"method":"script/begin","package":package,"model":saved,"revision":"2"}))
            .unwrap();
        assert!(dispatch(&json!({"method":"script/set","revision":"2","id":id,"feature":feature,"value":value})).is_err());
        assert!(dispatch(&json!({"method":"script/commit","revision":"2"})).is_err());
    }
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    for (body, ok) in [
        (
            "fn same(a: h.Catalog,b: h.Catalog) -> Bool = a.resource == b.resource && a.resources == b.resources;",
            true,
        ),
        (
            "fn wrong(a: h.Catalog) -> Bool = a.resource == a.resources;",
            false,
        ),
        (
            "fn wrong(a: h.Catalog) -> Bool = a.resources == a.localSet;",
            false,
        ),
    ] {
        let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, RCORE);
        let mut sem = DefinitionSource::new(
            "handles-behavior.rcore",
            SourceKind::RcoreBehavior,
            format!(
                "rcore 1; package checks version \"1\"; import model \"urn:intersect:handles\" as h; {body}"
            ),
        );
        sem.dependencies.push(model.id.clone());
        let check = json!({"method":"rcore_behavior/check","bundle":{"version":1,"sources":[model,sem],"namespaces":[]},"entry":sem.id,"conversions":input["conversions"]});
        let checked = dispatch(&check).unwrap();
        assert_eq!(checked["ok"], ok, "{checked}");
        assert_eq!(checked["executable"], false);
    }
    let mut export = input.clone();
    export["method"] = json!("package/export");
    export["compilation"] = package["compilation"].clone();
    export["metadata"] = json!({"package_id":"example.handles","release":"1","display_name":"Handles","description":"Scoped resources"});
    let exported = dispatch(&export).unwrap();
    assert_eq!(
        exported["manifest"]["runtime"]["conversions"],
        json!([
            input["conversions"][0],
            input["conversions"][2],
            input["conversions"][1],
            input["conversions"][3]
        ])
    );
    let loaded: Value =
        serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
    let restored: Value = serde_json::from_str(&saved.to_string()).unwrap();
    dispatch(&json!({"method":"script/begin","package":loaded,"model":restored,"revision":"3"}))
        .unwrap();
    assert_eq!(
        dispatch(&json!({"method":"script/commit","revision":"3"})).unwrap()["model"],
        restored
    );
    assert_eq!(dispatch(&json!({"method":"resource/read","package":loaded,"handle":wire(&input["conversions"][0]),"from":source})).unwrap()["source"]["text"],RCORE);
    let mut narrower = input.clone();
    narrower["conversions"][0]["declarations"] = json!([]);
    let narrowed = dispatch(&narrower).unwrap();
    assert_eq!(narrowed["ok"], true);
    assert_ne!(
        narrowed["package"]["schema"]["revision"],
        package["schema"]["revision"]
    );
    assert!(dispatch(&json!({"method":"script/begin","package":narrowed["package"],"model":saved,"revision":"4"})).is_err());
}
#[test]
fn scopes_provider_revisions_and_malformed_access_reject() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    let package = &compiled["package"];
    let source = &input["conversions"][0]["declarations"][0]["source"];
    let base = json!({"method":"resource/resolve","package":package,"handle":wire(&input["conversions"][0]),"from":source,"href":"#//Catalog"});
    for (pointer, bad) in [
        ("/handle/provider/version", json!("2")),
        ("/handle/datatype/source/sha256", json!("stale")),
        ("/handle/value/resource/sha256", json!("stale")),
        ("/handle/value", json!({"resources":[]})),
        (
            "/href",
            json!("https://never-loaded.test/file.ecore#//Item"),
        ),
        ("/from/sha256", json!("stale")),
        ("/package/schema/conversion_contracts", json!([])),
    ] {
        let mut bad_request = base.clone();
        *bad_request.pointer_mut(pointer).unwrap() = bad;
        assert!(dispatch(&bad_request).is_err());
    }
    for index in 0..4 {
        for (field, bad) in [
            ("serializable", json!(true)),
            ("provider", json!({"name":"example.rgb","version":"1"})),
            ("declarations", json!([{"source":source,"path":[2]}])),
            (
                "declarations",
                json!([{"source":source,"path":[]},{"source":source,"path":[]}]),
            ),
        ] {
            let mut invalid = input.clone();
            invalid["conversions"][index][field] = bad;
            assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
        }
    }
    for field in ["resource", "resources"] {
        let ecore = support::with_default(RCORE, field, "opaque");
        let source = intersect_model::definitions::bundle::DefinitionSource::new(
            "memory:model.rcore",
            intersect_model::definitions::bundle::SourceKind::Rcore,
            &ecore,
        )
        .id;
        let mut invalid = input.clone();
        invalid["rcore"] = json!(ecore);
        for contract in invalid["conversions"].as_array_mut().unwrap() {
            contract["declarations"][0]["source"] = json!(source);
            if contract["datatype"]["source"]["uri"] == "memory:model.rcore" {
                contract["datatype"]["source"] = json!(source);
            }
        }
        assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
    }
    let mut missing = input;
    missing["conversions"] = json!([]);
    assert_eq!(dispatch(&missing).unwrap()["ok"], false);
}
