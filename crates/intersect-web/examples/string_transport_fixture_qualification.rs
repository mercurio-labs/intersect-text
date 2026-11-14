//! Verification-only native core model construction; no production endpoint.
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::{ConversionRegistry, ReferenceContext},
    rcore_behavior::{
        function_lowering::compile_function,
        ir_v2::{DataWire, EntrySelection, Limits, TypeNode, ValueWire},
        modules::SymbolId,
    },
};
use intersect_model::rcore_behavior_functions::ExecutionLimits;
use serde_json::{Value, json};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let target = std::env::args().nth(1).ok_or("Missing new fixture path")?;
    let unit = ['"', '\\', '\n', '\t', '\0', '\u{e9}', '\u{1f600}']
        .into_iter()
        .collect::<String>();
    let text = unit.repeat(8192);
    assert_eq!(text.len(), 90112);
    assert_eq!(text.encode_utf16().count(), 65536);
    let structure = DefinitionSource::new(
        "memory:s09-model-payload.rcore",
        SourceKind::Rcore,
        "rcore 1; package modelPayload version \"1\"; metamodel \"urn:s09:model-context\" prefix m { class Holder { attr text: String [1,1]; } }",
    );
    let model_bundle = DefinitionBundle {
        version: 1,
        sources: vec![structure.clone()],
        namespaces: vec![],
    };
    let schema = intersect_model::definitions::compiler::compile_structure_bundle_with_conversions(
        &model_bundle,
        &structure.id,
        "urn:s09:model-context",
        &ConversionRegistry::default(),
    )?;
    let mut model = intersect_model::core::model::Model::default();
    model.create(
        &schema,
        "holder",
        "Holder",
        "memory:model-payload-unicode",
        0,
        0,
    )?;
    model.edit(
        &schema,
        &[("holder".into(), "text".into(), Value::String(text.clone()))],
    )?;
    model.validate(&schema)?;
    let model_value = serde_json::to_value(&model)?;
    let reference = ReferenceContext {
        bundle: model_bundle,
        entry: structure.id.clone(),
        namespace: "urn:s09:model-context".into(),
        conversions: vec![],
        materialized_types: vec![],
        applied_conversions: vec![],
        constructors: vec![],
        model: model_value.clone(),
    };
    let mut behavior = DefinitionSource::new(
        "memory:s09-model-payload-read.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package payloadRead version \"1\"; import model \"urn:s09:model-context\" as m; fn readText(x:m.Holder)->String=x.text;",
    );
    behavior.dependencies.push(structure.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![structure, behavior.clone()],
        namespaces: vec![],
    };
    let selection = EntrySelection {
        symbol: SymbolId {
            source: behavior.id.clone(),
            declaration: 1,
        },
        types: vec![],
        arguments: vec![],
    };
    let image = compile_function(&bundle, &behavior.id, &selection, &Limits::default())
        .map_err(|issues| std::io::Error::other(format!("{issues:?}")))?;
    let ty = image.program().functions[image.program().entry.0 as usize].parameters[0];
    let TypeNode::Model(target_type) = &image.program().types[ty.0 as usize] else {
        return Err("Expected actual checked model parameter".into());
    };
    let domain = intersect_model::model_reference::ReferenceDomain::new_runtime(
        target_type.clone(),
        reference.clone(),
        &ConversionRegistry::default(),
    )?;
    let argument = ValueWire {
        ty,
        data: DataWire::Model(domain.bind("holder")?),
    };
    let value = json!({
        "id":"stored-unicode-escaped-90112", "size":text.len(),
        "independent_expectation":{"codepoints":[34,92,10,9,0,233,128512],"repetitions":8192,"utf8_bytes":90112,"utf16_units":65536,"resource":"memory:model-payload-unicode","element":"holder","stored_member":"Holder.text"},
        "schema":schema,"core_model":model_value,
        "request":{"method":"rcore_behavior/startFunction","bundle":bundle,"entry":behavior.id,"selection":selection,"arguments":[argument],"reference_contexts":[reference],"expected_program":image.program(),"execution_limits":ExecutionLimits::default()}
    });
    let mut output = std::fs::File::create_new(target)?;
    serde_json::to_writer_pretty(&mut output, &value)?;
    output.write_all(b"\n")?;
    Ok(())
}
