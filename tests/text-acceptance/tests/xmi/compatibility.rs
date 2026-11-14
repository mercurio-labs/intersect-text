#[path = "support/compatibility.rs"]
mod corpus;
use corpus::*;
use intersect_model::core::{
    dynamic::DynamicModel,
    interchange::{ModelFormat, Resource},
};
use intersect_model::xmi::XmiFormat;
use intersect_text::distribution::ordinary as distribution;
use serde_json::json;

#[test]
fn dynamic_imported_text_and_xmi_matrix() {
    let original = distribution();
    let imported = distribution::import_files(extracted(&original)).unwrap();
    assert_eq!(
        serde_json::to_value(&original.package).unwrap(),
        serde_json::to_value(&imported.package).unwrap()
    );
    for seed in 0..32 {
        let c = case(seed, &imported);
        let schema = c.dynamic.schema().clone();
        let expected = c.dynamic.to_model().unwrap();
        let mut resources = c.dynamic.export(&registry(), "xmi").unwrap();
        let again = c.dynamic.export(&registry(), "xmi").unwrap();
        assert_eq!(resources, again);
        for _ in 0..3 {
            resources.reverse();
            let restored =
                DynamicModel::import(schema.clone(), &registry(), "xmi", &resources).unwrap();
            assert_eq!(
                restored.to_model().unwrap(),
                expected,
                "native identity/provenance seed {seed}"
            );
            resources = restored.export(&registry(), "xmi").unwrap();
        }
        let text_xmi = registry()
            .export("xmi", &imported.package.schema, &c.text)
            .unwrap();
        let text_again = registry()
            .import("xmi", &imported.package.schema, &text_xmi)
            .unwrap();
        assert_eq!(text_again, c.text);
        assert_eq!(
            normalized(&imported.package.schema, &text_again),
            normalized(schema.as_schema(), &expected)
        );
        // Equal definitions do not authorize crossing different revision identities.
        assert!(
            registry()
                .import("xmi", schema.as_schema(), &text_xmi)
                .unwrap_err()
                .contains("revision")
        );
        assert_eq!(
            c.text
                .elements
                .iter()
                .map(|e| (&e.resource, &e.class, e.values["name"].as_str().unwrap()))
                .collect::<std::collections::BTreeSet<_>>(),
            expected
                .elements
                .iter()
                .map(|e| (&e.resource, &e.class, e.values["name"].as_str().unwrap()))
                .collect()
        );
    }
}

#[test]
fn xmi_scalar_null_empty_boundary_and_collection_values_roundtrip() {
    let d = distribution();
    let mut c = case(3, &d);
    let node = c.dynamic.schema().class("Node").unwrap();
    let n = c.dynamic.object("node0").unwrap();
    for value in [i32::MIN, -1, 0, i32::MAX] {
        c.dynamic
            .transaction(|tx| {
                tx.set(&n, &node.feature("count")?, value)?;
                tx.set(&n, &node.feature("active")?, false)?;
                Ok(())
            })
            .unwrap();
        let schema = c.dynamic.schema().clone();
        let bytes = c.dynamic.export(&registry(), "xmi").unwrap();
        let restored = DynamicModel::import(schema, &registry(), "xmi", &bytes).unwrap();
        assert_eq!(restored.to_model().unwrap(), c.dynamic.to_model().unwrap());
    }
    let baseline = c.dynamic.to_json().unwrap();
    assert!(
        c.dynamic
            .transaction(|tx| tx.set(&n, &node.feature("count")?, json!(2147483648_i64)))
            .is_err()
    );
    assert_eq!(c.dynamic.to_json().unwrap(), baseline);
}

#[test]
fn malformed_xmi_is_rejected_without_panics_or_partial_publication() {
    let d = distribution();
    let c = case(4, &d);
    let schema = c.dynamic.schema().clone();
    let resources = c.dynamic.export(&registry(), "xmi").unwrap();
    for cut in (0..resources[0].bytes.len()).step_by(13) {
        let mut damaged = resources.clone();
        damaged[0].bytes.truncate(cut);
        assert!(
            DynamicModel::import(schema.clone(), &registry(), "xmi", &damaged).is_err(),
            "cut {cut}"
        );
    }
    let original = String::from_utf8(resources[0].bytes.clone()).unwrap();
    for (from, to) in [
        ("xmi:version=\"2.0\"", "xmi:version=\"2.5\""),
        ("xmi:id=\"e0\"", "xmi:id=\"bad id\""),
        ("xmlns:m=", "xmlns:foreign="),
        ("schemaRevision=\"", "unknown=\""),
    ] {
        if !original.contains(from) {
            continue;
        }
        let mut damaged = resources.clone();
        damaged[0].bytes = original.replace(from, to).into_bytes();
        assert!(
            registry()
                .import("xmi", schema.as_schema(), &damaged)
                .is_err(),
            "{from}"
        );
    }
    assert!(
        registry()
            .import("xmi", schema.as_schema(), &resources[..1])
            .is_err()
    );
    let mut duplicate = resources.clone();
    duplicate.push(resources[0].clone());
    assert!(
        registry()
            .import("xmi", schema.as_schema(), &duplicate)
            .is_err()
    );
}

#[test]
fn xtext_rejections_and_justified_profile_differences_are_explicit() {
    let d = distribution();
    for replacement in [
        "'node' name=ID | 'node' name=ID",
        "{g::Node} 'node' name=ID",
        "'node' name=ID active?='active'",
        "'node' name=ID labels+=STRING?",
    ] {
        let xtext=XTEXT.replace("'node' name=ID ('count' count=INT)? ('label' labels+=STRING)* ('peer' peer=[g::Node|ID])? ('link' links+=[g::Node|ID])* ';'",replacement);
        // Last variant is supported: optional STRING has disjoint prediction.
        if replacement.ends_with("STRING?") {
            assert!(
                intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
                    ECORE,
                    &xtext,
                    RCORE_BEHAVIOR
                )
                .is_ok()
            );
        } else {
            assert!(
                intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
                    ECORE,
                    &xtext,
                    RCORE_BEHAVIOR
                )
                .is_err(),
                "{replacement}"
            );
        }
    }
    for text in [
        "graph G { node ^node; }",
        "graph G { node n count -1; }",
        "graph G { node n count 2147483648; }",
        "graph G { node n peer missing; }",
        "graph G { node n; node n; edge e n to n; }",
    ] {
        let result = intersect_text::evaluate(
            &d.package,
            &[intersect_text::Source {
                uri: "case.graph".into(),
                text: text.into(),
            }],
        );
        assert!(!result.diagnostics.is_empty(), "{text}");
    }
}

#[test]
fn ascii_encoding_and_foreign_feature_namespace_guards() {
    let s = schema();
    let xml = r#"<?xml version="1.0" encoding="ASCII"?><Graph xmlns="urn:intersect:compatibility:graph" name="G"><nodes name="n"/></Graph>"#;
    let resource = |text: String| Resource {
        uri: "test.xmi".into(),
        bytes: text.into_bytes(),
    };
    XmiFormat
        .decode(s.as_schema(), &[resource(xml.into())])
        .unwrap();
    assert!(
        XmiFormat
            .decode(
                s.as_schema(),
                &[resource(xml.replace("name=\"n\"", "name=\"雪\""))]
            )
            .is_err()
    );
    assert!(
        XmiFormat
            .decode(
                s.as_schema(),
                &[resource(
                    xml.replace("<nodes ", "<nodes xmlns=\"urn:foreign\" ")
                )]
            )
            .is_err()
    );
}

#[test]
fn cached_independent_emf_observations_and_plain_xmi_match_native_graphs() {
    let d = distribution();
    for seed in 0..8 {
        let c = case(seed, &d);
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../fixtures/compatibility/emf-oracle/seed-{seed}"
        ));
        let observed: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("normalized.json")).unwrap())
                .unwrap();
        assert_eq!(observed, normalized(&d.package.schema, &c.text));
        assert_eq!(
            observed,
            normalized(
                c.dynamic.schema().as_schema(),
                &c.dynamic.to_model().unwrap()
            )
        );
        let resources = ["resource-0000.xmi", "resource-0001.xmi"].map(|name| {
            let bytes = std::fs::read(directory.join(name)).unwrap();
            assert!(
                !std::str::from_utf8(&bytes)
                    .unwrap()
                    .contains("xmi:Extension")
            );
            Resource {
                uri: name.into(),
                bytes,
            }
        });
        for schema in [&d.package.schema, c.dynamic.schema().as_schema()] {
            let imported = registry().import("xmi", schema, &resources).unwrap();
            assert_eq!(normalized(schema, &imported), observed);
        }
    }
}
