//! Verification-only transport to the existing public native source-policy API.
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
use std::io::{self, BufRead};
#[derive(serde::Deserialize)]
struct Request {
    compilation: BundleCompilation,
    profile: Profile,
    requested: CompilationSourcePolicy,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let result = intersect_text::ordinary::compile_with_source_policy(
            request.compilation,
            request.profile,
            request.requested,
            Default::default(),
        );
        let value = match result {
            Ok(package) => serde_json::json!({"ok":true,"package":package}),
            Err(diagnostics) => serde_json::json!({"ok":false,"diagnostics":diagnostics}),
        };
        println!("{}", serde_json::to_string(&value)?);
    }
    Ok(())
}
