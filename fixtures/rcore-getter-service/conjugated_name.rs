//! The authored language body through the public scalar adapter, native and WASM.
//! Graph/receiver selection remains covered by the language-package controls.
use intersect_model::{
    core::query::QueryBudget,
    definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        rcore_behavior::{ir_v2::EntrySelection, modules::SymbolId},
    },
    source_scalar_reads::SourceScalarRead,
};
use serde_json::{Value, json};

pub fn verify() -> Result<(), String> {
    let source = DefinitionSource::new(
        "intersect-kerml:/semantics/conjugated-port-name.rcore",
        SourceKind::RcoreBehavior,
        include_str!("conjugated-port-name.rcore"),
    );
    let entry = source.id.clone();
    let program = SourceScalarRead::compile(
        DefinitionBundle {
            version: 1,
            sources: vec![source],
            namespaces: vec![],
        },
        &entry,
        &EntrySelection {
            symbol: SymbolId {
                source: entry.clone(),
                declaration: 0,
            },
            types: vec![],
            arguments: vec![],
        },
    )?;
    for (input, expected) in [
        (Value::Null, Value::Null),
        (json!(""), json!("~")),
        (json!("Port"), json!("~Port")),
        (json!("π😀"), json!("~π😀")),
    ] {
        let mut measured = QueryBudget::new(1_000_000);
        if program.invoke(input.clone(), &mut measured)? != expected {
            return Err("Authored conjugated name changed".into());
        }
        let mut exact = QueryBudget::new(7 + measured.reads);
        exact.charge_many(7)?;
        if program.invoke(input.clone(), &mut exact)? != expected || exact.remaining() != 0 {
            return Err("Authored conjugated name replaced its caller ledger".into());
        }
        let mut short = QueryBudget::new(measured.reads - 1);
        if program.invoke(input.clone(), &mut short).is_ok()
            || program.invoke(input, &mut short).is_ok()
        {
            return Err("Short authored conjugated-name invocation produced a result".into());
        }
    }
    Ok(())
}
