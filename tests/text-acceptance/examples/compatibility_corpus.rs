//! Native test host: emit independently built dynamic/text models for actual ZIP/WASM controls.
#[path = "../tests/xmi/support/compatibility.rs"]
mod corpus;
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let d = match args.get(1).map(String::as_str) {
        None => corpus::distribution(),
        Some("archive-baseline") => corpus::ordinary_baseline_distribution(),
        Some(_) => return oracle(&args),
    };
    let mut cases = vec![];
    for seed in 0..8 {
        let c = corpus::case(seed, &d);
        cases.push(json!({"seed":seed,"sources":c.sources,"model":c.text,"dynamic":c.dynamic.to_model()?,"dynamic_schema":c.dynamic.schema().as_schema(),"normalized":corpus::normalized(&d.package.schema,&c.text)}));
    }
    println!("{}", json!({"distribution":d,"cases":cases}));
    Ok(())
}

fn oracle(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mode = &args[1];
    let directory = std::path::Path::new(args.get(2).ok_or("Missing directory")?);
    let d = corpus::distribution();
    if mode == "oracle-export" {
        std::fs::create_dir_all(directory)?;
        std::fs::write(directory.join("graph.ecore"), corpus::ECORE)?;
    }
    for seed in 0..8 {
        let c = corpus::case(seed, &d);
        for (origin, schema, model) in [
            (
                "dynamic",
                c.dynamic.schema().as_schema(),
                c.dynamic.to_model()?,
            ),
            ("text", &d.package.schema, c.text.clone()),
        ] {
            let folder = directory.join(format!("seed-{seed}/{origin}"));
            if mode == "oracle-export" {
                std::fs::create_dir_all(&folder)?;
                for r in corpus::registry().export("xmi", schema, &model)? {
                    std::fs::write(folder.join(r.uri), r.bytes)?;
                }
            } else if mode == "oracle-verify" {
                let expected = corpus::normalized(schema, &model);
                let observation: serde_json::Value =
                    serde_json::from_slice(&std::fs::read(folder.join("normalized.json"))?)?;
                if observation != expected {
                    return Err(
                        format!("EMF observed different values/edges for {seed} {origin}").into(),
                    );
                }
                let mut resources = vec![];
                for entry in std::fs::read_dir(&folder)? {
                    let path = entry?.path();
                    if path.extension().is_some_and(|s| s == "xmi") {
                        resources.push(intersect_model::core::interchange::Resource {
                            uri: path
                                .file_name()
                                .ok_or("Missing filename")?
                                .to_string_lossy()
                                .into(),
                            bytes: std::fs::read(path)?,
                        });
                    }
                }
                let restored = corpus::registry().import("xmi", schema, &resources)?;
                if corpus::normalized(schema, &restored) != expected {
                    return Err(format!("EMF return-trip mismatch {seed} {origin}").into());
                }
                println!("Every stored value and reference matches EMF: {seed} {origin}");
            } else {
                return Err("Expected oracle-export or oracle-verify".into());
            }
        }
    }
    Ok(())
}
