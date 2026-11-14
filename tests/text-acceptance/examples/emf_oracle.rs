//! Optional host harness for the independent EMF round-trip control.
use intersect_model::core::{
    interchange::{ModelFormat, Resource},
    model::{Model, values},
};
use intersect_model::definitions::schema::Schema;
use intersect_model::xmi::XmiFormat;
use intersect_text::{Source, evaluate};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

fn normalized(schema: &Schema, model: &Model) -> Result<Value, String> {
    let names: BTreeMap<_, _> = model
        .elements
        .iter()
        .map(|e| (e.id.as_str(), format!("{}:{}", e.class, e.values["name"])))
        .collect();
    let mut result = BTreeMap::new();
    for e in &model.elements {
        let mut fields = e.values.clone();
        for (name, feature) in schema.features(&e.class)? {
            if feature.reference {
                let targets: Result<Vec<_>, String> =
                    values(fields.get(&name).unwrap_or(&Value::Null), feature.upper)?
                        .into_iter()
                        .map(|v| {
                            Ok(Value::String(
                                names
                                    .get(v.as_str().ok_or("Invalid reference")?)
                                    .ok_or("Unknown reference")?
                                    .clone(),
                            ))
                        })
                        .collect();
                fields.insert(name, Value::Array(targets?));
            }
        }
        result.insert(names[&e.id.as_str()].clone(), fields);
    }
    serde_json::to_value(result).map_err(|e| e.to_string())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let mode = args
        .get(1)
        .ok_or("Expected export or verify, followed by directory")?;
    let directory = Path::new(args.get(2).ok_or("Missing directory")?);
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../fixtures/library/library.ecore"),
        include_str!("../../../fixtures/library/library.xtext"),
        include_str!("../../../fixtures/library/library-behavior.rcore"),
    )
    .map_err(|e| format!("{e:?}"))?;
    let expected = evaluate(
        &package,
        &[
            Source {
                uri: "catalog.library".into(),
                text: include_str!("../../../fixtures/library/catalog.library").into(),
            },
            Source {
                uri: "loans.library".into(),
                text: include_str!("../../../fixtures/library/loans.library").into(),
            },
        ],
    );
    if !expected.diagnostics.is_empty() {
        return Err("Fixture evaluation failed".into());
    }
    match mode.as_str() {
        "export" => {
            std::fs::create_dir_all(directory)?;
            for resource in XmiFormat.encode(&package.schema, &expected.model)? {
                std::fs::write(directory.join(resource.uri), resource.bytes)?;
            }
        }
        "verify" => {
            let mut resources = vec![];
            for entry in std::fs::read_dir(directory)? {
                let path = entry?.path();
                if path.extension().is_some_and(|e| e == "xmi") {
                    resources.push(Resource {
                        uri: path
                            .file_name()
                            .ok_or("Missing filename")?
                            .to_string_lossy()
                            .into_owned(),
                        bytes: std::fs::read(&path)?,
                    });
                }
            }
            let restored = XmiFormat.decode(&package.schema, &resources)?;
            if normalized(&package.schema, &restored)?
                != normalized(&package.schema, &expected.model)?
            {
                return Err("EMF round-trip model mismatch".into());
            }
            println!(
                "Native reimport matches every stored feature and reference of the Library control."
            );
        }
        _ => return Err("Unknown mode".into()),
    }
    Ok(())
}
