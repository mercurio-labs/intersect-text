//! Compiler/cache identities compose installed packages without filesystem access.
use sha2::{Digest, Sha256};

/// Changing this recipe invalidates native caches. It names the independent wire
/// versions rather than interpreting portable archives as cached compiler output.
pub const CACHE_IDENTITY_RECIPE: &str =
    "intersect.xtext.compiler.v1;source=3;grammar-cache=1;draft-cache=1;text=2;model-metadata=3";

fn frame(digest: &mut Sha256, tag: &str, value: &str) {
    for part in [tag, value] {
        digest.update((part.len() as u64).to_le_bytes());
        digest.update(part.as_bytes());
    }
}

fn composed(owned: &str, model: &str, recipe: &str) -> String {
    let mut digest = Sha256::new();
    frame(&mut digest, "recipe", recipe);
    frame(&mut digest, "xtext", owned);
    frame(&mut digest, "model", model);
    format!("{:x}", digest.finalize())
}

/// Identity of the installed Xtext compiler and model, including enabled features
/// and the versioned cache recipe. This is a cache key, never a signature or an
/// admission capability. No dependency source checkout is needed.
pub fn compiler_identity() -> &'static str {
    static IDENTITY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    IDENTITY.get_or_init(|| {
        composed(
            crate::SOURCE_IDENTITY,
            intersect_model::SOURCE_IDENTITY,
            CACHE_IDENTITY_RECIPE,
        )
    })
}

/// Host cache key for a selected profile and installed provider implementations.
/// Hosts must supply versioned provider identities, not just contract names.
/// Provider order is irrelevant; a different implementation must use a new key.
pub fn compiler_cache_identity(
    profile: crate::grammar::Profile,
    provider_identities: &[&str],
) -> Result<String, String> {
    let mut digest = Sha256::new();
    frame(&mut digest, "compiler", compiler_identity());
    let profile = serde_json::to_string(&profile).map_err(|error| error.to_string())?;
    frame(&mut digest, "profile", &profile);
    let mut providers = provider_identities.to_vec();
    providers.sort_unstable();
    for provider in providers {
        frame(&mut digest, "provider", provider);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn independently_changed_compiler_components_invalidate_caches() {
        let original = super::composed("text-a", "model-a", "recipe-a");
        for (text, model, recipe) in [
            ("text-b", "model-a", "recipe-a"),
            ("text-a", "model-b", "recipe-a"),
            ("text-a", "model-a", "recipe-b"),
        ] {
            assert_ne!(super::composed(text, model, recipe), original);
        }
        assert_ne!(
            super::composed("ab", "c", "r"),
            super::composed("a", "bc", "r")
        );
    }
}
