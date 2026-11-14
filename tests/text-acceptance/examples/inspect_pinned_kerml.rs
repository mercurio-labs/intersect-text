//! Host-only pinned grammar probe. Structural compilation is not domain qualification.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Usage: inspect_pinned_kerml PILOT_ROOT")?,
    );
    let profile = match std::env::args().nth(2).as_deref() {
        None => Profile::NativeV11,
        Some("--validated-syntax") => Profile::NativeV14,
        Some("--native-v21") => Profile::NativeV21,
        Some(_) => return Err("Unknown pinned probe option".into()),
    };
    let read = |path: &str, kind, expected: &str| -> Result<DefinitionSource, std::io::Error> {
        let source = DefinitionSource::new(path, kind, std::fs::read_to_string(root.join(path))?);
        if source.id.sha256 != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Pinned source digest mismatch: {path}"),
            ));
        }
        Ok(source)
    };
    let ecore = read(
        "org.omg.sysml/model/SysML.ecore",
        SourceKind::Ecore,
        "22719934c65588180fc19658f8a7c891cb209458e4027bf90ddf005c80ddc17f",
    )?;
    let mut base = read(
        "org.omg.kerml.expressions.xtext/src/org/omg/kerml/expressions/xtext/KerMLExpressions.xtext",
        SourceKind::Xtext,
        "7bea42879a913f7613ac5e32779174db01e8fd4fe75694f0845c495aa0191aa8",
    )?;
    let mut grammar = read(
        "org.omg.kerml.xtext/src/org/omg/kerml/xtext/KerML.xtext",
        SourceKind::Xtext,
        "cc02be703955603d2daa91f878f299f950195fcd2e3c59563ae17643ba91dfaa",
    )?;
    base.dependencies = vec![ecore.id.clone()];
    grammar.dependencies = vec![ecore.id.clone(), base.id.clone()];
    let mut semantics = DefinitionSource::new(
        "probe-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantics.dependencies = vec![ecore.id.clone()];
    let policy = CompilationSourcePolicy {
        structure_bytes: ecore.text.len(),
        xtext_bytes: grammar.text.len().max(base.text.len()),
        // 108 parent + 176 child declarations: 282 effective and two shadowed.
        xtext_declarations: 284,
    };
    let source_identities = [&ecore.id, &base.id, &grammar.id].map(Clone::clone);
    let compilation = BundleCompilation {
        ecore: ecore.id.clone(),
        materialized_types: vec![],
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantics.id.clone(),
        namespace: "https://www.omg.org/spec/SysML/20250201".into(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, base, grammar, semantics],
            namespaces: vec![],
        },
    };
    match compiler::compile_bundle_with_source_policy(compilation, profile, policy, policy) {
        Ok(package) => println!(
            "{}",
            serde_json::json!({"ok":true,"classes":package.schema.classes.len(),"sources":source_identities,"profile":profile.as_str(),"domain_semantics_verified":false})
        ),
        Err(diagnostics) => println!(
            "{}",
            serde_json::json!({"ok":false,"diagnostics":diagnostics,"sources":source_identities,"profile":profile.as_str(),"domain_semantics_verified":false})
        ),
    }
    Ok(())
}
