//! Shared actual-dispatcher archive/context controls, callable on native and WASM.
use intersect_model::definitions::bundle::SourceId;
use serde_json::{Value, json};

fn call(request: &Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn rejected(request: &Value) -> String {
    intersect_web::dispatch(&request.to_string()).unwrap_err()
}
fn rehash(files: &mut Value, name: &str, value: &Value) {
    let text = value.to_string();
    let mut manifest: Value =
        serde_json::from_str(files["manifest.json"].as_str().unwrap()).unwrap();
    manifest["files"][name] =
        json!({"bytes":text.len(),"sha256":SourceId::for_text("ignored", &text).sha256});
    files[name] = json!(text);
    files["manifest.json"] = json!(manifest.to_string());
}
pub fn verify_ordinary_catalog_archives() {
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/durable_rgb.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 5);
    let mut analyses = 0;
    for case in cases {
        let compiled = call(&case["compile_request"]);
        assert_eq!(compiled["ok"], true);
        let mut export_request = case["compile_request"].clone();
        export_request["method"] = json!("package/export");
        export_request["metadata"] = json!({"package_id":"document.context","release":"1","display_name":"Exact source context","description":"Original typed graph and installed providers"});
        let exported = call(&export_request);
        assert_eq!(exported["package"], compiled["package"]);
        let mut files = exported["files"].clone();
        files["manifest.json"] = json!(exported["manifest"].to_string());
        let conversions = case["compile_request"]["conversions"].clone();
        let request = json!({"method":"package/import_with_conversions","files":files,"conversions":conversions});
        let imported = call(&request);
        assert_eq!(imported["package"], compiled["package"]);
        assert_eq!(imported["manifest"], exported["manifest"]);
        assert_eq!(imported["files"], exported["files"]);
        assert!(
            rejected(&json!({"method":"package/import","files":files,"conversions":conversions}))
                .contains("Explicit conversion registry required")
        );
        for observation in case["upstream"].as_array().unwrap() {
            let sources =
                json!([{"uri":"memory:archive-context/current.model","text":observation["text"]}]);
            let original = call(
                &json!({"method":"document/analyze","package":compiled["package"],"sources":sources}),
            );
            let restored = call(
                &json!({"method":"document/analyze","package":imported["package"],"sources":sources}),
            );
            assert_eq!(
                restored, original,
                "complete model/node/error/accounting/source parity"
            );
            analyses += 1;
        }
        let mut conflicting = request.clone();
        conflicting["inference_context"] = json!({});
        assert!(rejected(&conflicting).contains("Conflicting ordinary and inferred"));
        let mut absent = request.clone();
        absent.as_object_mut().unwrap().remove("conversions");
        assert!(rejected(&absent).contains("conversion"));
        absent["conversions"] = json!([]);
        assert!(rejected(&absent).contains("conversion"));
        for field in [
            "provider_name",
            "provider_version",
            "source_hash",
            "declaration_path",
            "serializable",
            "duplicate",
            "null",
        ] {
            let mut bad = request.clone();
            match field {
                "provider_name" => {
                    bad["conversions"][0]["provider"]["name"] = json!("uninstalled.provider")
                }
                "provider_version" => bad["conversions"][0]["provider"]["version"] = json!("stale"),
                "source_hash" => {
                    bad["conversions"][0]["datatype"]["source"]["sha256"] = json!("0".repeat(64))
                }
                "declaration_path" => bad["conversions"][0]["datatype"]["path"] = json!([0]),
                "serializable" => bad["conversions"][0]["serializable"] = json!(false),
                "duplicate" => {
                    let contract = bad["conversions"][0].clone();
                    bad["conversions"].as_array_mut().unwrap().push(contract);
                }
                _ => bad["conversions"] = Value::Null,
            }
            assert!(!rejected(&bad).is_empty(), "{field}");
        }
        let mut bad = request.clone();
        let source = exported["manifest"]["definitions"]["bundle"]
            .as_str()
            .unwrap();
        bad["files"][source] = json!("corrupted before catalog creation");
        assert!(rejected(&bad).contains("Archive digest mismatch"));
        let package_path = exported["manifest"]["compiled_package"].as_str().unwrap();
        for field in [
            "nominal_declaration",
            "nominal_return_body",
            "schema_provider",
        ] {
            let mut bad = request.clone();
            let mut cached = compiled["package"].clone();
            match field {
                "nominal_declaration" => {
                    let rule = cached["grammar"]["rules"]
                        .as_object_mut()
                        .unwrap()
                        .values_mut()
                        .find(|rule| !rule["nominal_return"].is_null());
                    if let Some(rule) = rule {
                        rule["nominal_return"]["path"] = json!([0]);
                    } else {
                        cached["grammar"]["terminals"]
                            .as_object_mut()
                            .unwrap()
                            .values_mut()
                            .find(|terminal| !terminal["nominal_return"].is_null())
                            .unwrap()["nominal_return"]["path"] = json!([0]);
                    }
                }
                "nominal_return_body" => {
                    cached["grammar"]["rules"]["Paint"]["body"] = json!({"kind":"empty"})
                }
                _ => {
                    cached["schema"]["conversion_contracts"][0]["provider"]["version"] =
                        json!("forged")
                }
            }
            rehash(&mut bad["files"], package_path, &cached);
            assert!(
                rejected(&bad).contains("Archived compiled package differs"),
                "{field}"
            );
        }
        for field in ["runtime_provider", "profile", "compiler", "source_policy"] {
            let mut bad = request.clone();
            let mut manifest = exported["manifest"].clone();
            match field {
                "runtime_provider" => {
                    manifest["runtime"]["conversions"][0]["provider"]["version"] = json!("forged")
                }
                "profile" => manifest["runtime"]["profile"] = json!("intersect.xtext.native.v64"),
                "compiler" => manifest["compiler"]["version"] = json!("forged"),
                _ => {
                    manifest["runtime"]["source_policy"] =
                        json!({"structure_bytes":1_000_000_000,"xtext_bytes":1_000_000_000})
                }
            }
            bad["files"]["manifest.json"] = json!(manifest.to_string());
            assert!(!rejected(&bad).is_empty(), "{field}");
        }
    }
    assert_eq!(analyses, 34);
}
