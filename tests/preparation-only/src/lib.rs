//! External native/WASM consumer of preparation, never a language runtime.
use intersect_text::package::CompilationSourcePolicy;
#[path = "../../../fixtures/prepared-language/controls.rs"]
mod controls;
use intersect_text::grammar::Profile;
use intersect_model::definitions::preparation::BuiltinRuleCoverage;
use intersect_model::definitions::preparation::SemanticRequirement;
fn require(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.into()) }
}
pub fn verify_preparation_boundary() -> Result<(), String> {
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/prepared-language/strict-original-observations.json"
    ))
    .map_err(|error| error.to_string())?;
    require(
        controls::strict_observations()? == expected,
        "Strict whole Package/error/recovery outcomes differ from original executable",
    )?;
    let (input, origin) = controls::inputs(controls::EMPTY)?;
    let policy = CompilationSourcePolicy::default();
    let prepared = intersect_text::ordinary::prepare_with_context(
        &input,
        &origin,
        Profile::NativeV67,
        &[],
        None,
        policy,
        policy,
    )
    .map_err(|errors| format!("{errors:?}"))?;
    require(
        std::ptr::eq(prepared.sources(), &input.bundle)
            && std::ptr::eq(prepared.original_sources(), &origin.original),
        "Preparation copied or replaced source authority",
    )?;
    let (mut classes, mut missing, mut stored_unverified) = (0, 0, 0);
    for row in prepared.requirements() {
        match row? {
            SemanticRequirement::ClassBehaviorUnverified { original, .. } => {
                require(original.is_some(), "Lost original class identity")?;
                classes += 1;
            }
            SemanticRequirement::MemberBehaviorUnverified {
                original,
                feature,
                rule_coverage,
                ..
            } => {
                require(original.is_some(), "Lost original member identity")?;
                if feature.name == "count" {
                    require(
                        feature.derived && rule_coverage == BuiltinRuleCoverage::Missing,
                        "Missing derived provider disappeared",
                    )?;
                    missing += 1;
                }
                if feature.name == "name" {
                    require(
                        !feature.derived
                            && rule_coverage == BuiltinRuleCoverage::StoredMetadataOnly,
                        "Non-derived metadata became a language-semantic certificate",
                    )?;
                    stored_unverified += 1;
                }
            }
        }
    }
    require(
        (classes, missing, stored_unverified) == (2, 2, 2),
        "Original/inherited semantic requirement inventory changed",
    )?;
    let mut tampered = origin.clone();
    tampered
        .reports
        .first_mut()
        .and_then(|report| report.correspondence.first_mut())
        .ok_or("Missing tamper control declaration")?
        .original_span
        .end += 1;
    require(
        intersect_text::ordinary::prepare_with_context(
            &input,
            &tampered,
            Profile::NativeV67,
            &[],
            None,
            policy,
            policy,
        )
        .is_err(),
        "Tampered native migration correspondence admitted",
    )?;
    let (good, good_origin) = controls::inputs(controls::GOOD)?;
    let strict = intersect_text::ordinary::compile(good.clone(), Profile::NativeV67)
        .map_err(|errors| format!("{errors:?}"))?;
    let inspected = intersect_text::ordinary::prepare_with_context(
        &good,
        &good_origin,
        Profile::NativeV67,
        &[],
        None,
        policy,
        policy,
    )
    .map_err(|errors| format!("{errors:?}"))?;
    require(
        serde_json::to_value(inspected.schema()).map_err(|e| e.to_string())?
            == serde_json::to_value(&strict.schema).map_err(|e| e.to_string())?
            && serde_json::to_value(inspected.grammar()).map_err(|e| e.to_string())?
                == serde_json::to_value(&strict.grammar).map_err(|e| e.to_string())?,
        "Prepared structural/grammar projection differs from strict shared compiler",
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn verify_model_boundary() -> u32 {
    u32::from(verify_preparation_boundary().is_err())
}
#[cfg(test)]
mod tests {
    #[test]
    fn preparation_is_inspection_only_and_preserves_strict_outputs() -> Result<(), String> {
        super::verify_preparation_boundary()
    }
}
