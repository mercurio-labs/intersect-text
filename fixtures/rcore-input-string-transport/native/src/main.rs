use std::io::Write;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let input = std::fs::read_to_string(args.get(1).ok_or("Missing request path")?)?;
    let mut output = std::io::BufWriter::new(std::fs::File::create_new(
        args.get(2).ok_or("Missing output path")?,
    )?);
    for line in input.lines() {
        let request: s09_leaf_transport_qualifier::Request = serde_json::from_str(line)?;
        let value = s09_leaf_transport_qualifier::run(request).map_err(std::io::Error::other)?;
        serde_json::to_writer(&mut output, &value)?;
        output.write_all(b"\n")?;
    }
    output.flush()?;
    Ok(())
}
