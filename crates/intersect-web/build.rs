//! Generated adapters connect the browser example to the same handwritten Rust.
use intersect_model_test_support::contract;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (_, contracts, _) = contract::contracts()?;
    let output = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    for (index, name) in ["twice", "doubled"].into_iter().enumerate() {
        std::fs::write(
            output.join(format!("{name}.rs")),
            intersect_model::codegen::semantic_bindings::generate(
                &contracts[index],
                &format!("crate::native::{name}"),
                &format!("register_{name}"),
                "bindings-example-rust-v1",
            )?,
        )?;
    }
    println!("cargo:rerun-if-changed=../../fixtures/rcore/bindings");
    Ok(())
}
