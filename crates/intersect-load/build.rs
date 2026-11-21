//! Checked identity of shared loader code; no view dependency or I/O at runtime.
fn main() -> Result<(), String> {
    intersect_build_support::emit_scoped("INTERSECT_LOAD_SOURCE_IDENTITY")
}
