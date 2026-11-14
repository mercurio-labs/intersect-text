//! Focused native controls for explicit resource persistence; no I/O or Java.
use super::{doc, library, package, registry, resource};
use intersect_model::core::{
    interchange::{ModelFormat, ReferenceClosure},
    model::{Element, Model},
    resources::{ResourceRecord, SourceCorrespondence},
};
use intersect_model::xmi::XmiFormat;
use std::sync::Arc;

#[test]
fn empty_resources_and_explicit_root_order_round_trip() {
    let schema = package().schema;
    let formats = registry();
    let mut model = formats.import("xmi", &schema, &[resource("nested/catalog.xmi", &doc(r#"<l:Member xmi:id="first" name="Ada"/><l:Member xmi:id="second" name="Grace"/>"#))]).unwrap();
    let record = model.resources.get_mut("nested/catalog.xmi").unwrap();
    record.roots = Arc::new(record.roots.iter().cloned().rev().collect());
    record.content_revision = 9;
    record.diagnostics = vec!["retained diagnostic".into()];
    record.source = Some(SourceCorrespondence {
        uri: "source.library".into(),
        revision: "source-revision-9".into(),
    });
    model.resources.insert(
        "empty.xmi".into(),
        ResourceRecord::new("empty.xmi").unwrap(),
    );
    let encoded = formats
        .export_selected(
            "xmi",
            &schema,
            &model,
            &model.resources.keys().cloned().collect::<Vec<_>>(),
            ReferenceClosure::SelectedOnly,
        )
        .unwrap();
    assert_eq!(encoded.len(), 2);
    assert!(
        encoded
            .iter()
            .find(|r| r.uri == "empty.xmi")
            .unwrap()
            .bytes
            .windows(14)
            .any(|bytes| bytes == b"resourceRecord")
    );
    assert_eq!(formats.import("xmi", &schema, &encoded).unwrap(), model);
}

#[test]
fn selected_resource_reloads_against_retained_targets_and_closure_is_explicit() {
    let schema = package().schema;
    let formats = registry();
    let model = library();
    let loans = "file:///models/loans.library".to_string();
    let catalog = "file:///models/catalog.library".to_string();
    let encoded = formats
        .export_selected(
            "xmi",
            &schema,
            &model,
            std::slice::from_ref(&loans),
            ReferenceClosure::SelectedOnly,
        )
        .unwrap();
    assert_eq!(encoded.len(), 1);
    assert_eq!(encoded[0].uri, loans);
    assert!(formats.import("xmi", &schema, &encoded).is_err());
    let mut retained = model.clone();
    retained.elements.retain(|e| e.resource != loans);
    retained.resources.remove(&loans);
    assert_eq!(
        formats
            .import_with_context("xmi", &schema, &encoded, &retained)
            .unwrap(),
        model
    );
    let closure = formats
        .export_selected(
            "xmi",
            &schema,
            &model,
            std::slice::from_ref(&loans),
            ReferenceClosure::IncludeDependencies,
        )
        .unwrap();
    assert_eq!(closure.len(), 2);
    assert!(closure.iter().any(|r| r.uri == catalog));
    assert_eq!(formats.import("xmi", &schema, &closure).unwrap(), model);
    assert!(
        formats
            .import_with_context("xmi", &schema, &encoded, &model)
            .is_err()
    );
    assert_eq!(retained.resources.len(), 1);
}

#[test]
fn selected_hrefs_survive_unrelated_insertions_and_missing_targets_reject() {
    let schema = package().schema;
    let formats = registry();
    let mut model = library();
    let loans = vec!["file:///models/loans.library".to_string()];
    let before = formats
        .export_selected(
            "xmi",
            &schema,
            &model,
            &loans,
            ReferenceClosure::SelectedOnly,
        )
        .unwrap();
    let mut extra = Element::new(&schema, "unrelated", "Member", "a/unrelated.xmi", 0, 0).unwrap();
    extra.values.insert("name".into(), "Unrelated".into());
    model.elements.insert(0, extra);
    let mut record = ResourceRecord::new("a/unrelated.xmi").unwrap();
    record.roots = Arc::new(vec!["unrelated".into()]);
    model.resources.insert(record.uri.clone(), record);
    let after = formats
        .export_selected(
            "xmi",
            &schema,
            &model,
            &loans,
            ReferenceClosure::SelectedOnly,
        )
        .unwrap();
    let hrefs = |bytes: &[u8]| {
        String::from_utf8(bytes.to_vec())
            .unwrap()
            .lines()
            .filter(|line| line.contains("href="))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(hrefs(&before[0].bytes), hrefs(&after[0].bytes));
    let retained = Model {
        schema_revision: schema.revision.clone(),
        resources: Default::default(),
        ..Model::default()
    };
    assert!(
        formats
            .import_with_context("xmi", &schema, &after, &retained)
            .is_err()
    );
    let mut corrupt = after;
    corrupt[0].bytes = String::from_utf8(corrupt[0].bytes.clone())
        .unwrap()
        .replace("<m:Library", "<m:Unknown")
        .replace("</m:Library>", "</m:Unknown>")
        .into_bytes();
    assert!(formats.import("xmi", &schema, &corrupt).is_err());
    assert_eq!(model.resources.len(), 3);
}

#[test]
fn codecs_must_explicitly_support_retained_context_and_selection() {
    let schema = package().schema;
    let model = library();
    let codec = super::JsonFormat {
        extension: &["test"],
    };
    assert!(
        codec
            .decode_with_context(&schema, &[], &model)
            .unwrap_err()
            .contains("does not support retained resources")
    );
    assert!(
        codec
            .encode_selected(&schema, &model, &[])
            .unwrap_err()
            .contains("does not support resource selection")
    );
    assert!(
        XmiFormat
            .encode_selected(&schema, &model, &["missing.xmi".into()])
            .is_err()
    );
}

fn library_at(catalog: &str, loans: &str) -> Model {
    let mut model = library();
    for (old, new) in [
        ("file:///models/catalog.library", catalog),
        ("file:///models/loans.library", loans),
    ] {
        let mut record = model.resources.remove(old).unwrap();
        record.uri = new.into();
        model.resources.insert(new.into(), record);
        for element in &mut model.elements {
            if element.resource == old {
                element.resource = new.into();
            }
        }
    }
    model
}

#[test]
fn selected_hrefs_preserve_relative_colons_and_filename_directory_boundaries() {
    let schema = package().schema;
    let formats = registry();
    for (loans, catalog, expected) in [
        ("a/source.xmi", "b/folder/x:y", "../b/folder/x:y#"),
        ("models/loans.xmi", "models/catalog:xmi", "./catalog:xmi#"),
        ("catalog/loans.xmi", "catalog", "../catalog#"),
        (
            "https://example/source.xmi",
            "https://other",
            "https://other#",
        ),
    ] {
        let model = library_at(catalog, loans);
        let encoded = formats
            .export_selected(
                "xmi",
                &schema,
                &model,
                &[loans.into()],
                ReferenceClosure::SelectedOnly,
            )
            .unwrap();
        let text = std::str::from_utf8(&encoded[0].bytes).unwrap();
        assert!(
            text.contains(&format!("href=\"{expected}")),
            "source={loans} target={catalog}: {text}"
        );
        let mut retained = model.clone();
        retained
            .elements
            .retain(|element| element.resource != loans);
        retained.resources.remove(loans);
        let actual = formats
            .import_with_context("xmi", &schema, &encoded, &retained)
            .unwrap();
        assert_eq!(actual, model, "source={loans} target={catalog}");
    }
}

#[test]
fn unrepresentable_absolute_source_to_relative_target_rejects_before_emitting_bytes() {
    let schema = package().schema;
    let formats = registry();
    for catalog in ["/catalog.xmi", "relative/catalog.xmi"] {
        let model = library_at(catalog, "https://example/source.xmi");
        assert!(
            formats
                .export_selected(
                    "xmi",
                    &schema,
                    &model,
                    &["https://example/source.xmi".into()],
                    ReferenceClosure::SelectedOnly
                )
                .is_err()
        );
    }
}
