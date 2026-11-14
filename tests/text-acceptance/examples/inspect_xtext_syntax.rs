//! Host-only exact-source syntax probe; syntax import is not model execution.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::SourceLimits;
use intersect_text::grammar::syntax;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("Usage: inspect_xtext_syntax FILE EXPECTED_SHA256")?;
    let expected = args.next().ok_or("Expected source SHA256 required")?;
    let source = DefinitionSource::new(&path, SourceKind::Xtext, std::fs::read_to_string(&path)?);
    if source.id.sha256 != expected {
        return Err("Pinned source digest mismatch".into());
    }
    let result = syntax::parse(
        &source,
        SourceLimits {
            source_bytes: 256_000,
            declarations: 284,
        },
    );
    match result {
        Ok(ast) => println!(
            "{}",
            serde_json::json!({"source":ast.source,"grammar":ast.name.value,"rules":ast.rules.len(),"syntax_imported":true,"executable_support_claimed":false})
        ),
        Err(diagnostic) => {
            println!(
                "{}",
                serde_json::json!({"source":source.id,"syntax_imported":false,"diagnostic":diagnostic})
            );
            return Err("Syntax import failed".into());
        }
    }
    Ok(())
}
