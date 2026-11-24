use intersect_text::{
    compiler,
    distribution::{Distribution, ExecutionManifest, ExecutionProfile},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const SOURCE_PATH: &str = "execution/demo.rcore";
const SOURCE_TEXT: &str = "// π\r\nrcore 1; package runtime version \"1\";\r\n";

fn source_archive() -> Value {
    let package = compiler::compile_rcore_project(
        r#"
        rcore 1;
        package example version "1";
        metamodel "urn:example" prefix example {
            class Item { attr name: String [1,1]; }
        }
        "#,
        r#"
        grammar example.Items with org.eclipse.xtext.common.Terminals
        import "urn:example" as example
        Item returns example::Item: 'item' name=ID ';';
        "#,
    )
    .unwrap();
    call(
        "package/export",
        json!({
            "compilation": package.compilation.unwrap(),
            "metadata": {"package_id": "example.items", "release": "1", "display_name": "Items", "description": ""}
        }),
    )
    .unwrap()
}

fn call(method: &str, mut params: Value) -> Result<Value, String> {
    params["method"] = json!(method);
    intersect_web::dispatch(&params.to_string())
}

fn archive_files(archive: &Value) -> Value {
    let mut files = archive["files"].clone();
    let manifest = serde_json::to_string_pretty(&archive["manifest"]).unwrap();
    files["manifest.json"] = json!(format!("{manifest}\n"));
    files
}

fn execution_archive(source: Value) -> Value {
    let source: Distribution = serde_json::from_value(source).unwrap();
    let execution = ExecutionManifest {
        default: Some("demo".into()),
        profiles: BTreeMap::from([(
            "demo".into(),
            ExecutionProfile {
                engine: "intersect.state-machine@1".into(),
                source: SOURCE_PATH.into(),
            },
        )]),
    };
    serde_json::to_value(
        source
            .with_execution(
                execution,
                BTreeMap::from([(SOURCE_PATH.into(), SOURCE_TEXT.into())]),
            )
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn web_execution_archive_reexport_preserves_the_complete_archive() {
    let source = source_archive();
    let source_files = archive_files(&source);
    let loaded = call("package/import", json!({"files": source_files})).unwrap();
    assert_eq!(loaded["manifest"]["format_version"], 2);
    assert_eq!(loaded["manifest"], source["manifest"]);
    assert_eq!(loaded["files"], source["files"]);
    assert!(
        call(
            "package/import_with_execution",
            json!({"files": source_files})
        )
        .unwrap_err()
        .contains("require a format-3")
    );

    let archive = execution_archive(source);
    let files = archive_files(&archive);
    assert!(
        call("package/import", json!({"files": files}))
            .unwrap_err()
            .contains("execution")
    );
    let imported = call("package/import_with_execution", json!({"files": files})).unwrap();
    assert_eq!(imported["manifest"]["format_version"], 3);
    assert_eq!(imported["manifest"], archive["manifest"]);
    assert_eq!(imported["package"], archive["package"]);
    assert_eq!(imported["files"], files);
    assert_eq!(imported["files"][SOURCE_PATH].as_str(), Some(SOURCE_TEXT));
    assert!(imported["keywords"].is_array());
    let exported = call(
        "package/export_with_execution",
        json!({"files": imported["files"]}),
    )
    .unwrap();
    assert_eq!(exported["manifest"], imported["manifest"]);
    assert_eq!(exported["package"], imported["package"]);
    assert_eq!(exported["files"], files);
}

#[test]
fn web_execution_archive_reexport_rechecks_retained_files() {
    let archive = execution_archive(source_archive());
    let files = archive_files(&archive);
    let import = call("package/import_with_execution", json!({"files": files})).unwrap();
    let mut files = import["files"].clone();
    files[SOURCE_PATH] = json!("changed execution source");
    assert!(
        call("package/export_with_execution", json!({"files": files}))
            .unwrap_err()
            .contains("digest mismatch")
    );
}
