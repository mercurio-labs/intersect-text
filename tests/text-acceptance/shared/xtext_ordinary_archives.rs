//! Original ordinary parameter recipes through the shared archive verifier.
use intersect_text::grammar::Profile;
use intersect_text::package::CompilationSourcePolicy;
use intersect_text::{Source, distribution, entry::EntryLanguage};
use serde_json::{Value, json};

pub fn frozen_roundtrips() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ))
    .unwrap();
    let observations: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/observations.json"
    ))
    .unwrap();
    let mut counts = [0, 0, 0, 0];
    for (case, metadata) in cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(observations["metadata"].as_array().unwrap())
    {
        assert_eq!(case["id"], metadata["id"]);
        let inputs = super::imported_parameters::input(case);
        let archive = distribution::build_bundle_with_source_policy(
            inputs.clone(),
            super::archives::metadata(),
            Profile::NativeV57,
            CompilationSourcePolicy::default(),
            CompilationSourcePolicy::default(),
        );
        let archive = match archive {
            Ok(archive) => {
                assert_eq!(metadata["all_sources_accepted"], true, "{}", case["id"]);
                counts[0] += 1;
                archive
            }
            Err(error) => {
                assert_eq!(
                    metadata["all_sources_accepted"], false,
                    "{}: {error}",
                    case["id"]
                );
                counts[1] += 1;
                continue;
            }
        };
        assert_eq!(
            archive.manifest.definitions,
            std::collections::BTreeMap::from([("bundle".into(), "definitions/bundle.json".into())])
        );
        assert_eq!(
            serde_json::from_str::<Value>(&archive.files["definitions/bundle.json"]).unwrap(),
            serde_json::to_value(&inputs).unwrap()
        );
        assert!(archive.package.xtext_inference.is_none());
        let fresh = intersect_text::ordinary::compile(inputs, Profile::NativeV57).unwrap();
        assert_eq!(
            serde_json::to_value(&archive.package).unwrap(),
            serde_json::to_value(&fresh).unwrap()
        );
        let loaded = distribution::import_files(super::archives::extracted(&archive)).unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&archive).unwrap()
        );
        if archive.manifest.compiler.name == "intersect-text" {
            let error = intersect_text::distribution::ordinary::import_files(
                super::archives::extracted(&archive),
            )
            .unwrap_err();
            assert!(error.contains("installed language frontend"), "{error}");
        }
        let entry_language =
            EntryLanguage::prepare(&loaded.package, CompilationSourcePolicy::default()).unwrap();
        for run in observations["runs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["case"] == case["id"])
        {
            if run["entry"] != "resource" {
                super::named_entries::check(&entry_language, run);
                counts[3] += 1;
                continue;
            }
            let text = run["text"].as_str().unwrap();
            let result = intersect_text::evaluate(
                &loaded.package,
                &[Source {
                    uri: "memory:ordinary-archive.test".into(),
                    text: text.into(),
                }],
            );
            let parsed = run["exception"].is_null()
                && run["errors"].as_array().is_some_and(|e| e.is_empty());
            assert_eq!(
                result.diagnostics.is_empty(),
                parsed,
                "{}: {:?}",
                case["id"],
                result.diagnostics
            );
            super::parameters::check_resource(
                &result,
                "memory:ordinary-archive.test",
                text,
                parsed,
                if parsed {
                    run["resource_contents"].as_u64().unwrap() as usize
                } else {
                    0
                },
            );
            if run["accepted"] == true {
                assert_eq!(
                    super::fragment_runtime::model_projection(&loaded.package.schema, &result),
                    run["model"]
                );
            } else {
                assert!(result.model.elements.is_empty());
            }
            counts[2] += 1;
        }
    }
    assert_eq!(counts, [31, 17, 77, 14]);
}

pub fn integrity_and_legacy() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-parameters/cases.json"
    ))
    .unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "false_call")
        .unwrap();
    let inputs = super::imported_parameters::input(case);
    let policy = CompilationSourcePolicy::default();
    let archive = distribution::build_bundle_with_profile(
        inputs.clone(),
        super::archives::metadata(),
        Profile::NativeV57,
    )
    .unwrap();
    assert_eq!(archive.manifest.compiler.name, "intersect-text");
    assert!(archive.manifest.runtime.source_policy.is_none());
    let files = super::archives::extracted(&archive);
    distribution::import_files(files.clone()).unwrap();
    let explicit = distribution::build_bundle_with_source_policy(
        inputs.clone(),
        super::archives::metadata(),
        Profile::NativeV57,
        policy,
        policy,
    )
    .unwrap();
    assert_ne!(explicit.manifest.revision, archive.manifest.revision);
    assert!(
        distribution::build_bundle_with_profile(
            inputs.clone(),
            super::archives::metadata(),
            Profile::NativeV56
        )
        .is_err()
    );
    assert!(
        distribution::import_files_with_source_policy(
            files.clone(),
            CompilationSourcePolicy {
                xtext_bytes: policy.xtext_bytes - 1,
                ..policy
            }
        )
        .is_err()
    );
    let mut corruptions = vec![];
    let mut invalid = files.clone();
    invalid.get_mut("package.json").unwrap().push(' ');
    corruptions.push(invalid);
    let mut invalid = files.clone();
    let mut cached: Value = serde_json::from_str(&invalid["package.json"]).unwrap();
    cached["grammar"]["entry"] = json!("FORGED");
    super::archives::write_payload(&mut invalid, "package.json", &cached);
    corruptions.push(invalid);
    let mut invalid = files.clone();
    let mut retained: Value = serde_json::from_str(&invalid["definitions/bundle.json"]).unwrap();
    retained["bundle"]["sources"][0]["text"] = json!("changed without source identity");
    super::archives::write_payload(&mut invalid, "definitions/bundle.json", &retained);
    corruptions.push(invalid);
    for field in ["name", "version"] {
        let mut invalid = files.clone();
        super::archives::write_manifest(&mut invalid, |m| {
            m["compiler"][field] = json!(if field == "name" {
                "intersect-model"
            } else {
                "forged-version"
            })
        });
        corruptions.push(invalid);
    }
    let mut invalid = files.clone();
    super::archives::write_manifest(&mut invalid, |m| {
        m["definitions"] = json!({"inference":"definitions/bundle.json"})
    });
    corruptions.push(invalid);
    let mut invalid = files.clone();
    super::archives::write_manifest(&mut invalid, |m| {
        m["runtime"]["profile"] = serde_json::to_value(Profile::NativeV56).unwrap()
    });
    corruptions.push(invalid);
    for invalid in corruptions {
        assert!(distribution::import_files(invalid).is_err());
    }
    let recovered = distribution::import_files(files).unwrap();
    assert_eq!(
        serde_json::to_value(&recovered.package).unwrap(),
        serde_json::to_value(&archive.package).unwrap()
    );
    // The ordinary compatibility corpus must also round-trip through the newly
    // installed producer, with every legacy manifest/payload byte preserved.
    let legacy: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-archive/ordinary-baseline.json"
    ))
    .unwrap();
    // Existing compatibility control verifies exact file digests and both importers.
    super::archives::ordinary_compatibility();
    let ordinary = super::archives::legacy_distribution();
    let rebuilt = distribution::build_bundle_with_profile(
        ordinary.package.compilation.clone().unwrap(),
        ordinary.manifest.metadata.clone(),
        ordinary.package.grammar.as_ref().unwrap().profile,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&rebuilt.manifest).unwrap(),
        legacy["manifest"]
    );
    assert_eq!(rebuilt.files, ordinary.files);
}
pub fn verify() -> usize {
    frozen_roundtrips();
    integrity_and_legacy();
    2
}
