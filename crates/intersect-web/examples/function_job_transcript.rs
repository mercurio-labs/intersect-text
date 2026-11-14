//! Native parity harness for ordered requests against one real function job host.
use std::io::{self, Read};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let requests: Vec<serde_json::Value> = serde_json::from_str(&input)?;
    let mut host = intersect_web::function_jobs::FunctionJobHost::default();
    let mut results = Vec::new();
    for request in requests {
        let result = match host.dispatch(&serde_json::to_string(&request)?) {
            Ok(result) => result,
            Err(error) => serde_json::json!({"error": error}),
        };
        results.push(result);
    }
    println!("{}", serde_json::to_string(&results)?);
    Ok(())
}
