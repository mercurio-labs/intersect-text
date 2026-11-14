fn main() {
    let path = std::env::args().nth(1).unwrap();
    let output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .unwrap();
    serde_json::to_writer(output, &intersect_numeric_payload_qualification::observe()).unwrap();
}
