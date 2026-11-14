#[path = "support/compatibility.rs"]
mod corpus;
use corpus::{normalized, registry};
use intersect_model::core::{
    dynamic::DynamicModel,
    interchange::{ModelFormat, Resource},
};
use intersect_model::definitions::dynamic::{ScalarType, SchemaBuilder};
use intersect_model::xmi::XmiFormat;
use serde_json::json;
fn upstream(name: &str) -> Resource {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/compatibility/upstream/emf")
        .join(name);
    Resource {
        uri: name.into(),
        bytes: std::fs::read(path).unwrap(),
    }
}

#[test]
fn unmodified_emf_john_mary_match_dynamic_instances_in_both_resource_orders() {
    let mut b = SchemaBuilder::new("pack", "http://www.eclipse.org/emf/pack/person");
    let person = b.class("Person").unwrap();
    let name = person
        .attribute("name", ScalarType::String)
        .optional()
        .unwrap();
    let father = person.reference("father", &person).optional().unwrap();
    let schema = b.build().unwrap();
    let mut dynamic = DynamicModel::new(schema.clone()).unwrap();
    dynamic
        .transaction(|tx| {
            let john = tx.create(&person, "john", "john.xmi")?;
            let mary = tx.create(&person, "mary", "mary.xmi")?;
            tx.set(&john, &name, "John")?;
            tx.set(&mary, &name, "Mary")?;
            tx.set(&mary, &father, &john)
        })
        .unwrap();
    for names in [["john.xmi", "mary.xmi"], ["mary.xmi", "john.xmi"]] {
        let resources = names.map(upstream);
        let model = registry()
            .import("xmi", schema.as_schema(), &resources)
            .unwrap();
        assert_eq!(
            normalized(schema.as_schema(), &model),
            normalized(schema.as_schema(), &dynamic.to_model().unwrap())
        );
        assert_eq!(
            model
                .elements
                .iter()
                .find(|e| e.values["name"] == "Mary")
                .unwrap()
                .values["father"],
            json!("john.xmi#/")
        );
        assert_eq!(
            registry()
                .import(
                    "xmi",
                    schema.as_schema(),
                    &registry()
                        .export("xmi", schema.as_schema(), &model)
                        .unwrap()
                )
                .unwrap(),
            model
        );
    }
    assert!(
        registry()
            .import("xmi", schema.as_schema(), &[upstream("mary.xmi")])
            .unwrap_err()
            .contains("Unresolved")
    );
}

#[test]
fn unmodified_emf_family_matches_dynamic_containment() {
    let mut b = SchemaBuilder::new(
        "pack",
        "http://www.eclipse.org/emf/test.core.change/testLoadChangeDescritpion",
    );
    let family = b.class("Family").unwrap();
    let person = b.class("Person").unwrap();
    let name = person
        .attribute("name", ScalarType::String)
        .optional()
        .unwrap();
    let spouse1 = family.reference("spouse1", &person).containment().unwrap();
    let spouse2 = family.reference("spouse2", &person).containment().unwrap();
    let children = family
        .reference("children", &person)
        .many()
        .containment()
        .unwrap();
    let schema = b.build().unwrap();
    let mut dynamic = DynamicModel::new(schema.clone()).unwrap();
    dynamic
        .transaction(|tx| {
            let f = tx.create(&family, "f", "family.xmi")?;
            for (n, feature, many) in [
                ("John", &spouse1, false),
                ("Lisa", &spouse2, false),
                ("Paul", &children, true),
                ("Anna", &children, true),
            ] {
                let p = tx.create(&person, n, "family.xmi")?;
                tx.set(&p, &name, n)?;
                if many {
                    tx.add(&f, feature, &p)?;
                } else {
                    tx.set(&f, feature, &p)?;
                }
            }
            Ok(())
        })
        .unwrap();
    let imported = registry()
        .import("xmi", schema.as_schema(), &[upstream("family.xmi")])
        .unwrap();
    assert_eq!(imported.elements.len(), 5);
    assert_eq!(
        normalized(schema.as_schema(), &imported),
        normalized(schema.as_schema(), &dynamic.to_model().unwrap())
    );
    assert_eq!(
        registry()
            .import(
                "xmi",
                schema.as_schema(),
                &registry()
                    .export("xmi", schema.as_schema(), &imported)
                    .unwrap()
            )
            .unwrap(),
        imported
    );
}

#[test]
fn unmodified_emf_xrefs_stored_projection_preserves_every_edge() {
    // Full upstream metamodel is deliberately tested as unsupported below.
    let mut b = SchemaBuilder::new(
        "xrefsmodel",
        "http://www.eclipse.org/EMF/2014/test/xrefsmodel",
    );
    let a = b.class("A").unwrap();
    let name = a.attribute("name", ScalarType::String).required().unwrap();
    let others = a.reference("others", &a).many().build().unwrap();
    let non = a.reference("nonOthers", &a).many().build().unwrap();
    let schema = b.build().unwrap();
    let mut dynamic = DynamicModel::new(schema.clone()).unwrap();
    dynamic
        .transaction(|tx| {
            let mut objects = vec![];
            for i in 0..7 {
                let o = tx.create(
                    &a,
                    &format!("a{i}"),
                    if [0, 5, 6].contains(&i) {
                        "xrefs0.xmi"
                    } else {
                        "xrefs1.xmi"
                    },
                )?;
                tx.set(&o, &name, format!("a{i}"))?;
                objects.push(o);
            }
            for (from, to) in [(0, 1), (0, 2), (1, 3), (1, 4), (2, 5), (2, 6), (6, 1)] {
                tx.add(&objects[from], &others, &objects[to])?;
            }
            tx.add(&objects[0], &non, &objects[3])
        })
        .unwrap();
    let mut resources = vec![upstream("xrefs0.xmi"), upstream("xrefs1.xmi")];
    for _ in 0..2 {
        let imported = registry()
            .import("xmi", schema.as_schema(), &resources)
            .unwrap();
        assert_eq!(imported.elements.len(), 7);
        assert_eq!(
            normalized(schema.as_schema(), &imported),
            normalized(schema.as_schema(), &dynamic.to_model().unwrap())
        );
        let saved = registry()
            .export("xmi", schema.as_schema(), &imported)
            .unwrap();
        assert_eq!(
            registry()
                .import("xmi", schema.as_schema(), &saved)
                .unwrap(),
            imported
        );
        resources.reverse();
    }
}

#[test]
fn unchanged_upstream_metamodels_and_xtext_grammars_fail_explicitly() {
    for (name, diagnostic) in [
        ("Dynamic.ecore", "iD"),
        (
            "proxy.ecore",
            "Missing resource or undeclared exact dependency",
        ),
        ("xrefsopposite.ecore", "iD"),
    ] {
        let resource = upstream(name);
        let source = std::str::from_utf8(&resource.bytes).unwrap();
        let error =
            intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(source)
                .unwrap_err();
        assert!(error.contains(diagnostic), "{name}: {error}");
    }
    for name in ["Domainmodel.xtext", "Statemachine.xtext"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/compatibility/upstream/xtext")
            .join(name);
        let text = std::fs::read_to_string(path).unwrap();
        let error = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            corpus::ECORE,
            &text,
            corpus::RCORE_BEHAVIOR,
        )
        .unwrap_err();
        assert!(
            error.iter().any(|e| e.source == "memory:language.xtext"),
            "{name}: {error:?}"
        );
    }
}

#[test]
fn unchanged_upstream_library_enum_defaults_round_trip_through_xmi() {
    let resource = upstream("library.ecore");
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        std::str::from_utf8(&resource.bytes).unwrap(),
    )
    .unwrap();
    let category = &schema.enumerations["BookCategory"];
    let expected = serde_json::to_value(category.parse("Mystery").unwrap()).unwrap();
    let mut book =
        intersect_model::core::model::Element::new(&schema, "book", "Book", "library.xmi", 0, 0)
            .unwrap();
    assert_eq!(book.values["category"], expected);
    let mut writer = intersect_model::core::model::Element::new(
        &schema,
        "writer",
        "Writer",
        "library.xmi",
        1,
        1,
    )
    .unwrap();
    book.values.insert("author".into(), json!("writer"));
    writer.values.insert("books".into(), json!(["book"]));
    let mut model = intersect_model::core::model::Model {
        schema_revision: schema.revision.clone(),
        resources: Default::default(),
        elements: vec![book, writer],
    };
    model.resources =
        intersect_model::core::resources::records_for_elements(&schema, &model.elements).unwrap();
    let saved = registry().export("xmi", &schema, &model).unwrap();
    assert!(
        String::from_utf8(saved[0].bytes.clone())
            .unwrap()
            .contains("category=\"Mystery\"")
    );
    assert_eq!(registry().import("xmi", &schema, &saved).unwrap(), model);
}

#[test]
fn unchanged_simple_library_transient_container_roundtrips_through_persistent_opposite() {
    let resource = upstream("SimpleLib.ecore");
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        std::str::from_utf8(&resource.bytes).unwrap(),
    )
    .unwrap();
    assert!(schema.features("Person").unwrap()["library"].transient);
    let mut model = intersect_model::core::model::Model::default();
    model
        .create(&schema, "library", "Library", "model", 0, 0)
        .unwrap();
    model
        .create(&schema, "person", "Person", "model", 0, 0)
        .unwrap();
    model
        .create(&schema, "address", "Address", "model", 0, 0)
        .unwrap();
    model
        .create(&schema, "cafeteria", "Cafeteria", "model", 0, 0)
        .unwrap();
    model
        .edit(
            &schema,
            &[
                ("library".into(), "writers".into(), json!(["person"])),
                ("library".into(), "address".into(), json!("address")),
                ("library".into(), "cafeteria".into(), json!("cafeteria")),
            ],
        )
        .unwrap();
    assert_eq!(
        XmiFormat
            .decode(&schema, &XmiFormat.encode(&schema, &model).unwrap())
            .unwrap(),
        model
    );
}

#[test]
fn unchanged_xrefs_import_preserves_computed_flags_without_inventing_a_provider() {
    let resource = upstream("xrefsmodel.ecore");
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        std::str::from_utf8(&resource.bytes).unwrap(),
    )
    .unwrap();
    let fields = schema.features("A").unwrap();
    let computed = &fields["allOthers"];
    assert!(computed.derived && computed.volatile && computed.transient);
    assert!(!computed.changeable);
    assert!(!fields["others"].volatile);
    let mut model = intersect_model::core::api::BasicModel::new(schema).unwrap();
    use intersect_model::core::api::ModelEdit;
    model.create("a", "A", "model").unwrap();
    assert!(
        model
            .is_set("a", "allOthers")
            .unwrap_err()
            .contains("semantic provider")
    );
    assert!(model.set("a", "allOthers", serde_json::json!([])).is_err());
}
