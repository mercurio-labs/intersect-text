//! Verification-only JSON process consumer of the same production web dispatcher.
use std::io::{self, BufRead};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let line = line?;
        let value = intersect_web::dispatch(&line)
            .unwrap_or_else(|error| serde_json::json!({"error":error}));
        println!("{}", serde_json::to_string(&value)?);
    }
    Ok(())
}
