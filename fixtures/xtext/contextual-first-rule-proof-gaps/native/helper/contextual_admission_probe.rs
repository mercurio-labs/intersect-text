//! Verification-only host probe. All admission semantics come from the public native adapter.
use intersect_xtext::grammar::SourceLimits;
use intersect_model::definitions::schema::BundleCompilation;
use serde_json::{Value, json};
use std::{error::Error, fs, io::Write, path::Path};

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("Usage: contextual_admission_probe INPUT_JSON NEW_OUTPUT_JSON".into());
    }
    let input: Value = serde_json::from_slice(&fs::read(&args[0])?)?;
    let cases = input["cases"]
        .as_array()
        .ok_or("Missing exact source cases")?;
    let limits = SourceLimits::default();
    let mut rows = Vec::new();
    for case in cases {
        let compilation: BundleCompilation = serde_json::from_value(case["compilation"].clone())?;
        let entry = compilation
            .xtext
            .as_ref()
            .ok_or("Missing exact authored grammar")?;
        compilation.bundle.validate()?;
        let result = match intersect_xtext::ordinary::validate_authored_program(
            &compilation.bundle,
            entry,
            limits,
        ) {
            Ok(()) => json!({"ok": true}),
            Err(diagnostic) => {
                let origin = serde_json::to_value(&diagnostic.origin)?;
                json!({"ok": false, "diagnostic": diagnostic, "origin": origin})
            }
        };
        rows.push(json!({
            "id": case["id"],
            "request": {
                "bundle": compilation.bundle,
                "entry": entry,
                "source_limits": {"source_bytes": limits.source_bytes, "declarations": limits.declarations}
            },
            "result": result
        }));
    }
    let output = json!({
        "version": 1,
        "scope": "Public authored-source validation only under fixed NativeV61. No executable package, generation/runtime, inferred-profile or full-support award.",
        "input": input,
        "cases": rows
    });
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&args[1]))?;
    serde_json::to_writer_pretty(&mut file, &output)?;
    file.write_all(b"\n")?;
    Ok(())
}
