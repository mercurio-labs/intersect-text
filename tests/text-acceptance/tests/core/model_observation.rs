use intersect_model::core::{
    model::Model,
    storage::{ModelRead, ModelView, Query, index_keys},
};
use intersect_model::definitions::conversion::{
    ConversionRegistry, InvocationContext, InvocationObserver, ProviderId,
};
use serde_json::json;
use std::{cell::Cell, rc::Rc};
struct Probe {
    work: Cell<usize>,
    stop: usize,
}
impl InvocationObserver for Probe {
    fn before_invoke(&self, _: &ProviderId) -> Result<(), String> {
        panic!("builtin model must not dispatch a provider")
    }
    fn work(&self, units: usize) -> Result<(), String> {
        if self.work.get() + units > self.stop {
            return Err("model interrupted".into());
        }
        self.work.set(self.work.get() + units);
        Ok(())
    }
}
#[test]
fn model_validation_interrupts_each_segment_without_mutating_snapshot_or_index_semantics() {
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap()
    .schema;
    let mut model = Model::default();
    for (id, class) in [
        ("library", "Library"),
        ("copy", "BookCopy"),
        ("member", "Member"),
    ] {
        let index = model
            .create(&schema, id, class, "memory:observed", 0, 0)
            .unwrap();
        model.elements[index]
            .values
            .insert("name".into(), json!(id));
    }
    model.elements[0]
        .values
        .insert("copies".into(), json!(["copy"]));
    model.elements[0]
        .values
        .insert("members".into(), json!(["member"]));
    model.elements[1]
        .values
        .insert("library".into(), json!("library"));
    model.resources =
        intersect_model::core::resources::records_for_elements(&schema, &model.elements).unwrap();
    let snapshot = model.clone();
    let registry = ConversionRegistry::default();
    let probe = Rc::new(Probe {
        work: Cell::new(0),
        stop: usize::MAX,
    });
    let observation = InvocationContext::new(probe.clone());
    model
        .validate_with_conversions_observed(&schema, &registry, &observation)
        .unwrap();
    let total = probe.work.get();
    assert!(total > 50);
    for stop in 0..total {
        let probe = Rc::new(Probe {
            work: Cell::new(0),
            stop,
        });
        let observation = InvocationContext::new(probe.clone());
        let error = model
            .validate_with_conversions_observed(&schema, &registry, &observation)
            .unwrap_err();
        assert!(error.contains("model interrupted"), "{stop}: {error}");
        assert_eq!(probe.work.get(), stop);
        assert_eq!(model, snapshot);
    }
    let exact = Rc::new(Probe {
        work: Cell::new(0),
        stop: total,
    });
    model
        .validate_with_conversions_observed(
            &schema,
            &registry,
            &InvocationContext::new(exact.clone()),
        )
        .unwrap();
    assert_eq!(exact.work.get(), total);
    model.validate(&schema).unwrap();
    let view = ModelView::new(&model).unwrap();
    for element in &model.elements {
        let keys = index_keys(element);
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            view.matches(&element.id, Query::Class(&element.class))
                .unwrap()
        );
        for (name, value) in &element.values {
            for atom in value
                .as_array()
                .map_or(std::slice::from_ref(value), Vec::as_slice)
            {
                if !atom.is_null() {
                    assert!(view.matches(&element.id, Query::Value(name, atom)).unwrap());
                }
            }
        }
    }
    // Earlier structural errors remain errors, with no subsequent polling.
    model.elements[1].id = "library".into();
    let probe = Rc::new(Probe {
        work: Cell::new(0),
        stop: usize::MAX,
    });
    let error = model
        .validate_with_conversions_observed(
            &schema,
            &registry,
            &InvocationContext::new(probe.clone()),
        )
        .unwrap_err();
    assert_eq!(error, "Duplicate or empty model identity");
    let exact = Rc::new(Probe {
        work: Cell::new(0),
        stop: probe.work.get(),
    });
    assert_eq!(
        model
            .validate_with_conversions_observed(&schema, &registry, &InvocationContext::new(exact))
            .unwrap_err(),
        error
    );
}

#[test]
fn null_atoms_and_empty_fields_each_have_an_indexing_checkpoint() {
    use intersect_model::core::model::Element;
    use std::collections::BTreeMap;
    let mut model = Model {
        schema_revision: "test".into(),
        resources: Default::default(),
        elements: vec![Element {
            implicit: false,
            initialization: None,
            assignment_state: None,
            id: "object".into(),
            class: "C".into(),
            type_identity: "C".into(),
            resource: "memory:test".into(),
            start: 0,
            end: 0,
            values: BTreeMap::new(),
        }],
    };
    let count = |model: &Model| {
        let probe = Rc::new(Probe {
            work: Cell::new(0),
            stop: usize::MAX,
        });
        let view = ModelView::new_observed(model, &InvocationContext::new(probe.clone())).unwrap();
        assert!(view.matches("object", Query::Class("C")).unwrap());
        probe.work.get()
    };
    let baseline = count(&model);
    let original_keys = index_keys(&model.elements[0]);
    let atoms = 4096;
    model.elements[0]
        .values
        .insert("nulls".into(), json!(vec![serde_json::Value::Null; atoms]));
    assert_eq!(count(&model), baseline + atoms + 1);
    for n in 0..128 {
        model.elements[0]
            .values
            .insert(format!("empty{n}"), json!([]));
    }
    let total = baseline + atoms + 1 + 128;
    assert_eq!(count(&model), total);
    assert_eq!(index_keys(&model.elements[0]), original_keys);
    for stop in [0, 1, 4, 16, 128, 256, 2048, total - 1] {
        let probe = Rc::new(Probe {
            work: Cell::new(0),
            stop,
        });
        let result = ModelView::new_observed(&model, &InvocationContext::new(probe.clone()));
        assert_eq!(result.err().expect("must stop"), "model interrupted");
        assert_eq!(probe.work.get(), stop);
    }
    let exact = Rc::new(Probe {
        work: Cell::new(0),
        stop: total,
    });
    let view = ModelView::new_observed(&model, &InvocationContext::new(exact.clone())).unwrap();
    assert_eq!(exact.work.get(), total);
    assert!(
        !view
            .matches("object", Query::Value("nulls", &serde_json::Value::Null))
            .unwrap()
    );
    assert_eq!(
        model.elements[0].values["nulls"].as_array().unwrap().len(),
        atoms
    );
}

#[test]
fn structural_values_yield_individually_without_publishing_partial_ownership() {
    use intersect_model::core::storage::StructuralPreparation;
    let schema=intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="m" nsURI="urn:wide"><eClassifiers x:type="e:EClass" name="Node"><eStructuralFeatures x:type="e:EAttribute" name="tags" upperBound="-1" unique="false" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers></e:EPackage>"##).unwrap();
    let mut model = Model::default();
    model.create(&schema, "n", "Node", "r", 0, 0).unwrap();
    model.elements[0]
        .values
        .insert("tags".into(), json!(vec!["same"; 64]));
    let registry = ConversionRegistry::default();
    let probe = Rc::new(Probe {
        work: Cell::new(0),
        stop: usize::MAX,
    });
    let observation = InvocationContext::new(probe.clone());
    let view = ModelView::new(&model).unwrap();
    let mut pending = StructuralPreparation::ownership();
    let mut slices = 0;
    let mut largest_quantum = 0;
    loop {
        let before = probe.work.get();
        let done = pending
            .advance(&view, &schema, Some(&registry), Some(&observation))
            .unwrap();
        largest_quantum = largest_quantum.max(probe.work.get() - before);
        slices += 1;
        if done {
            break;
        }
        // Each value now retains policy selection, codec and payload transitions.
        assert!(slices < 1024);
    }
    // Source/default/codec atoms now report their internal work. Pin this
    // fixture envelope below its 64-value width; do not infer a general host
    // quantum bound from it (remaining source continuations are S09 carryover).
    assert_eq!(
        largest_quantum, 48,
        "nonunique source/codec fixture quantum"
    );
    assert!(slices >= 64 * 5);
    assert!(pending.finish().unwrap().parent("n").is_none());
    drop(view);
    model.elements[0].values.get_mut("tags").unwrap()[63] = json!(17);
    let view = ModelView::new(&model).unwrap();
    let mut pending = StructuralPreparation::ownership();
    loop {
        match pending.advance(
            &view,
            &schema,
            Some(&registry),
            Some(&InvocationContext::default()),
        ) {
            Ok(false) => {}
            Ok(true) => panic!("invalid last value accepted"),
            Err(_) => break,
        }
    }
    assert!(pending.finish().is_err());
}

#[test]
fn structural_uniqueness_yields_comparison_and_cleanup_before_value_validation() {
    use intersect_model::core::storage::StructuralPreparation;
    let schema=intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="m" nsURI="urn:wide"><eClassifiers x:type="e:EClass" name="Node"><eStructuralFeatures x:type="e:EAttribute" name="tags" upperBound="-1" unique="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers></e:EPackage>"##).unwrap();
    let mut model = Model::default();
    model.create(&schema, "n", "Node", "r", 0, 0).unwrap();
    model.elements[0].values.insert(
        "tags".into(),
        json!((0..64).map(|n| format!("tag{n}")).collect::<Vec<_>>()),
    );
    let registry = ConversionRegistry::default();
    let probe = Rc::new(Probe {
        work: Cell::new(0),
        stop: usize::MAX,
    });
    let observation = InvocationContext::new(probe.clone());
    let view = ModelView::new(&model).unwrap();
    let mut pending = StructuralPreparation::ownership();
    let mut slices = 0;
    let mut largest_quantum = 0;
    loop {
        let before = probe.work.get();
        let done = pending
            .advance(&view, &schema, Some(&registry), Some(&observation))
            .unwrap();
        largest_quantum = largest_quantum.max(probe.work.get() - before);
        slices += 1;
        if done {
            break;
        }
        assert!(slices < 1152);
    }
    assert_eq!(largest_quantum, 48, "unique source/codec fixture quantum");
    assert!(slices >= 192 + 64 * 4);
    assert!(pending.finish().unwrap().parent("n").is_none());
    drop(view);
    model.elements[0].values.get_mut("tags").unwrap()[63] = json!("tag0");
    let view = ModelView::new(&model).unwrap();
    let mut pending = StructuralPreparation::ownership();
    loop {
        match pending.advance(
            &view,
            &schema,
            Some(&registry),
            Some(&InvocationContext::default()),
        ) {
            Ok(false) => {}
            Ok(true) => panic!("invalid last value accepted"),
            Err(error) => {
                assert_eq!(error, "Duplicate value: Node.tags");
                break;
            }
        }
    }
    assert!(pending.finish().is_err());
}

#[test]
fn structural_unknown_fields_yield_one_stored_field_before_rejection() {
    use intersect_model::core::storage::StructuralPreparation;
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="m" nsURI="urn:unknown"><eClassifiers x:type="e:EClass" name="Node"/></e:EPackage>"#,
    )
    .unwrap();
    let mut model = Model::default();
    model.create(&schema, "n", "Node", "r", 0, 0).unwrap();
    for index in 0..64 {
        model.elements[0]
            .values
            .insert(format!("unknown{index:02}"), json!(index));
    }
    let view = ModelView::new(&model).unwrap();
    let mut pending = StructuralPreparation::ownership();
    let mut slices = 0;
    loop {
        match pending.advance(&view, &schema, None, None) {
            Ok(false) => slices += 1,
            Ok(true) => panic!("unknown field accepted"),
            Err(error) => {
                assert_eq!(error, "Unknown feature Node.unknown00");
                break;
            }
        }
        assert!(slices < 200);
    }
    assert!(slices > 0);
    assert!(pending.finish().is_err());
}

#[test]
fn wide_known_field_prefix_yields_before_unknown_field_and_value_checks() {
    use intersect_model::core::storage::StructuralPreparation;
    let members = (0..64).map(|i| format!(r#"<eStructuralFeatures x:type="e:EAttribute" name="f{i:02}" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>"#)).collect::<String>();
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&format!(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="m" nsURI="urn:known-prefix"><eClassifiers x:type="e:EClass" name="Node">{members}</eClassifiers></e:EPackage>"#)).unwrap();
    let mut model = Model::default();
    model.create(&schema, "n", "Node", "r", 0, 0).unwrap();
    // The first value is invalid, but the header must reject the final unknown
    // field before attempting datatype validation.
    model.elements[0].values.insert("f00".into(), json!(17));
    model.elements[0]
        .values
        .insert("z-unknown".into(), json!(null));
    let view = ModelView::new(&model).unwrap();
    let probe = Rc::new(Probe {
        work: Cell::new(0),
        stop: usize::MAX,
    });
    let observation = InvocationContext::new(probe.clone());
    let mut pending = StructuralPreparation::ownership();
    let mut slices = 0;
    let mut largest_quantum = 0;
    loop {
        let before = probe.work.get();
        let result = pending.advance(&view, &schema, None, Some(&observation));
        largest_quantum = largest_quantum.max(probe.work.get() - before);
        slices += 1;
        match result {
            Ok(false) => {}
            Ok(true) => panic!("unknown field accepted"),
            Err(error) => {
                assert_eq!(error, "Unknown feature Node.z-unknown");
                break;
            }
        }
        // Inventory, member selection and cache projection also visit each field.
        assert!(slices < 1000);
    }
    // This includes metered source/member atoms, not just stored field visits.
    // The literal fixture envelope stays below the 64-field prefix width.
    assert_eq!(largest_quantum, 34, "wide source/member fixture quantum");
    assert!(slices >= 128);
    assert!(pending.finish().is_err());
}
