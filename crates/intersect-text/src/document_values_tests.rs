//! Prospective focused controls. Cached native fields are raw observations;
//! these controls do not parse a new project or qualify original24/K01.
use super::*;
use intersect_model::definitions::conversion::{InvocationObserver, ProviderId};
use std::{cell::Cell, rc::Rc};

fn capture(
    fields: &BTreeMap<String, Value>,
    pool: &mut ValuePool,
    usage: &mut AssociationUsage,
    limits: AnalysisLimits,
) -> ParsedValues {
    let mut values = ParsedValues::new();
    for (name, value) in fields {
        values
            .capture(name, ValueView::json(value), pool, usage, limits)
            .unwrap();
    }
    values
}
#[test]
fn document_values_exact_numeric_wire_and_sharing_identity() {
    let limits = AnalysisLimits::workbench();
    let mut usage = AssociationUsage::default();
    let mut pool = ValuePool::new(&mut usage, limits).unwrap();
    let fields: BTreeMap<String, Value> =
        serde_json::from_str(r#"{"minus":-0.0,"plus":0.0,"integer":1,"float":1.0,"same":-0.0}"#)
            .unwrap();
    let values = capture(&fields, &mut pool, &mut usage, limits);
    assert_eq!(
        serde_json::to_vec(&values).unwrap(),
        serde_json::to_vec(&fields).unwrap()
    );
    assert!(Arc::ptr_eq(&values.0["minus"], &values.0["same"]));
    assert!(!Arc::ptr_eq(&values.0["minus"], &values.0["plus"]));
    assert!(!Arc::ptr_eq(&values.0["integer"], &values.0["float"]));
    assert_eq!(pool.retained_value_count(), 4);
    assert_eq!(
        values.keys().cloned().collect::<Vec<_>>(),
        fields.keys().cloned().collect::<Vec<_>>()
    );
    assert_eq!(values.get("float"), fields.get("float"));
    assert_eq!(values["minus"].to_string(), "-0.0");
}
#[test]
fn document_values_cached_actual_native_base_is_lossless_and_smaller() {
    let fixture: Value = serde_json::from_str(include_str!(
        "document_values_tests/native-base-raw-fields.json"
    ))
    .unwrap();
    assert_eq!(
        fixture["source"]["sha256"],
        "0a684334290742602432b9e8cf44203fa9b3fd61d04888863923b80223fcfd2e"
    );
    let limits = AnalysisLimits::workbench();
    let mut usage = AssociationUsage::default();
    let mut pool = ValuePool::new(&mut usage, limits).unwrap();
    let mut old_bytes = 0;
    let mut old_records = 0;
    let mut count = 0;
    for object in fixture["objects"].as_array().unwrap() {
        let fields: BTreeMap<String, Value> =
            serde_json::from_value(object["values"].clone()).unwrap();
        for (name, value) in &fields {
            let mut traversal = TraversalBudget::new(usize::MAX, 64, usize::MAX);
            let payload = traversal.inspect(value).unwrap();
            let nodes = traversal.visited_nodes();
            old_bytes += nodes * RETAINED_NODE_BYTES + payload + name.len();
            old_records += nodes;
            count += 1;
        }
        let values = capture(&fields, &mut pool, &mut usage, limits);
        assert_eq!(
            serde_json::to_vec(&values).unwrap(),
            serde_json::to_vec(&fields).unwrap()
        );
    }
    assert_eq!(count, 1356);
    assert_eq!(old_bytes, 404190);
    assert_eq!(old_records, 2611);
    assert_eq!(pool.retained_value_count(), 168);
    assert!(
        usage.bytes < old_bytes,
        "compact raw bytes {} vs old {old_bytes}",
        usage.bytes
    );
    assert!(usage.records < old_records);
    // Same exact historical native Base fields and full capture loop as the
    // executed source275 baseline, not a different fixture or limit.
    assert!(
        usage.operations < 400_905,
        "fingerprint operations{} vs baseline400905",
        usage.operations
    );
    println!(
        "CACHED_BASE_FINGERPRINT_COMPARISON previous_operations=400905 current_operations={} saved_operations={}",
        usage.operations,
        400_905 - usage.operations
    );
    println!(
        "CACHED_BASE_RAW_ASSOCIATIONS bytes={} records={} operations={}",
        usage.bytes, usage.records, usage.operations
    );
}
#[test]
fn document_values_pool_is_document_local_and_preserves_last_write() {
    let limits = AnalysisLimits::workbench();
    let mut a = AssociationUsage::default();
    let mut b = a;
    let mut first = ValuePool::new(&mut a, limits).unwrap();
    let mut second = ValuePool::new(&mut b, limits).unwrap();
    let fields = BTreeMap::from([(
        "value".into(),
        serde_json::json!({"nested":[null,false,"exact"]}),
    )]);
    let mut av = capture(&fields, &mut first, &mut a, limits);
    let bv = capture(&fields, &mut second, &mut b, limits);
    assert!(!Arc::ptr_eq(&av.0["value"], &bv.0["value"]));
    av.capture(
        "value",
        ValueView::json(&Value::Bool(true)),
        &mut first,
        &mut a,
        limits,
    )
    .unwrap();
    assert_eq!(av.len(), 1);
    assert_eq!(av["value"], Value::Bool(true));
    assert_ne!(av["value"], bv["value"]);
}
#[test]
fn document_values_byte_record_work_short_refuse_without_publication() {
    let value = serde_json::json!({"original":["payload",false,null]});
    let full = AnalysisLimits::workbench();
    let mut measured = AssociationUsage::default();
    let mut pool = ValuePool::new(&mut measured, full).unwrap();
    let mut fields = ParsedValues::new();
    fields
        .capture(
            "raw",
            ValueView::json(&value),
            &mut pool,
            &mut measured,
            full,
        )
        .unwrap();
    for limits in [
        AnalysisLimits {
            association_bytes: measured.bytes - 1,
            ..full
        },
        AnalysisLimits {
            association_records: measured.records - 1,
            ..full
        },
        AnalysisLimits {
            association_operations: measured.operations - 1,
            ..full
        },
    ] {
        let mut usage = AssociationUsage::default();
        let mut pool = ValuePool::new(&mut usage, limits).unwrap();
        let mut fields = ParsedValues::new();
        assert!(
            fields
                .capture(
                    "raw",
                    ValueView::json(&value),
                    &mut pool,
                    &mut usage,
                    limits
                )
                .is_err()
        );
        assert!(fields.is_empty());
        assert!(
            usage.bytes <= limits.association_bytes
                && usage.records <= limits.association_records
                && usage.operations <= limits.association_operations
        );
    }
}
#[test]
fn document_values_actual_canonical_store_duplicate_hit_preserves_wire_and_sharing() {
    use intersect_model::{
        definitions::dynamic::{ScalarType, SchemaBuilder},
        frozen::FrozenStore,
        model::Model,
        storage::ModelRead,
    };
    let raw = serde_json::json!({"null":null,"bool":false,"text":"?actual\u{03bb}",
        "minus":-0.0,"plus":0.0,"integer":1,"float":1.0,
        "list":[false,null,"exact"],"other":{"kind":"retained","path":[175,0]}});
    let mut builder = SchemaBuilder::new("observed", "urn:document-control");
    let row = builder.class("RawObservedControl").unwrap();
    for (name, value) in raw.as_object().unwrap() {
        let field = row.attribute(name, ScalarType::String);
        if value.is_array() {
            field.many().unique(false).build().unwrap();
        } else {
            field.optional().unwrap();
        }
    }
    let schema = builder.build().unwrap();
    let schema = schema.as_schema();
    let objects = ["memory://document#a", "memory://document#b"]
        .into_iter()
        .map(|id| {
            serde_json::json!({
        "id":id,"class":"RawObservedControl","type_identity":schema.identity("RawObservedControl"),
        "resource":"memory://document","start":0,"end":0,"values":raw.clone()})
        })
        .collect::<Vec<_>>();
    let model: Model = serde_json::from_value(
        serde_json::json!({"schema_revision":"raw-control","resources":{},"elements":objects}),
    )
    .unwrap();
    let store = FrozenStore::from_materialized(schema, &model).unwrap();
    let limits = AnalysisLimits::workbench();
    let mut usage = AssociationUsage::default();
    let mut pool = ValuePool::new(&mut usage, limits).unwrap();
    let mut captured = Vec::new();
    for object in &model.elements {
        assert!(
            store
                .inspect(&object.id, &mut |view| {
                    let mut values = ParsedValues::new();
                    for (name, value) in view.fields() {
                        values.capture(name, value, &mut pool, &mut usage, limits)?;
                    }
                    assert_eq!(
                        serde_json::to_vec(&values).unwrap(),
                        serde_json::to_vec(&object.values).unwrap()
                    );
                    captured.push(values);
                    Ok(())
                })
                .unwrap()
        );
    }
    for key in captured[0].keys() {
        assert!(Arc::ptr_eq(&captured[0].0[key], &captured[1].0[key]));
    }
    assert!(!Arc::ptr_eq(
        &captured[0].0["minus"],
        &captured[0].0["plus"]
    ));
    assert!(!Arc::ptr_eq(
        &captured[0].0["integer"],
        &captured[0].0["float"]
    ));
    assert_eq!(pool.retained_value_count(), raw.as_object().unwrap().len());
}
struct Observer {
    cancel: bool,
    storage_limit: usize,
    storage: Cell<usize>,
    work_limit: usize,
    work: Cell<usize>,
}
impl InvocationObserver for Observer {
    fn before_invoke(&self, _: &ProviderId) -> Result<(), String> {
        Err("raw export is not semantic invocation".into())
    }
    fn checkpoint(&self) -> Result<(), String> {
        if self.cancel {
            Err("raw export cancelled".into())
        } else {
            Ok(())
        }
    }
    fn work(&self, n: usize) -> Result<(), String> {
        self.checkpoint()?;
        let next = self.work.get().checked_add(n).ok_or("work overflow")?;
        if next > self.work_limit {
            return Err("raw export work limit".into());
        }
        self.work.set(next);
        Ok(())
    }
    fn retain_storage(&self, n: usize) -> Result<(), String> {
        self.checkpoint()?;
        let next = self
            .storage
            .get()
            .checked_add(n)
            .ok_or("storage overflow")?;
        if next > self.storage_limit {
            return Err("raw export storage limit".into());
        }
        self.storage.set(next);
        Ok(())
    }
}
#[test]
fn document_values_owned_export_observes_cancel_storage_and_work() {
    let limits = AnalysisLimits::workbench();
    let mut usage = AssociationUsage::default();
    let mut pool = ValuePool::new(&mut usage, limits).unwrap();
    let fields = BTreeMap::from([("exact".into(), serde_json::json!({"raw":[false,-0.0,1.0]}))]);
    let values = capture(&fields, &mut pool, &mut usage, limits);
    for (cancel, storage_limit, work_limit) in [
        (true, usize::MAX, usize::MAX),
        (false, 0, usize::MAX),
        (false, usize::MAX, 0),
    ] {
        let observer = Rc::new(Observer {
            cancel,
            storage_limit,
            storage: Cell::new(0),
            work_limit,
            work: Cell::new(0),
        });
        assert!(
            values
                .to_owned_map_observed(&InvocationContext::new(observer))
                .is_err()
        );
    }
    let observer = Rc::new(Observer {
        cancel: false,
        storage_limit: usize::MAX,
        storage: Cell::new(0),
        work_limit: usize::MAX,
        work: Cell::new(0),
    });
    let result = values
        .to_owned_map_observed(&InvocationContext::new(observer.clone()))
        .unwrap();
    assert!(observer.storage.get() > 0 && observer.work.get() > 0);
    let used_work = observer.work.get();
    for grant in [used_work - 1, used_work] {
        let bounded = Rc::new(Observer {
            cancel: false,
            storage_limit: usize::MAX,
            storage: Cell::new(0),
            work_limit: grant,
            work: Cell::new(0),
        });
        let exported = values.to_owned_map_observed(&InvocationContext::new(bounded.clone()));
        assert_eq!(exported.is_ok(), grant == used_work);
        assert!(bounded.work.get() <= grant);
    }
    assert_eq!(
        serde_json::to_vec(&result).unwrap(),
        serde_json::to_vec(&fields).unwrap()
    );
}

#[test]
fn document_values_forced_coarse_fingerprint_collision_keeps_exact_representations() {
    let limits = AnalysisLimits::workbench();
    let mut usage = AssociationUsage::default();
    let mut pool = ValuePool::new(&mut usage, limits).unwrap();
    let a = Value::String("same-head-AAAAAAAA-same-tail".into());
    let b = Value::String("same-head-BBBBBBBB-same-tail".into());
    let a_wire = serde_json::to_vec(&a).unwrap();
    let b_wire = serde_json::to_vec(&b).unwrap();
    assert_ne!(a_wire, b_wire);
    assert_eq!(a_wire.len(), b_wire.len());
    let mut identity_usage = AssociationUsage::default();
    assert_eq!(
        fingerprint(&a_wire, &mut identity_usage, limits).unwrap(),
        fingerprint(&b_wire, &mut identity_usage, limits).unwrap()
    );
    let mut fields = ParsedValues::new();
    let mut second_work = 0;
    for (name, value) in [("a", &a), ("b", &b), ("again-a", &a), ("again-b", &b)] {
        fields
            .capture(name, ValueView::json(value), &mut pool, &mut usage, limits)
            .unwrap();
        if name == "b" {
            second_work = usage.operations;
        }
    }
    assert_eq!(pool.values.len(), 1);
    assert_eq!(pool.retained_value_count(), 2);
    assert!(!Arc::ptr_eq(&fields.0["a"], &fields.0["b"]));
    assert!(Arc::ptr_eq(&fields.0["a"], &fields.0["again-a"]));
    assert!(Arc::ptr_eq(&fields.0["b"], &fields.0["again-b"]));
    assert_eq!(
        serde_json::to_vec(&fields).unwrap(),
        serde_json::to_vec(&BTreeMap::from([
            ("a", a.clone()),
            ("b", b.clone()),
            ("again-a", a),
            ("again-b", b)
        ]))
        .unwrap()
    );
    let short = AnalysisLimits {
        association_operations: second_work - 1,
        ..limits
    };
    let mut short_usage = AssociationUsage::default();
    let mut short_pool = ValuePool::new(&mut short_usage, short).unwrap();
    let mut short_fields = ParsedValues::new();
    let a = fields.get("a").unwrap();
    let b = fields.get("b").unwrap();
    short_fields
        .capture(
            "a",
            ValueView::json(a),
            &mut short_pool,
            &mut short_usage,
            short,
        )
        .unwrap();
    assert!(
        short_fields
            .capture(
                "b",
                ValueView::json(b),
                &mut short_pool,
                &mut short_usage,
                short
            )
            .is_err()
    );
    assert_eq!(short_fields.get("a"), Some(a));
    assert!(short_fields.get("b").is_none());
    assert_eq!(short_pool.retained_value_count(), 1);
    assert!(short_usage.operations <= short.association_operations);
}
