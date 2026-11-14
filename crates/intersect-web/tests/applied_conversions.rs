use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use serde_json::{Value, json};
#[test]
fn applied_value_failures_are_diagnostics_not_transport_errors() {
    let source = DefinitionSource::new(
        "memory:applied-collections.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/ecore-generics/applied-collections.ecore"),
    );
    let id = |n| json!({"source":source.id,"path":[n]});
    let argument = json!({"kind":"classifier","declaration":id(0),"arguments":[],"raw":false});
    let application =
        json!({"kind":"classifier","declaration":id(2),"arguments":[argument],"raw":false});
    let mut request = json!({"method":"conversion/applied","bundle":{"version":1,"sources":[source],"namespaces":[]},
        "conversions":[{"application":application,"conversion":{"datatype":id(2),"provider":{"name":"ecore.list","version":"1"},"serializable":false,
            "members":[{"nullable":false,"value":{"kind":"datatype","declaration":id(0)}}]}}],
        "operation":{"kind":"bind","application":application,"payload":{"items":[{"kind":"datatype","datatype":id(0),"value":"hello"}]}}});
    let call = |request: &Value| intersect_web::dispatch(&request.to_string()).unwrap();
    let valid = call(&request);
    assert_eq!(valid["ok"], true);
    assert_eq!(valid["value"]["kind"], "converted_application_v1");
    request["operation"]["payload"]["items"][0]["value"] = json!(42);
    let invalid = call(&request);
    assert_eq!(invalid["ok"], false);
    assert_eq!(invalid["model_published"], false);
    assert!(invalid.get("error").is_none());
    assert!(
        invalid["diagnostics"][0]["message"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    request["operation"] =
        json!({"kind":"validate","application":application,"value":valid["value"]});
    assert_eq!(call(&request)["ok"], true);
}
