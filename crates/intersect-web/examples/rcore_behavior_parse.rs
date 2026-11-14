//! Native host parity harness; reads a JSON request and writes its response.
use std::io::{self, Read};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut request = String::new();
    io::stdin().read_to_string(&mut request)?;
    // Match the repository's native test-thread stack allowance. Windows' main
    // thread does not inherit RUST_MIN_STACK, and recursive grammar fixtures
    // exercise the same deep debug-build paths as the Rust suite.
    let result = std::thread::Builder::new()
        .name("native-parity".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || intersect_web::dispatch(&request))?
        .join()
        .map_err(|_| io::Error::other("native parity worker panicked"))??;
    println!("{result}");
    Ok(())
}
