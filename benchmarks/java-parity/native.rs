use intersect_model::core::{
    interchange::{ModelFormat, Resource},
    model::Model,
    storage::{self, ModelRead, ModelStore},
};
use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
use intersect_model::definitions::schema::Schema;
use intersect_model::xmi::XmiFormat;
use intersect_text::package::BundleCompilation;
use intersect_text::{EvaluationLimits, Source, evaluate_stored};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::Path,
    time::Instant,
};

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}
// Diagnostic only: mutate an already loaded valid graph, then exercise the
// structural validator. Normal timing runs do not select this branch.
fn structural_control(
    store: &mut impl ModelStore,
    schema: &Schema,
    case: &str,
) -> Result<(), String> {
    let named = |name: &str| -> Result<String, String> {
        let ids = store.query(storage::Query::Value("name", &json!(name)))?;
        if ids.len() != 1 {
            return Err(format!("Control requires one {name}"));
        }
        ids.into_iter()
            .next()
            .ok_or("Missing control object".into())
    };
    let (part, port, dependency, requirement, package) = (
        named("p0")?,
        named("a0")?,
        named("p1")?,
        named("q0")?,
        named("package0")?,
    );
    storage::atomic(store, |store| match case {
        "valid" => Ok(()),
        "missing-required" => store.set_value(&part, "type", None),
        "wrong-target" => store.set_resolved_value(&part, "type", Some(json!(port))),
        "duplicate-many" => {
            store.set_resolved_value(&part, "dependencies", Some(json!([dependency, dependency])))
        }
        "dangling-reference" => {
            store.set_resolved_value(&part, "type", Some(json!("missing-control-object")))
        }
        "wrong-datatype" => store.set_value(&requirement, "text", Some(json!(42))),
        "containment-cycle" => store.append_value(&package, "elements", json!(package)),
        _ => Err(format!("Unknown structural control {case}")),
    })?;
    eprintln!("CONTRACT_MUTATION_APPLIED");
    storage::validate(store, schema)
}
// The XMI API returns an owned snapshot. Mutate that representation directly
// for diagnostic controls, then use its actual shared ModelView validator.
fn structural_snapshot(model: &mut Model, schema: &Schema, case: &str) -> Result<(), String> {
    let names: BTreeMap<_, _> = model
        .elements
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            e.values
                .get("name")
                .and_then(Value::as_str)
                .map(|name| (name.to_owned(), i))
        })
        .collect();
    let index = |name: &str| {
        names
            .get(name)
            .copied()
            .ok_or_else(|| format!("Missing control {name}"))
    };
    let (part, port, dependency, requirement, package) = (
        index("p0")?,
        index("a0")?,
        index("p1")?,
        index("q0")?,
        index("package0")?,
    );
    let (port_id, dependency_id, package_id) = (
        model.elements[port].id.clone(),
        model.elements[dependency].id.clone(),
        model.elements[package].id.clone(),
    );
    match case {
        "valid" => {}
        "missing-required" => {
            model.elements[part].values.remove("type");
        }
        "wrong-target" => {
            model.elements[part]
                .values
                .insert("type".into(), json!(port_id));
        }
        "duplicate-many" => {
            model.elements[part]
                .values
                .insert("dependencies".into(), json!([dependency_id, dependency_id]));
        }
        "dangling-reference" => {
            model.elements[part]
                .values
                .insert("type".into(), json!("missing-control-object"));
        }
        "wrong-datatype" => {
            model.elements[requirement]
                .values
                .insert("text".into(), json!(42));
        }
        "containment-cycle" => {
            model.elements[package]
                .values
                .get_mut("elements")
                .and_then(Value::as_array_mut)
                .ok_or("Missing control containment")?
                .push(json!(package_id));
        }
        _ => return Err(format!("Unknown structural control {case}")),
    }
    eprintln!("CONTRACT_MUTATION_APPLIED");
    storage::validate(&storage::ModelView::new(model)?, schema)
}
// Fixture-specific receipt, outside timed loading and retained samples.
fn fixture_identities(model: &Model, idless: Option<&str>) -> Result<usize, String> {
    const FILES: [&str; 10] = [
        "r0.xmi", "r1.xmi", "r2.xmi", "r3.xmi", "r4.xmi", "r5.xmi", "r6.xmi", "r7.xmi", "r8.xmi",
        "r9.xmi",
    ];
    for element in &model.elements {
        let name = element
            .values
            .get("name")
            .and_then(Value::as_str)
            .ok_or("Missing fixture name")?;
        let digit = *name.as_bytes().last().ok_or("Empty fixture name")?;
        if !digit.is_ascii_digit()
            || element.resource.rsplit('/').next() != Some(FILES[usize::from(digit - b'0')])
        {
            return Err(format!("Fixture resource identity mismatch {name}"));
        }
        let (resource, fragment) = element
            .id
            .rsplit_once('#')
            .ok_or("Missing fixture fragment")?;
        if resource != element.resource
            || if idless == Some(name) {
                !fragment.starts_with('/')
            } else {
                fragment != name
            }
        {
            return Err(format!("Fixture explicit identity mismatch {name}"));
        }
    }
    Ok(model.elements.len())
}
// Same canonical graph receipt as checksum(), using borrowed fields instead of
// exporting the entire live store to an owned compatibility snapshot. Verification
// remains outside measured phases. --check-dto cross-checks both readers.
fn checksum_store(
    store: &impl ModelRead,
    schema: &Schema,
    expected: usize,
) -> Result<String, String> {
    let mut ids = Vec::new();
    let mut after = None;
    loop {
        let page = store.id_page(after, 4096)?;
        if page.is_empty() {
            break;
        }
        after = page.last().map(|(row, _)| *row);
        ids.extend(page.into_iter().map(|(_, id)| id));
    }
    if ids.len() != expected {
        return Err(format!("Object count {} expected {expected}", ids.len()));
    }
    let mut objects = BTreeMap::new();
    for (index, id) in ids.iter().enumerate() {
        let name = store
            .value(id, "name")?
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or("Missing name")?;
        if objects.insert(name, index).is_some() {
            return Err("Duplicate name".into());
        }
    }
    let id_names: std::collections::HashMap<_, _> = objects
        .iter()
        .map(|(name, &index)| (ids[index].as_str(), name.as_str()))
        .collect();
    let mut hash = Sha256::new();
    for (name, index) in &objects {
        if !store.inspect(&ids[*index], &mut |object| {
            let class = &schema
                .classes
                .get(object.class())
                .ok_or("Missing class")?
                .name;
            let mut line = format!("{class}|{name}");
            for (key, feature) in schema.features(object.class())? {
                if key == "name" {
                    continue;
                }
                line.push('|');
                line.push_str(&key);
                line.push('=');
                let mut atom =
                    |value: intersect_model::core::storage::ValueView<'_>| -> Result<(), String> {
                        let s = value.as_str().ok_or_else(|| format!("Nonstring {key}"))?;
                        if feature.containment || schema.classes.contains_key(&feature.target) {
                            line.push_str(
                                id_names
                                    .get(s)
                                    .ok_or_else(|| format!("Unresolved target {s}"))?,
                            );
                        } else {
                            line.push_str(s);
                        }
                        Ok(())
                    };
                if let Some(value) = object.value(&key) {
                    if let Some(values) = value.collection() {
                        // Keep the delimiter separate from the closure's borrow.
                        let mut rendered = Vec::new();
                        for value in values.iter() {
                            let s = value.as_str().ok_or_else(|| format!("Nonstring {key}"))?;
                            rendered.push(
                                if feature.containment
                                    || schema.classes.contains_key(&feature.target)
                                {
                                    *id_names
                                        .get(s)
                                        .ok_or_else(|| format!("Unresolved target {s}"))?
                                } else {
                                    s
                                },
                            );
                        }
                        line.push_str(&rendered.join(","));
                    } else if !value.is_null() {
                        atom(value)?;
                    }
                }
            }
            line.push('\n');
            hash.update(line.as_bytes());
            Ok(())
        })? {
            return Err("Object disappeared during verification".into());
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn event(phase: &str, extra: Value) -> Result<(), String> {
    let mut row = extra.as_object().cloned().ok_or("Event must be object")?;
    row.insert("phase".into(), json!(phase));
    println!("{}", Value::Object(row));
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut line = String::new();
    if io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?
        == 0
    {
        return Err("Handshake closed".into());
    }
    Ok(())
}
fn checksum(model: &Model, schema: &Schema, expected: usize) -> Result<String, String> {
    if model.elements.len() != expected {
        return Err(format!(
            "Object count {} expected {expected}",
            model.elements.len()
        ));
    }
    let mut id_names = BTreeMap::new();
    let mut objects = BTreeMap::new();
    for e in &model.elements {
        let name = e
            .values
            .get("name")
            .and_then(Value::as_str)
            .ok_or("Missing name")?;
        id_names.insert(e.id.as_str(), name);
        if objects.insert(name, e).is_some() {
            return Err("Duplicate name".into());
        }
    }
    let mut hash = Sha256::new();
    for (name, e) in objects {
        let class = &schema.classes.get(&e.class).ok_or("Missing class")?.name;
        let mut line = format!("{class}|{name}");
        let fields = schema.features(&e.class)?;
        for (key, feature) in &fields {
            if key == "name" {
                continue;
            }
            line.push('|');
            line.push_str(key);
            line.push('=');
            let val = e.values.get(key).unwrap_or(&Value::Null);
            let values = match val {
                Value::Array(a) => a.iter().collect::<Vec<_>>(),
                Value::Null => vec![],
                v => vec![v],
            };
            for (i, v) in values.iter().enumerate() {
                if i > 0 {
                    line.push(',');
                }
                let s = v.as_str().ok_or_else(|| format!("Nonstring {key}: {v}"))?;
                if feature.containment || schema.classes.contains_key(&feature.target) {
                    line.push_str(
                        id_names
                            .get(s)
                            .ok_or_else(|| format!("Unresolved target {s}"))?,
                    );
                } else {
                    line.push_str(s);
                }
            }
        }
        line.push('\n');
        hash.update(line.as_bytes());
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let mode = args.get(1).ok_or("mode required")?;
    let folder = Path::new(args.get(2).ok_or("folder required")?);
    let n: usize = args
        .get(3)
        .ok_or("size required")?
        .parse()
        .map_err(|e| format!("{e}"))?;
    let expected = args.get(4).ok_or("checksum required")?;
    let definitions = folder
        .parent()
        .and_then(Path::parent)
        .ok_or("definitions path")?;
    let t = Instant::now();
    let rcore =
        std::fs::read_to_string(definitions.join("model.rcore")).map_err(|e| e.to_string())?;
    let grammar =
        std::fs::read_to_string(definitions.join("SysmlLike.xtext")).map_err(|e| e.to_string())?;
    let model_source = DefinitionSource::new("memory:benchmark.rcore", SourceKind::Rcore, &rcore);
    let mut grammar_source =
        DefinitionSource::new("memory:benchmark.xtext", SourceKind::Xtext, &grammar);
    grammar_source.dependencies.push(model_source.id.clone());
    let terminal_text =
        std::fs::read_to_string(definitions.join("Terminals.xtext")).map_err(|e| e.to_string())?;
    let mut terminals = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        &terminal_text,
    );
    terminals.dependencies.push(model_source.id.clone());
    grammar_source.dependencies.push(terminals.id.clone());
    let compilation = BundleCompilation {
        materialized_types: vec![],
        ecore: model_source.id.clone(),
        namespace: "urn:intersect:benchmark:sysml-like".into(),
        xtext: if mode == "text" {
            Some(grammar_source.id.clone())
        } else {
            None
        },
        rcore_behavior: model_source.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: if mode == "text" {
                vec![model_source, grammar_source, terminals]
            } else {
                vec![model_source]
            },
            namespaces: vec![],
        },
    };
    let package =
        intersect_text::ordinary::compile(compilation, intersect_text::grammar::Profile::NativeV82)
            .map_err(|e| format!("{e:?}"))?;
    event("baseline", json!({"setup_ms":ms(t)}))?;
    let t = Instant::now();
    let mut resources = Vec::new();
    let mut sources = Vec::new();
    for r in 0..10 {
        let path = folder.join(format!(
            "r{r}.{}",
            if mode == "text" { "sml" } else { "xmi" }
        ));
        let uri = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
        if mode == "text" {
            sources.push(Source {
                uri,
                text: std::fs::read_to_string(path).map_err(|e| e.to_string())?,
            });
        } else {
            resources.push(Resource {
                uri,
                bytes: std::fs::read(path).map_err(|e| e.to_string())?,
            });
        }
    }
    event("inputs", json!({"io_ms":ms(t)}))?;
    for trial in ["cold", "warm"] {
        let t = Instant::now();
        if mode == "text" {
            let mut result = evaluate_stored(&package, &sources, EvaluationLimits::native())?;
            if let Some(case) = args.iter().find_map(|s| s.strip_prefix("--structural=")) {
                structural_control(&mut result.store, &package.schema, case)?;
            }
            let elapsed = ms(t);
            if !result.evaluation.diagnostics.is_empty() {
                return Err(format!("{:?}", result.evaluation.diagnostics));
            }
            event(&format!("{trial}_loaded"), json!({"pipeline_ms":elapsed}))?;
            event(&format!("{trial}_retained"), json!({}))?;
            let t = Instant::now();
            let actual = checksum_store(&result.store, &package.schema, n)?;
            if args.get(5).is_some_and(|s| s == "--check-dto") {
                let model = intersect_model::core::storage::export(&result.store)?;
                if checksum(&model, &package.schema, n)? != actual {
                    return Err("Verifier disagreement".into());
                }
            }
            if &actual != expected {
                return Err(format!("Graph checksum {actual} expected {expected}"));
            }
            event(
                &format!("{trial}_verified"),
                json!({"objects":n,"checksum":actual,"verification_ms":ms(t)}),
            )?;
        } else {
            let mut model = XmiFormat.decode(&package.schema, &resources)?;
            if let Some(case) = args.iter().find_map(|s| s.strip_prefix("--structural=")) {
                structural_snapshot(&mut model, &package.schema, case)?;
            }
            let elapsed = ms(t);
            event(&format!("{trial}_loaded"), json!({"pipeline_ms":elapsed}))?;
            event(&format!("{trial}_retained"), json!({}))?;
            let t = Instant::now();
            if args.iter().any(|s| s == "--fixture-identities") {
                let idless = args
                    .iter()
                    .find_map(|s| s.strip_prefix("--fixture-idless="));
                eprintln!(
                    "FIXTURE_IDENTITIES_VERIFIED {}",
                    fixture_identities(&model, idless)?
                );
            }
            let actual = checksum(&model, &package.schema, n)?;
            if &actual != expected {
                return Err(format!("Graph checksum {actual} expected {expected}"));
            }
            event(
                &format!("{trial}_verified"),
                json!({"objects":n,"checksum":actual,"verification_ms":ms(t)}),
            )?;
        }
        if trial == "cold" {
            event("between", json!({}))?;
        }
    }
    println!("{}", json!({"phase":"done"}));
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
