//! Verification host exporting the shared observation suite to a new JSON file.
#[path = "../tests/support/first_rule_proof_gap_observations.rs"]
mod observations;
use std::{error::Error, fs::OpenOptions, io::Write};
fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args()
        .nth(1)
        .ok_or("Select a NEW output JSON file")?;
    let value = observations::suite()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    serde_json::to_writer_pretty(&mut file, &value)?;
    file.write_all(b"\n")?;
    Ok(())
}
