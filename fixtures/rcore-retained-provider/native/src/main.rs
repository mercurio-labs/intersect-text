#[path = "../../driver.rs"]
mod driver;
fn main() {
    println!("{}", driver::observe());
}
