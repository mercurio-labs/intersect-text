//! Source-rebuilding archives share the same independent model consumers.
use intersect_text::package::{CompilationSourcePolicy, Package};
#[path = "../tests/xmi/support/compatibility.rs"]
mod ordinary;
use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use intersect_text::{
    distribution::{self, Distribution, PackageMetadata},
    inference::InferredLanguage,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub(crate) fn metadata() -> PackageMetadata {
    PackageMetadata {
        package_id: "inference.archive".into(),
        release: "1".into(),
        display_name: "Inferred source archive".into(),
        description: String::new(),
    }
}
pub(crate) fn extracted(d: &Distribution) -> BTreeMap<String, String> {
    let mut files = d.files.clone();
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&d.manifest).unwrap(),
    );
    files
}
pub(crate) fn roundtrip(language: &InferredLanguage) -> Package {
    // InferredLanguage is a checked producer object; callers supplied this policy
    // when creating it. Separate negative controls deny self-admitted archives.
    let policy = language.package().source_policy.unwrap();
    let d = distribution::build_inferred_with_source_policy(
        language.request().clone(),
        metadata(),
        policy,
        policy,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&d.package).unwrap(),
        serde_json::to_value(language.package()).unwrap()
    );
    assert_eq!(
        d.manifest.definitions,
        BTreeMap::from([("inference".into(), "definitions/inference.json".into())])
    );
    assert_eq!(
        serde_json::from_str::<Value>(&d.files["definitions/inference.json"]).unwrap(),
        serde_json::to_value(language.request()).unwrap()
    );
    let loaded = distribution::import_files_with_source_policy(extracted(&d), policy).unwrap();
    assert_eq!(
        serde_json::to_value(&loaded.package).unwrap(),
        serde_json::to_value(language.package()).unwrap()
    );
    loaded.package
}
pub(crate) fn model_roundtrip(
    language: &InferredLanguage,
    text: &str,
    expected: &intersect_text::Evaluation,
) {
    let package = roundtrip(language);
    let result = intersect_text::evaluate(
        &package,
        &[intersect_text::Source {
            uri: expected.model.elements[0].resource.clone(),
            text: text.into(),
        }],
    );
    assert_eq!(result.model, expected.model);
    assert_eq!(result.diagnostics, expected.diagnostics);
}
pub(crate) fn write_manifest(files: &mut BTreeMap<String, String>, edit: impl FnOnce(&mut Value)) {
    let mut manifest: Value = serde_json::from_str(&files["manifest.json"]).unwrap();
    edit(&mut manifest);
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&manifest).unwrap(),
    );
}
pub(crate) fn write_payload(files: &mut BTreeMap<String, String>, path: &str, value: &Value) {
    let text = serde_json::to_string(value).unwrap();
    write_manifest(
        files,
        |m| {
            m["files"][path] =
                json!({"bytes":text.len(),"sha256":format!("{:x}",Sha256::digest(text.as_bytes()))})
        },
    );
    files.insert(path.into(), text);
}
pub fn integrity_and_policy() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-fragment-runtime/cases.json"
    ))
    .unwrap();
    let request = super::composition::input(&cases[0]);
    let d = distribution::build_inferred(request.clone(), metadata()).unwrap();
    let files = extracted(&d);
    assert!(
        intersect_text::distribution::ordinary::import_files(files.clone())
            .unwrap_err()
            .contains("language frontend")
    );
    assert_eq!(
        serde_json::to_value(distribution::build_inferred(request.clone(), metadata()).unwrap())
            .unwrap(),
        serde_json::to_value(&d).unwrap()
    );
    let mut invalids = vec![];
    let mut invalid = files.clone();
    invalid.get_mut("package.json").unwrap().push(' ');
    invalids.push(invalid);
    let mut invalid = files.clone();
    let mut value: Value = serde_json::from_str(&invalid["package.json"]).unwrap();
    value["grammar"]["entry"] = json!("FORGED");
    write_payload(&mut invalid, "package.json", &value);
    invalids.push(invalid);
    let mut invalid = files.clone();
    let mut value: Value = serde_json::from_str(&invalid["definitions/inference.json"]).unwrap();
    value["bundle"]["sources"][0]["text"] = json!("changed without identity");
    write_payload(&mut invalid, "definitions/inference.json", &value);
    invalids.push(invalid);
    let mut invalid = files.clone();
    let mut value: Value = serde_json::from_str(&invalid["definitions/inference.json"]).unwrap();
    value["schema"] = json!({});
    write_payload(&mut invalid, "definitions/inference.json", &value);
    invalids.push(invalid);
    let mut invalid = files.clone();
    let mut value: Value = serde_json::from_str(&invalid["definitions/inference.json"]).unwrap();
    value["version"] = json!(99);
    write_payload(&mut invalid, "definitions/inference.json", &value);
    invalids.push(invalid);
    let mut invalid = files.clone();
    let mut value: Value = serde_json::from_str(&invalid["definitions/inference.json"]).unwrap();
    value["bundle"]["sources"][0]["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!({"uri":"memory:missing","sha256":"0".repeat(64)}));
    write_payload(&mut invalid, "definitions/inference.json", &value);
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(&mut invalid, |m| {
        m["compiler"]["name"] = json!("intersect-model")
    });
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(&mut invalid, |m| {
        m["compiler"]["version"] = json!("different")
    });
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(&mut invalid, |m| {
        m["runtime"]["profile"] =
            serde_json::to_value(intersect_text::grammar::Profile::NativeV55).unwrap()
    });
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(&mut invalid, |m| {
        m["runtime"]["source_policy"] = Value::Null
    });
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(&mut invalid, |m| {
        m["runtime"]["source_policy"]["xtext_bytes"] = json!(256001)
    });
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(&mut invalid, |m| {
        m["definitions"]["bundle"] = json!("definitions/inference.json")
    });
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(&mut invalid, |m| m["format_version"] = json!(99));
    invalids.push(invalid);
    let mut invalid = files.clone();
    invalid.remove("package.json");
    invalids.push(invalid);
    let mut invalid = files.clone();
    invalid.insert("../unsafe".into(), String::new());
    invalids.push(invalid);
    let mut invalid = files.clone();
    invalid.insert("oversized".into(), "x".repeat(2_000_001));
    invalids.push(invalid);
    let mut invalid = files.clone();
    for i in 0..65 {
        invalid.insert(format!("extra/{i}"), String::new());
    }
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(
        &mut invalid,
        |m| m["python"] = json!({"generator":"intersect-python-api","version":7,"revision":m["revision"],"host_api":"intersect.script.v1","source":"absent.py","stub":"absent.pyi"}),
    );
    invalids.push(invalid);
    let mut invalid = files.clone();
    write_manifest(
        &mut invalid,
        |m| m["runtime"]["conversions"] = json!([{"datatype":{"source":request.grammar,"path":[]},"provider":{"name":"forged","version":"1"},"serializable":true}]),
    );
    let error = distribution::import_files(invalid).unwrap_err();
    assert!(error.contains("runtime requirements"), "{error}");
    for invalid in invalids {
        assert!(distribution::import_files(invalid).is_err());
        assert_eq!(
            distribution::import_files(files.clone())
                .unwrap()
                .manifest
                .revision,
            d.manifest.revision
        );
    }
    // Valid edits must be rebuilt as a new source archive, not redigested while
    // retaining the old revision and cached executable output.
    let mut edited = request.clone();
    let original = &edited.bundle.sources[0];
    let mut source = DefinitionSource::new(
        original.id.uri.clone(),
        SourceKind::Xtext,
        format!("{}\n// changed archive source", original.text),
    );
    source.dependencies = original.dependencies.clone();
    edited.grammar = source.id.clone();
    edited.bundle.sources[0] = source;
    let edited_distribution = distribution::build_inferred(edited.clone(), metadata()).unwrap();
    assert_ne!(edited_distribution.manifest.revision, d.manifest.revision);
    let mut invalid = files.clone();
    write_payload(
        &mut invalid,
        "definitions/inference.json",
        &serde_json::to_value(&edited).unwrap(),
    );
    assert!(
        distribution::import_files(invalid)
            .unwrap_err()
            .contains("identity mismatch")
    );
    distribution::import_files(extracted(&edited_distribution)).unwrap();
    let mut relabeled = metadata();
    relabeled.release = "2".into();
    relabeled.display_name = "Another label".into();
    assert_eq!(
        distribution::build_inferred(request.clone(), relabeled)
            .unwrap()
            .manifest
            .revision,
        d.manifest.revision
    );
    let encoding_cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-archive/source-encoding-cases.json"
    ))
    .unwrap();
    let encoding_rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-archive/source-encoding-observations.json"
    ))
    .unwrap();
    for (case, row) in encoding_cases.iter().zip(&encoding_rows) {
        assert_eq!(case["id"], row["id"]);
        let mut encoded = request.clone();
        let old = &encoded.bundle.sources[0];
        let mut source = DefinitionSource::new(
            old.id.uri.clone(),
            SourceKind::Xtext,
            case["sources"][0]["source"].as_str().unwrap(),
        );
        source.dependencies = old.dependencies.clone();
        encoded.grammar = source.id.clone();
        encoded.bundle.sources[0] = source;
        let result = distribution::build_inferred(encoded.clone(), metadata());
        assert_eq!(
            result.is_ok(),
            row["all_sources_accepted"] == true,
            "{}",
            case["id"]
        );
        if let Ok(d) = result {
            let loaded = distribution::import_files(extracted(&d)).unwrap();
            assert_eq!(
                serde_json::to_value(&loaded.package.xtext_inference).unwrap(),
                serde_json::to_value(&encoded).unwrap()
            );
        }
    }
    let wide = CompilationSourcePolicy {
        structure_bytes: 300_000,
        xtext_bytes: 300_000,
        xtext_declarations: 128,
    };
    let wide_distribution =
        distribution::build_inferred_with_source_policy(request.clone(), metadata(), wide, wide)
            .unwrap();
    assert!(distribution::import_files(extracted(&wide_distribution)).is_err());
    distribution::import_files_with_source_policy(extracted(&wide_distribution), wide).unwrap();
    assert!(
        distribution::build_inferred_with_source_policy(
            request,
            metadata(),
            wide,
            CompilationSourcePolicy::default()
        )
        .is_err()
    );
}
pub fn ordinary_compatibility() {
    let expected: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-archive/ordinary-baseline.json"
    ))
    .unwrap();
    let d = ordinary::ordinary_baseline_distribution();
    assert_eq!(
        serde_json::to_value(&d.manifest).unwrap(),
        expected["manifest"]
    );
    assert_eq!(d.files.len(), expected["files"].as_object().unwrap().len());
    for (name, text) in &d.files {
        assert_eq!(
            json!({"bytes":text.len(),"sha256":format!("{:x}",Sha256::digest(text.as_bytes()))}),
            expected["files"][name]
        );
    }
    let source = extracted(&d);
    let model = intersect_text::distribution::ordinary::import_files(source.clone()).unwrap();
    let language = distribution::import_files(source).unwrap();
    assert_eq!(
        serde_json::to_value(model.package).unwrap(),
        serde_json::to_value(language.package).unwrap()
    );
    let mut invalid = extracted(&d);
    write_manifest(&mut invalid, |m| {
        m["compiler"]["name"] = json!("intersect-text")
    });
    assert!(distribution::import_files(invalid).is_err());
}
pub fn verify() -> usize {
    integrity_and_policy();
    ordinary_compatibility();
    2
}

pub(crate) fn legacy_distribution() -> Distribution {
    ordinary::ordinary_baseline_distribution()
}
