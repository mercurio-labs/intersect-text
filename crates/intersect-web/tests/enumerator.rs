#[path = "support/mod.rs"]
mod support;
use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    compiler::standard_datatype,
};
use serde_json::{Value, json};
fn dispatch(request: &Value) -> Result<Value, String> {
    intersect_web::dispatch(&request.to_string())
}
fn request() -> Value {
    let ecore = "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='values' nsURI='urn:values'><eClassifiers xsi:type='e:EEnum' name='Mode'><eLiterals name='First' value='7'/><eLiterals name='Alias' value='7'/></eClassifiers><eClassifiers xsi:type='e:EEnum' name='Other'><eLiterals name='First' value='7'/></eClassifiers><eClassifiers xsi:type='e:EClass' name='Item'><eStructuralFeatures xsi:type='e:EAttribute' name='value' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EEnumerator'/></eClassifiers></e:EPackage>";
    let rcore = support::migrate_text(ecore);
    let source = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, &rcore);
    json!({"method":"compile_rcore","rcore":rcore,"xtext":"grammar example.Values import 'urn:values' as v Item returns v::Item: 'item';", "rcore_behavior":r#"rcore 1; package emptyRules version "1";"#,
        "conversions":[{"datatype":standard_datatype("EEnumerator").unwrap().declaration(),"provider":{"name":"ecore.enumerator","version":"1"},"serializable":false,"declarations":[support::declaration(&source,"Mode")]}]})
}
fn scope(input: &Value, name: &str) -> Value {
    let source = DefinitionSource::new(
        "memory:model.rcore",
        SourceKind::Rcore,
        input["rcore"].as_str().unwrap(),
    );
    json!(support::declaration(&source, name))
}
fn value(package: &Value, name: &str, ordinal: usize) -> Value {
    let contract = &package["schema"]["conversion_contracts"][0];
    let enumeration = &package["schema"]["enumerations"][name];
    json!({"kind":"converted","datatype":contract["datatype"],"provider":contract["provider"],"value":{"kind":"enum","enumeration":enumeration["declaration"],"literal":enumeration["literals"][ordinal]["declaration"]}})
}
#[test]
fn explicit_enum_scope_survives_export_rebuild_and_validates_script_edits() {
    let input = request();
    let compiled = dispatch(&input).unwrap();
    assert_eq!(compiled["ok"], true, "{compiled}");
    let package = &compiled["package"];
    assert_eq!(
        package["schema"]["conversion_contracts"],
        input["conversions"]
    );
    let evaluated = dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"values.model","text":"item"}]})).unwrap();
    assert_eq!(evaluated["diagnostics"], json!([]));
    let model = &evaluated["model"];
    assert!(model["elements"][0]["values"]["value"].is_null());
    let id = &model["elements"][0]["id"];
    for (value, ok) in [
        (value(package, "Mode", 0), true),
        (value(package, "Mode", 1), true),
        (value(package, "Other", 0), false),
    ] {
        dispatch(&json!({"method":"script/begin","package":package,"model":model,"revision":"1"}))
            .unwrap();
        let set = dispatch(
            &json!({"method":"script/set","revision":"1","id":id,"feature":"value","value":value}),
        );
        assert_eq!(set.is_ok(), ok, "{set:?}");
        let prepared = dispatch(&json!({"method":"script/commit","revision":"1"}));
        assert_eq!(prepared.is_ok(), ok);
        if ok {
            assert_eq!(
                prepared.unwrap()["model"]["elements"][0]["values"]["value"],
                value
            );
        }
    }
    let mut export = input.clone();
    export["method"] = json!("package/export");
    export["compilation"] = package["compilation"].clone();
    export["metadata"] = json!({"package_id":"example.enums","release":"1","display_name":"Enums","description":"Explicit enum scope"});
    let exported = dispatch(&export).unwrap();
    assert_eq!(
        exported["manifest"]["runtime"]["conversions"],
        input["conversions"]
    );
    let retained: Value = serde_json::from_str(
        exported["files"]["definitions/conversions.json"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(retained, input["conversions"]);
    let loaded: Value =
        serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
    assert_eq!(dispatch(&json!({"method":"evaluate","package":loaded,"sources":[{"uri":"values.model","text":"item"}]})).unwrap(), evaluated);
    let mut wider = input.clone();
    let other = scope(&input, "Other");
    wider["conversions"][0]["declarations"]
        .as_array_mut()
        .unwrap()
        .push(other);
    let wide = dispatch(&wider).unwrap();
    assert_eq!(wide["ok"], true);
    assert_ne!(
        wide["package"]["schema"]["revision"],
        package["schema"]["revision"]
    );
    assert!(
        dispatch(
            &json!({"method":"script/begin","package":wide["package"],"model":model,"revision":"1"})
        )
        .is_err()
    );
    export["conversions"] = wider["conversions"].clone();
    assert_ne!(
        dispatch(&export).unwrap()["manifest"]["revision"],
        exported["manifest"]["revision"]
    );
}
#[test]
fn enum_scope_rejects_wrong_kinds_duplicates_stale_sources_and_unconsumed_config() {
    let input = request();
    let original = input["conversions"][0]["declarations"][0].clone();
    let other = scope(&input, "Other");
    let wrong = scope(&input, "Item");
    let mut stale = original.clone();
    stale["source"]["sha256"] = json!("changed");
    for declarations in [
        json!([original, original]),
        json!([other, original]),
        json!([wrong]),
        json!([stale]),
        json!(vec![original.clone(); 257]),
    ] {
        let mut invalid = input.clone();
        invalid["conversions"][0]["declarations"] = declarations;
        assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
    }
    for provider in ["example.rgb", "ecore.class"] {
        let mut invalid = input.clone();
        invalid["conversions"][0]["provider"]["name"] = json!(provider);
        assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
    }
    let mut empty = input.clone();
    empty["conversions"][0]["declarations"] = json!([]);
    let compiled = dispatch(&empty).unwrap();
    assert_eq!(compiled["ok"], true);
    let package = &compiled["package"];
    let evaluated = dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"values.model","text":"item"}]})).unwrap();
    dispatch(&json!({"method":"script/begin","package":package,"model":evaluated["model"],"revision":"1"})).unwrap();
    assert!(dispatch(&json!({"method":"script/set","revision":"1","id":evaluated["model"]["elements"][0]["id"],"feature":"value","value":value(package,"Mode",0)})).is_err());
}
