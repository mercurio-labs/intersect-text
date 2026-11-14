//! Source services are explicit; these model declarations contain no fake getter.
use intersect_model::{
    api::ModelEdit,
    compile_rcore_at,
    core::query::QueryBudget,
    definitions::{
        bundle::{DefinitionSource, SourceKind},
        rcore_behavior::{
            ir,
            ir_v2::{DataWire, EntrySelection, Limits, ValueWire},
            modules::SymbolId,
            types::Scalar,
        },
    },
    rcore::{ExecutionLimits, RcorePackage},
    source_model_functions::SourceModelFunction,
};
use serde_json::{Value, json};
pub const MODEL: &str = include_str!("read_model.rcore");
pub const BODY: &str = include_str!("read_function.rcore");
pub fn supplement(package: &RcorePackage, body: &str) -> DefinitionSource {
    let mut source = DefinitionSource::new(
        "memory:standalone-read.rcore",
        SourceKind::RcoreBehavior,
        body,
    );
    source.dependencies.push(package.entry().clone());
    source
}
pub fn selection(source: &DefinitionSource) -> EntrySelection {
    EntrySelection {
        symbol: SymbolId {
            source: source.id.clone(),
            declaration: 1,
        },
        types: vec![],
        arguments: vec![],
    }
}
pub fn arguments(short: bool, prefix: &str) -> Vec<DataWire> {
    vec![
        DataWire::Scalar(ir::Value {
            ty: ir::ValueType::Scalar(Scalar::Bool),
            payload: Value::Bool(short),
        }),
        DataWire::Scalar(ir::Value {
            ty: ir::ValueType::Scalar(Scalar::String),
            payload: Value::String(prefix.into()),
        }),
    ]
}
fn label(value: &ValueWire) -> Result<Option<&str>, String> {
    let DataWire::Ok(value) = &value.data else {
        return Err("Missing successful read result".into());
    };
    let DataWire::Record(fields) = &value.data else {
        return Err("Missing structured read result".into());
    };
    let value = fields.get("label").ok_or("Missing result label")?;
    match &value.data {
        DataWire::None => Ok(None),
        DataWire::Some(value) => match &value.data {
            DataWire::Scalar(value) => value
                .payload
                .as_str()
                .map(Some)
                .ok_or_else(|| "Bad result string".into()),
            _ => Err("Bad result label".into()),
        },
        _ => Err("Bad result option".into()),
    }
}
fn target(value: &ValueWire) -> Result<Option<&str>, String> {
    let DataWire::Ok(value) = &value.data else {
        return Err("Missing successful read result".into());
    };
    let DataWire::Record(fields) = &value.data else {
        return Err("Missing structured read result".into());
    };
    let value = fields.get("target").ok_or("Missing result target")?;
    match &value.data {
        DataWire::None => Ok(None),
        DataWire::Some(value) => match &value.data {
            DataWire::Model(value) => value["element"]
                .as_str()
                .map(Some)
                .ok_or_else(|| "Bad result identity".into()),
            _ => Err("Bad result target".into()),
        },
        _ => Err("Bad result target option".into()),
    }
}
pub fn verify() -> Result<(), String> {
    run().map_err(|error| error.to_string())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let package = compile_rcore_at("memory:standalone-model.rcore", MODEL)?;
    let original = package.sources().clone();
    let schema = package.schema().clone();
    let source = supplement(&package, BODY);
    let function = SourceModelFunction::from_rcore(
        &package,
        vec![source.clone()],
        &selection(&source),
        &Limits::default(),
        ExecutionLimits::default(),
        Default::default(),
    )?;
    let mut model = package.new_model()?;
    model.create("a", "Node", "memory:objects")?;
    model.create("b", "Child", "memory:objects")?;
    model.set("a", "name", json!("π😀"))?;
    model.set("a", "next", json!("b"))?;
    for short in [false, true] {
        let mut measured = QueryBudget::new(10_000_000);
        let result = function.invoke_model(&*model, "a", arguments(short, "~"), &mut measured)?;
        if short {
            assert!(
                matches!(&result.data, DataWire::Err(value) if matches!(&value.data, DataWire::Scalar(value) if value.payload == json!("~")))
            );
        } else {
            assert_eq!(label(&result)?, Some("~π😀"));
            assert_eq!(target(&result)?, Some("b"));
        }
        let mut exact = QueryBudget::new(7 + measured.reads);
        exact.charge_many(7)?;
        assert_eq!(
            function.invoke_model(&*model, "a", arguments(short, "~"), &mut exact)?,
            result
        );
        assert_eq!(exact.remaining(), 0);
        let mut denied = QueryBudget::new(7 + measured.reads - 1);
        denied.charge_many(7)?;
        assert!(
            function
                .invoke_model(&*model, "a", arguments(short, "~"), &mut denied)
                .is_err()
        );
        assert_eq!(denied.remaining(), 0);
        assert!(
            function
                .invoke_model(&*model, "a", arguments(short, "~"), &mut denied)
                .is_err()
        );
    }
    let mut work = QueryBudget::new(10_000_000);
    model.set("a", "name", json!("changed"))?;
    assert_eq!(
        label(&function.invoke_model(&*model, "a", arguments(false, "="), &mut work)?)?,
        Some("=changed")
    );
    let changed_source = supplement(&package, &BODY.replace("prefix + name", "name"));
    assert_ne!(source.id, changed_source.id);
    let changed = SourceModelFunction::from_rcore(
        &package,
        vec![changed_source.clone()],
        &selection(&changed_source),
        &Limits::default(),
        ExecutionLimits::default(),
        Default::default(),
    )?;
    assert_eq!(
        label(&changed.invoke_model(&*model, "a", arguments(false, "="), &mut work)?)?,
        Some("changed")
    );
    assert_eq!(
        label(&function.invoke_model(&*model, "a", arguments(false, "="), &mut work)?)?,
        Some("=changed")
    );
    model.set("a", "name", json!(null))?;
    model.set("a", "next", json!(null))?;
    let result = function.invoke_model(&*model, "a", arguments(false, "="), &mut work)?;
    assert_eq!(label(&result)?, None);
    assert_eq!(target(&result)?, None);
    assert!(
        function
            .invoke_model(&*model, "missing", arguments(false, "="), &mut work)
            .is_err()
    );
    assert!(
        function
            .invoke_model(&*model, "a", vec![], &mut work)
            .is_err()
    );
    assert!(
        function
            .invoke_model(
                &*model,
                "a",
                arguments(false, "=").into_iter().rev().collect(),
                &mut work
            )
            .is_err()
    );
    assert_eq!(package.sources(), &original);
    assert_eq!(package.schema(), &schema);
    let saved = package.save(&model)?;
    let (reloaded, restored) = RcorePackage::reload(&saved)?;
    let fresh_source = supplement(&reloaded, BODY);
    let fresh = SourceModelFunction::from_rcore(
        &reloaded,
        vec![fresh_source.clone()],
        &selection(&fresh_source),
        &Limits::default(),
        ExecutionLimits::default(),
        Default::default(),
    )?;
    assert_eq!(
        target(&fresh.invoke_model(&*restored, "a", arguments(false, "~"), &mut work)?)?,
        None
    );
    Ok(())
}
