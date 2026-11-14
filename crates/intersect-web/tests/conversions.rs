#[path = "support/mod.rs"]
mod support;
use serde_json::{Value, json};
fn request() -> Value {
    let ecore = "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='paint' nsURI='urn:paint'><eClassifiers xsi:type='e:EDataType' name='RGB' instanceClassName='example.RGB'/><eClassifiers xsi:type='e:EClass' name='Paint'><eStructuralFeatures xsi:type='e:EAttribute' name='color' eType='#//RGB' defaultValueLiteral='#123abc'/></eClassifiers></e:EPackage>";
    let rcore = support::migrate_text(ecore);
    let source = intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:model.rcore",
        intersect_model::definitions::bundle::SourceKind::Rcore,
        &rcore,
    );
    json!({"method":"compile_rcore","rcore":rcore,"xtext":"grammar example.Paint import 'urn:paint' as p Paint returns p::Paint: 'paint';",
        "rcore_behavior":r#"rcore 1; package emptyRules version "1";"#,
        "conversions":[{"datatype":support::declaration(&source,"RGB"),"provider":{"name":"example.rgb","version":"1"},"serializable":true}]})
}
fn declared_object_request() -> Value {
    let mut input = request();
    let ecore = input["rcore"]
        .as_str()
        .unwrap()
        .replace("example.RGB", "java.lang.Object");
    let source = intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:model.rcore",
        intersect_model::definitions::bundle::SourceKind::Rcore,
        &ecore,
    );
    input["rcore"] = json!(ecore);
    input["conversions"][0]["datatype"]["source"] = json!(source.id);
    input
}
fn object_request() -> Value {
    let mut input = request();
    input["rcore"] = json!(
        r##"rcore 1; package paint version "1"; metamodel "urn:paint" prefix p { class Paint { attr color: external "http://www.eclipse.org/emf/2002/Ecore#//EJavaObject" [0,1] default "#123abc"; } }"##
    );
    let datatype =
        intersect_model::definitions::compiler::standard_datatype("EJavaObject").unwrap();
    assert_eq!(datatype.declaration().path, vec![39]);
    assert_eq!(datatype.instance_class_name(), Some("java.lang.Object"));
    assert!(datatype.serializable());
    input["conversions"][0]["datatype"] = json!(datatype.declaration());
    input
}
fn dispatch(value: &Value) -> Result<Value, String> {
    intersect_web::dispatch(&value.to_string())
}
#[test]
fn host_selects_only_installed_providers_and_rebuilds_untrusted_packages() {
    for input in [request(), declared_object_request(), object_request()] {
        let compiled = dispatch(&input).unwrap();
        assert_eq!(compiled["ok"], true, "{compiled}");
        let mut evaluation = json!({"method":"evaluate","package":compiled["package"],"sources":[{"uri":"paint.model","text":"paint"}]});
        let result = dispatch(&evaluation).unwrap();
        assert_eq!(result["diagnostics"], json!([]));
        assert_eq!(
            result["model"]["elements"][0]["values"]["color"]["value"],
            json!({"red":18,"green":58,"blue":188})
        );
        evaluation["package"]["schema"]["classes"]["Paint"]["features"]["color"]["default"] =
            json!("forged");
        assert_eq!(dispatch(&evaluation).unwrap(), result);
        for (field, bad) in [("name", "unknown.provider"), ("version", "2")] {
            let mut invalid = input.clone();
            invalid["conversions"][0]["provider"][field] = json!(bad);
            assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
            let mut invalid = evaluation.clone();
            invalid["package"]["schema"]["conversion_contracts"][0]["provider"][field] = json!(bad);
            assert!(dispatch(&invalid).is_err());
        }
        let mut missing = input.clone();
        missing["conversions"] = json!([]);
        assert_eq!(dispatch(&missing).unwrap()["ok"], false);
        let mut duplicate = input.clone();
        duplicate["conversions"]
            .as_array_mut()
            .unwrap()
            .push(input["conversions"][0].clone());
        assert_eq!(dispatch(&duplicate).unwrap()["ok"], false);
        evaluation["package"]["sources"] = json!(null);
        assert!(dispatch(&evaluation).is_err());
    }
}

#[test]
fn exported_custom_package_retains_checked_provider_requirements_and_reloads() {
    for mut input in [request(), declared_object_request(), object_request()] {
        let compiled = dispatch(&input).unwrap();
        input["compilation"] = compiled["package"]["compilation"].clone();
        input["method"] = json!("package/export");
        input["metadata"] = json!({"package_id":"example.paint","release":"1","display_name":"Paint","description":"Custom conversion control"});
        let exported = dispatch(&input).unwrap();
        assert_eq!(
            exported["manifest"]["runtime"]["conversions"],
            input["conversions"]
        );
        let contracts: Value = serde_json::from_str(
            exported["files"]["definitions/conversions.json"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(contracts, input["conversions"]);
        let package: Value =
            serde_json::from_str(exported["files"]["package.json"].as_str().unwrap()).unwrap();
        assert_eq!(package, exported["package"]);
        let result=dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"paint.model","text":"paint"}]})).unwrap();
        assert_eq!(result["diagnostics"], json!([]));
        assert_eq!(
            result["model"]["elements"][0]["values"]["color"]["value"],
            json!({"red":18,"green":58,"blue":188})
        );
    }
}

#[test]
fn script_transaction_validates_custom_values_before_publication() {
    for input in [request(), declared_object_request(), object_request()] {
        let compiled = dispatch(&input).unwrap();
        let package = &compiled["package"];
        let evaluated=dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"paint.model","text":"paint"}]})).unwrap();
        let before = &evaluated["model"];
        let context = json!({"id":"provider-script","language_revision":"1","base_generation":0});
        dispatch(&json!({"method":"script/begin","package":package,"model":before,"revision":"1","context":context})).unwrap();
        let id = before["elements"][0]["id"].as_str().unwrap();
        let mut value = before["elements"][0]["values"]["color"].clone();
        value["value"]["red"] = json!(255);
        dispatch(
            &json!({"method":"script/set","revision":"1","id":id,"feature":"color","value":value}),
        )
        .unwrap();
        let prepared = dispatch(&json!({"method":"script/commit","revision":"1"})).unwrap();
        assert_eq!(prepared["model"]["elements"][0]["values"]["color"], value);
        let published=dispatch(&json!({"method":"script/publish","package":package,"before":before,"model":prepared["model"],"current":before,"context":context,"revision":"1","generation":0})).unwrap();
        assert_eq!(published["model"], prepared["model"]);
        dispatch(&json!({"method":"script/begin","package":package,"model":before,"revision":"1","context":context})).unwrap();
        value["value"]["red"] = json!(256);
        assert!(
        dispatch(
            &json!({"method":"script/set","revision":"1","id":id,"feature":"color","value":value})
        )
        .is_err()
    );
        assert!(dispatch(&json!({"method":"script/commit","revision":"1"})).is_err());
    }
}

#[test]
fn pinned_class_provider_compiles_standard_defaults_operations_and_exports() {
    use intersect_model::definitions::compiler::standard_datatype;
    let datatype = standard_datatype("EJavaClass").unwrap();
    assert_eq!(
        datatype.declaration().source.sha256,
        "4ef00e244eda85b2da8670bb5582eb7c8d762d459e3e2c834b228384733c0aa6"
    );
    assert!(
        datatype
            .source_payload()
            .unwrap()
            .contains("eTypeParameters")
    );
    assert_eq!(datatype.instance_class_name(), Some("java.lang.Class"));
    assert!(datatype.serializable());
    for typed in ["", "ecore:EDataType "] {
        for operations in [
            "",
            "<eOperations name='kind' eType='http://www.eclipse.org/emf/2002/Ecore#//EJavaClass'><eParameters name='value' eType='http://www.eclipse.org/emf/2002/Ecore#//EJavaClass'/></eOperations>",
        ] {
            let ecore = format!(
                "<ecore:EPackage xmlns:ecore='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='paint' nsURI='urn:paint'><eClassifiers xsi:type='ecore:EClass' name='Paint'><eStructuralFeatures xsi:type='ecore:EAttribute' name='color' eType='{typed}http://www.eclipse.org/emf/2002/Ecore#//EJavaClass' defaultValueLiteral='[Ljava.lang.String;'/>{operations}</eClassifiers></ecore:EPackage>"
            );
            let mut input = request();
            input["rcore"] = json!(support::migrate_text(&ecore));
            input["conversions"] = json!([{"datatype":datatype.declaration(), "provider":{"name":"ecore.class","version":"1"},"serializable":true}]);
            let compiled = dispatch(&input).unwrap();
            assert_eq!(compiled["ok"], true, "{compiled}");
            let package = &compiled["package"];
            assert_eq!(
                package["schema"]["datatypes"]["__standard_EJavaClass"]["declaration"],
                json!(datatype.declaration())
            );
            let result = dispatch(&json!({"method":"evaluate","package":package,"sources":[{"uri":"model","text":"paint"}]})).unwrap();
            assert_eq!(result["diagnostics"], json!([]));
            assert_eq!(
                result["model"]["elements"][0]["values"]["color"]["value"],
                json!({"component":{"kind":"named","name":"java.lang.String","identity":{"name":"intersect.portable-java-core","version":"1"}},"dimensions":1})
            );
            if !operations.is_empty() {
                assert_eq!(
                    package["schema"]["classes"]["Paint"]["operations"][0]["result"]["target"],
                    json!({"kind":"datatype","name":"__standard_EJavaClass"})
                );
            }
            let mut export = input.clone();
            export["method"] = json!("package/export");
            export["compilation"] = package["compilation"].clone();
            export["metadata"] = json!({"package_id":"example.class","release":"1","display_name":"Class","description":"Portable class descriptors"});
            let exported = dispatch(&export).unwrap();
            assert_eq!(
                exported["manifest"]["runtime"]["conversions"],
                input["conversions"]
            );
            for bad in ["[V", "[Lunknown.Class;", "int[]"] {
                let mut invalid = input.clone();
                invalid["rcore"] = json!(support::migrate_text(
                    &ecore.replace("[Ljava.lang.String;", bad)
                ));
                assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
            }
            let mut invalid = input.clone();
            invalid["conversions"] = json!([]);
            assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
            for pointer in [
                "/conversions/0/provider/version",
                "/conversions/0/datatype/source/sha256",
            ] {
                let mut invalid = input.clone();
                *invalid.pointer_mut(pointer).unwrap() = json!("changed");
                assert_eq!(dispatch(&invalid).unwrap()["ok"], false);
            }
        }
    }
}

#[test]
fn rcore_behavior_custom_datatypes_check_with_exact_converters_without_claiming_execution() {
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    for domain in ["declared", "declared_object", "class", "object"] {
        let mut input = match domain {
            "object" => object_request(),
            "declared_object" => declared_object_request(),
            _ => request(),
        };
        if domain == "class" {
            input["rcore"] = json!(
                r#"rcore 1; package paint version "1"; metamodel "urn:paint" prefix p { class Paint { attr color: external "http://www.eclipse.org/emf/2002/Ecore#//EJavaClass" [0,1] default "int"; } }"#
            );
            input["conversions"] = json!([{"datatype":intersect_model::definitions::compiler::standard_datatype("EJavaClass").unwrap().declaration(),"provider":{"name":"ecore.class","version":"1"},"serializable":true}]);
        }
        for (body, ok) in [
            (
                "fn same(a: p.Paint, b: p.Paint) -> Bool = a.color == b.color;",
                true,
            ),
            ("fn wrong(a: p.Paint) -> String? = a.color;", false),
        ] {
            let model = DefinitionSource::new(
                "memory:model.rcore",
                SourceKind::Rcore,
                input["rcore"].as_str().unwrap(),
            );
            let text = format!(
                "rcore 1; package checks version \"1\"; import model \"urn:paint\" as p; {body}"
            );
            let mut entry =
                DefinitionSource::new("rcore_behavior", SourceKind::RcoreBehavior, &text);
            entry.dependencies.push(model.id.clone());
            let check = json!({"method":"rcore_behavior/check","bundle":{"version":1,"sources":[model,entry],"namespaces":[]},"entry":entry.id,"conversions":input["conversions"]});
            let result = dispatch(&check).unwrap();
            assert_eq!(result["ok"], ok, "{result}");
            assert_eq!(result["executable"], false);
            let mut missing = check.clone();
            missing.as_object_mut().unwrap().remove("conversions");
            assert_eq!(dispatch(&missing).unwrap()["ok"], false);
            let mut missing = check.clone();
            missing["conversions"] = json!([]);
            assert_eq!(dispatch(&missing).unwrap()["ok"], false);
            let mut compile = input.clone();
            compile["rcore_behavior"] = json!(text);
            let compiled = dispatch(&compile).unwrap();
            assert_eq!(compiled["ok"], ok, "{compiled}");
            if ok {
                assert_eq!(compiled["package"]["semantics"]["derived"], json!([]));
                assert_eq!(compiled["package"]["semantics"]["validations"], json!([]));
            }
            if domain.starts_with("declared") && ok {
                let mut nominal = check.clone();
                let source = DefinitionSource::new(
                    "rcore_behavior",
                    SourceKind::RcoreBehavior,
                    "rcore 1; package checks version \"1\"; import model \"urn:paint\" as p; fn identity(value: p.RGB) -> p.RGB = value; fn read(a: p.Paint) -> p.RGB? = a.color;",
                );
                nominal["entry"] = json!(source.id);
                nominal["bundle"]["sources"][1]["id"] = json!(source.id);
                nominal["bundle"]["sources"][1]["text"] = json!(source.text);
                assert_eq!(dispatch(&nominal).unwrap()["ok"], true);
            }
        }
    }
}

#[test]
fn named_object_entries_rebuild_the_installed_conversion_context() {
    for input in [request(), declared_object_request(), object_request()] {
        let compiled = dispatch(&input).unwrap();
        assert_eq!(compiled["ok"], true);
        let catalog =
            dispatch(&json!({"method":"text/entries","package":compiled["package"]})).unwrap();
        assert_eq!(catalog["entries"]["Paint"]["datatype"], false);
        let request = json!({"method":"evaluate","package":compiled["package"],"entry_rule":"Paint","sources":[{"uri":"paint.model","text":"paint"}]});
        let result = dispatch(&request).unwrap();
        assert_eq!(result["diagnostics"], json!([]));
        assert_eq!(
            result["model"]["elements"][0]["values"]["color"]["value"],
            json!({"red":18,"green":58,"blue":188})
        );
        assert_eq!(
            result["entry_values"]["paint.model"],
            json!({"kind":"object","value":result["model"]["elements"][0]["id"]})
        );
        let mut forged = request;
        forged["package"]["schema"]["conversion_contracts"][0]["provider"]["version"] =
            json!("uninstalled");
        assert!(dispatch(&forged).is_err());
    }
}
