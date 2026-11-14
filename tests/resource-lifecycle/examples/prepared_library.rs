//! Native host measurements; no clocks enter the shared model runtime.
fn main() -> Result<(), String> {
    println!("{}", intersect_resource_lifecycle_tests::measure_cache()?);
    Ok(())
}
