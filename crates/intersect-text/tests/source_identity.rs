//! The compiler identity is one SHA-256 digest in lowercase hex.
#[test]
fn source_identity_is_a_sha256_hex_digest() {
    let identity = intersect_text::SOURCE_IDENTITY;
    assert_eq!(identity.len(), 64, "{identity}");
    assert!(
        identity
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "{identity}"
    );
}

#[test]
fn compiler_cache_keys_include_profile_and_installed_provider_versions() -> Result<(), String> {
    use intersect_text::{compiler_cache_identity, compiler_identity, grammar::Profile};
    let key = compiler_cache_identity(Profile::NativeV80, &["scope@1", "values@1"])?;
    assert_eq!(
        key,
        compiler_cache_identity(Profile::NativeV80, &["values@1", "scope@1"])?
    );
    assert_ne!(
        key,
        compiler_cache_identity(Profile::NativeV80, &["scope@2", "values@1"])?
    );
    assert_ne!(
        key,
        compiler_cache_identity(Profile::NativeV82, &["scope@1", "values@1"])?
    );
    assert_ne!(compiler_identity(), intersect_text::SOURCE_IDENTITY);
    assert_ne!(compiler_identity(), intersect_model::SOURCE_IDENTITY);
    Ok(())
}

#[test]
fn legacy_ordinary_descriptor_matches_the_frozen_source_archive() -> Result<(), String> {
    let observed: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-archive/ordinary-baseline.json"
    ))
    .map_err(|error| error.to_string())?;
    let installed = intersect_text::distribution::model_compiler_identity();
    assert_eq!(
        serde_json::to_value(installed).map_err(|error| error.to_string())?,
        observed["manifest"]["compiler"]
    );
    Ok(())
}
