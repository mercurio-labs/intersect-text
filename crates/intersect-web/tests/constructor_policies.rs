use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use serde_json::{Value, json};
fn dispatch(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn fixture(wrapped: bool) -> (Value, Value, Value) {
    fixture_domains(
        wrapped,
        ("ecore.list", "org.eclipse.emf.common.util.EList"),
        ("ecore.list", "org.eclipse.emf.common.util.EList"),
    )
}
fn fixture_domains(
    wrapped: bool,
    bound: (&str, &str),
    actual: (&str, &str),
) -> (Value, Value, Value) {
    let text = include_str!("../../../fixtures/ecore-generics/constructor-model.ecore")
        .replace(
            "name=\"List\" instanceClassName=\"org.eclipse.emf.common.util.EList\"",
            &format!("name=\"List\" instanceClassName=\"{}\"", bound.1),
        )
        .replace(
            "name=\"Alias\" instanceClassName=\"org.eclipse.emf.common.util.EList\"",
            &format!("name=\"Alias\" instanceClassName=\"{}\"", actual.1),
        );
    let source = DefinitionSource::new("memory:constructor-model", SourceKind::Ecore, text);
    let (source, ids) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |n| json!(ids[&vec![n]]);
    let ty = |n, args: Vec<Value>| json!({"kind":"classifier","declaration":id(n),"arguments":args,"raw":false});
    let codec = |n, members: Vec<Value>| json!({"datatype":id(n),"provider":{"name":if n == 2 {actual.0} else {"ecore.list"},"version":"1"},"serializable":false,"members":members});
    let alias = json!({"application":ty(2,vec![ty(0,vec![])]),"conversion":codec(2,vec![json!({"nullable":false,"value":{"kind":"datatype","declaration":id(0)}})])});
    let bounded = json!({"application":ty(3,vec![alias["application"].clone()]),"conversion":codec(3,vec![json!({"nullable":false,"value":{"kind":"applied","contract":alias}})])});
    let wrapper = codec(
        4,
        vec![json!({"nullable":false,"value":{"kind":"applied","contract":bounded}})],
    );
    let mut grammar = DefinitionSource::new(
        "memory:constructor.xtext",
        SourceKind::Xtext,
        format!(
            r#"grammar example.Constructors import "urn:constructors" as c
{0} returns c::{0}: 'holder' name=ID;"#,
            if wrapped { "OnlyWrapped" } else { "Holder" }
        ),
    );
    grammar.dependencies.push(source.id.clone());
    let mut semantics = DefinitionSource::new(
        "memory:constructor-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies.push(source.id.clone());
    let bundle = json!({"version":1,"sources":[source,grammar,semantics],"namespaces":[]});
    let constructors = [1,2].map(|n|dispatch(json!({"method":"conversion/constructor","bundle":bundle,"datatype":id(n),"provider":{"name":format!("{}.constructor", if n == 1 {bound.0} else {actual.0}),"version":"1"}}))["contract"].clone());
    let receiver = ty(if wrapped { 6 } else { 5 }, vec![]);
    let compilation = json!({"bundle":bundle,"ecore":source.id,"namespace":"urn:constructors","xtext":grammar.id,"rcore_behavior":semantics.id,"materialized_types":[receiver]});
    let alias_value = json!({"kind":"converted_application_v1","application":alias["application"],"converted":{"kind":"converted","datatype":id(2),"provider":alias["conversion"]["provider"],"value":{"items":[{"kind":"datatype","datatype":id(0),"value":"a"}]}}});
    let bounded_value = json!({"kind":"converted_application_v1","application":bounded["application"],"converted":{"kind":"converted","datatype":id(3),"provider":bounded["conversion"]["provider"],"value":{"items":[alias_value]}}});
    let value = if wrapped {
        json!({"kind":"converted","datatype":id(4),"provider":wrapper["provider"],"value":{"items":[bounded_value]}})
    } else {
        bounded_value
    };
    (
        json!({"method":"compile_bundle","compilation":compilation,"conversions":if wrapped {vec![wrapper]} else {vec![]},"applied_conversions":if wrapped {vec![]} else {vec![bounded]},"constructors":constructors}),
        value,
        alias_value,
    )
}

#[test]
fn installed_constructor_aliases_replay_bound_only_dependencies_and_preserve_nominal_values() {
    for wrapped in [false, true] {
        let (request, value, alias) = fixture(wrapped);
        let compiled = dispatch(request.clone());
        assert_eq!(compiled["ok"], true, "{compiled}");
        let package = &compiled["package"];
        assert_eq!(
            package["schema"]["constructor_contracts"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        if wrapped {
            assert!(
                package["schema"]
                    .get("applied_conversion_contracts")
                    .is_none()
            );
        }
        let mut materialize = request.clone();
        materialize["method"] = "ecore/materialize".into();
        materialize["bundle"] = request["compilation"]["bundle"].clone();
        materialize["types"] = request["compilation"]["materialized_types"].clone();
        materialize["instances"] = json!([{"id":"h","type":materialize["types"][0]}]);
        let field = if wrapped { "wrapped" } else { "values" };
        materialize["edits"] = json!([["h", field, value]]);
        let result = dispatch(materialize.clone());
        assert_eq!(result["ok"], true, "{result}");
        assert_eq!(result["model"]["elements"][0]["values"][field], value);
        let mut reversed = request.clone();
        reversed["constructors"].as_array_mut().unwrap().reverse();
        assert_eq!(dispatch(reversed)["package"], *package);
        let evaluation = dispatch(
            json!({"method":"evaluate","package":package,"sources":[{"uri":"memory:model","text":"holder one"}]}),
        );
        assert_eq!(
            evaluation["model"]["elements"].as_array().unwrap().len(),
            1,
            "{evaluation}"
        );
        let mut wrong = value.clone();
        let bounded = if wrapped {
            &mut wrong["value"]["items"][0]
        } else {
            &mut wrong
        };
        bounded["converted"]["value"]["items"][0]["application"]["declaration"]["path"] =
            json!([1]);
        bounded["converted"]["value"]["items"][0]["converted"]["datatype"]["path"] = json!([1]);
        materialize["edits"] = json!([["h", field, wrong]]);
        assert_eq!(dispatch(materialize)["ok"], false);
        for selected in [json!([]), json!([request["constructors"][0]])] {
            let mut missing = request.clone();
            missing["constructors"] = selected;
            assert_eq!(dispatch(missing)["ok"], false);
        }
        if wrapped {
            let mut isolated = request.clone();
            let inner = json!({"bundle":request["compilation"]["bundle"],"entry":request["compilation"]["ecore"],"namespace":"urn:constructors","conversions":request["conversions"],"model":{"elements":[]}});
            isolated["conversions"][0]["members"][0]["value"] = json!({"kind":"reference","class":request["compilation"]["materialized_types"][0]["declaration"],"context":inner});
            let rejected = dispatch(isolated);
            assert_eq!(rejected["ok"], false);
            assert!(
                rejected.to_string().contains("assignability contract"),
                "{rejected}"
            );
        }
        let mut stale = package.clone();
        stale["schema"]["constructor_contracts"][0]["provider"]["version"] = "2".into();
        assert!(
            intersect_web::dispatch(
                &json!({"method":"evaluate","package":stale,"sources":[]}).to_string()
            )
            .is_err()
        );
        let mut forged = request.clone();
        forged["constructors"][0]["datatype"]["name"] = "Forged".into();
        assert_eq!(dispatch(forged)["ok"], false);
        if !wrapped {
            let bound = dispatch(
                json!({"method":"conversion/applied","bundle":request["compilation"]["bundle"],"conversions":request["applied_conversions"],"constructors":request["constructors"],"operation":{"kind":"bind","application":request["applied_conversions"][0]["application"],"payload":value["converted"]["value"]}}),
            );
            assert_eq!(bound["value"], value);
            let empty = dispatch(
                json!({"method":"collection/step","package":package,"application":request["applied_conversions"][0]["application"],"operation":{"kind":"empty"}}),
            );
            let inserted = dispatch(
                json!({"method":"collection/step","package":package,"application":request["applied_conversions"][0]["application"],"collection":empty["collection"],"operation":{"kind":"insert","index":0,"item":alias}}),
            );
            assert_eq!(inserted["collection"], value);
        }
    }
}

#[test]
fn installed_directional_collection_domains_check_bounds_and_execute_values() {
    let domains = [
        ("ecore.list", "org.eclipse.emf.common.util.EList"),
        ("java.list", "java.util.List"),
        ("java.collection", "java.util.Collection"),
    ];
    for (bound_index, bound) in domains.iter().enumerate() {
        for (actual_index, actual) in domains.iter().enumerate() {
            let (request, value, alias) = fixture_domains(false, *bound, *actual);
            let compiled = dispatch(request.clone());
            let valid = actual_index <= bound_index;
            assert_eq!(compiled["ok"], valid, "{actual:?} -> {bound:?}: {compiled}");
            if !valid {
                assert!(compiled.to_string().contains("violates generic bounds"));
                continue;
            }
            let package = &compiled["package"];
            let mut mismatch = request.clone();
            mismatch["applied_conversions"][0]["application"]["arguments"][0]["arguments"][0]["declaration"]
                ["path"] = json!([7]);
            mismatch["applied_conversions"][0]["conversion"]["members"][0]["value"]["contract"]["application"]
                ["arguments"][0]["declaration"]["path"] = json!([7]);
            mismatch["applied_conversions"][0]["conversion"]["members"][0]["value"]["contract"]["conversion"]
                ["members"][0]["value"]["declaration"]["path"] = json!([7]);
            let rejected = dispatch(mismatch);
            assert_eq!(rejected["ok"], false);
            assert!(
                rejected.to_string().contains("violates generic bounds"),
                "{rejected}"
            );
            let mut materialize = request.clone();
            materialize["method"] = "ecore/materialize".into();
            materialize["bundle"] = request["compilation"]["bundle"].clone();
            materialize["types"] = request["compilation"]["materialized_types"].clone();
            materialize["instances"] = json!([{"id":"h","type":materialize["types"][0]}]);
            materialize["edits"] = json!([["h", "values", value]]);
            let result = dispatch(materialize.clone());
            assert_eq!(result["ok"], true, "{result}");
            assert_eq!(result["model"]["elements"][0]["values"]["values"], value);
            let mut wrong = value.clone();
            wrong["converted"]["value"]["items"][0]["application"]["declaration"]["path"] =
                json!([1]);
            wrong["converted"]["value"]["items"][0]["converted"]["datatype"]["path"] = json!([1]);
            materialize["edits"] = json!([["h", "values", wrong]]);
            assert_eq!(dispatch(materialize)["ok"], false);
            let bound_value = dispatch(
                json!({"method":"conversion/applied","bundle":request["compilation"]["bundle"],"conversions":[request["applied_conversions"][0]["conversion"]["members"][0]["value"]["contract"]],"operation":{"kind":"bind","application":alias["application"],"payload":alias["converted"]["value"]}}),
            );
            assert_eq!(bound_value["value"], alias);
            let evaluation = dispatch(
                json!({"method":"evaluate","package":package,"sources":[{"uri":"memory:model","text":"holder one"}]}),
            );
            assert_eq!(evaluation["model"]["elements"].as_array().unwrap().len(), 1);
            let mut reversed = request.clone();
            reversed["constructors"].as_array_mut().unwrap().reverse();
            assert_eq!(dispatch(reversed)["package"], *package);
        }
    }
}
