//! Handwritten host-only benchmark of the unchanged production evaluator.
use intersect_model::core::storage::{ModelRead, visit_views};

use intersect_text::{EvaluationLimits, Source, evaluate_stored};
use serde_json::json;
use std::{env, error::Error, fs, io, time::Instant};

#[cfg(feature = "allocation-profile")]
#[path = "support/allocations.rs"]
mod allocations;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or("expected input path and element count")?;
    let expected: usize = args.next().ok_or("expected element count")?.parse()?;
    let stored = match args.next().as_deref() {
        None => false,
        Some("--stored") => true,
        Some(_) => return Err("optional third argument must be --stored".into()),
    };
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../fixtures/library/library.ecore"),
        include_str!("../../../fixtures/library/library.xtext"),
        include_str!("../../../fixtures/library/library-behavior.rcore"),
    )
    .map_err(|errors| format!("Definition compilation failed: {errors:?}"))?;
    let sources = [Source {
        uri: "benchmark.library".into(),
        text: fs::read_to_string(path)?,
    }];
    // Parent samples process memory at explicit boundaries. Input and definitions
    // are resident before timing; the evaluated model stays alive through DONE.
    println!("READY");
    io::stdin().read_line(&mut String::new())?;
    #[cfg(feature = "allocation-profile")]
    let allocation_start = allocations::begin();
    let start = Instant::now();
    let result = evaluate_stored(&package, &sources, EvaluationLimits::native())?;
    let metadata = result.evaluation;
    let (model, store) = if stored {
        (None, Some(result.store))
    } else {
        (Some(result.store.into_model()?), None)
    };
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    #[cfg(feature = "allocation-profile")]
    let allocation = allocations::finish(allocation_start);
    #[cfg(not(feature = "allocation-profile"))]
    let allocation = serde_json::Value::Null;
    let mut elements = 0;
    let mut verified = metadata.diagnostics.is_empty();
    if let Some(model) = &model {
        elements = model.elements.len();
        verified &= model.elements.iter().enumerate().all(|(i, e)| {
            e.class == if i == 0 { "Library" } else { "Member" }
                && e.values.get("name").and_then(serde_json::Value::as_str)
                    == Some(
                        if i == 0 {
                            "Scale".into()
                        } else {
                            format!("m{i}")
                        }
                        .as_str(),
                    )
        });
    }
    if let Some(store) = &store {
        // Verification uses borrowed views, retaining the canonical result.
        visit_views(store, |e| {
            let name = if elements == 0 {
                "Scale".into()
            } else {
                format!("m{elements}")
            };
            verified &= e.class() == if elements == 0 { "Library" } else { "Member" }
                && e.value("name").and_then(|v| v.as_str()) == Some(name.as_str());
            elements += 1;
            Ok(())
        })?;
        verified &= !store.revision()?.is_empty();
    }
    verified &= elements == expected;
    println!("MEASURED");
    io::stdin().read_line(&mut String::new())?;
    println!(
        "{}",
        json!({
            "engine": "intersect", "stage": if stored { "evaluate_stored" } else { "evaluate" }, "status": if verified { "ok" } else { "rejected" },
            "elapsed_ms": elapsed_ms, "elements": elements,
            "verified": verified, "diagnostics": metadata.diagnostics,
            "allocation": allocation,
        })
    );
    std::hint::black_box((&model, &store, &metadata));
    Ok(())
}
