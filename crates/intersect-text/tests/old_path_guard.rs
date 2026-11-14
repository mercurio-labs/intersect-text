//! Handwritten deprecation guard (not generated) for the old parse path owned by
//! intersect-text. Inventory, consumers and migration order:
//! docs/old-path-inventory.md. Scan rules:
//! intersect-text/tests/support/old_path_scan.rs.
//!
//! Guarded entry points, recorded 2026-11-05 (non-test files under
//! crates/*/src / crates/*/examples):
//! - `PhysicalDraftLanguage` 14 / 2 and the other `draft::physical` outputs
//!   (`PhysicalDraftAnalysis`, `PhysicalDraftGraph`, cohort, project and
//!   retained-store variants) 16 / 1: old pipeline only.
//! - the legacy Package evaluate path (`intersect_text::evaluate*`,
//!   `apply_semantics*`, `apply_stored_semantics*`, and the same names through
//!   `crate::`/`super::` inside intersect-text) 5 / 0: evaluates a Package
//!   into the former store at that checkpoint. The generic Package entry
//!   points now use frozen storage.
//! - `DraftCapability` 37 / 4 and `DraftLanguage::bind` 2 / 5: SHARED with the
//!   frozen path today (`OpenDraftCapability` is a `DraftCapability`, and a
//!   frozen load binds with `DraftLanguage::bind` and then calls
//!   `parse_runtime_into*` with a `FrozenStore`). They are capped so the old
//!   pipeline does not grow; a new frozen-path host that must use them may
//!   raise a number only together with a line in docs/old-path-inventory.md
//!   that names it. Raised 2026-11-06 to 40 / 3 and 3 / 4 by the frozen-path
//!   modules `intersect_kerml::text_capability`, `intersect_kerml::frozen_load`
//!   and `intersect_text::draft::runtime_text` (migration steps 1 and 2).
//!
//! 2026-11-06, migration step 3 (K01 pipeline and xtext draft/physical
//! pipeline deleted): `PhysicalDraftLanguage` 0 / 0, other `draft::physical`
//! outputs 0 / 0, Package evaluate path 5 / 0 (unchanged, step 4),
//! `DraftCapability` 7 / 2, `DraftLanguage::bind` 1 / 2.
//!
//! Lower a number when a migration removes uses.
#[path = "support/old_path_scan.rs"]
mod scan;

use scan::{Entry, File};

const PHYSICAL_DRAFT_OUTPUTS: [&str; 9] = [
    "PhysicalDraftAnalysis",
    "PhysicalDraftGraph",
    "PhysicalDraftCohortAnalysis",
    "PhysicalDraftCohortGraph",
    "PhysicalDraftProjectAnalysis",
    "PhysicalDraftProjectGraph",
    "PhysicalDraftCurrentReads",
    "PhysicalDraftEdits",
    "PhysicalDraftRevisionReads",
];

fn evaluate_name(name: &str) -> bool {
    name.starts_with("evaluate")
        || name.starts_with("apply_semantics")
        || name.starts_with("apply_stored_semantics")
}

fn uses_package_evaluate(f: &File) -> bool {
    let mut names = scan::qualified_names(&f.code, "intersect_text::");
    if f.krate() == "intersect-text" {
        names.extend(scan::qualified_names(&f.code, "crate::"));
        names.extend(scan::qualified_names(&f.code, "super::"));
    }
    names.into_iter().any(evaluate_name)
}

fn uses_draft_language_bind(f: &File) -> bool {
    f.code.match_indices("DraftLanguage::bind(").any(|(at, _)| {
        !f.code[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

#[test]
fn old_parse_path_users_do_not_grow() -> Result<(), String> {
    scan::check(&[
        Entry {
            name: "PhysicalDraftLanguage",
            src: 0,
            examples: 0,
            uses: |f: &File| f.has_word("PhysicalDraftLanguage"),
        },
        Entry {
            name: "draft::physical outputs",
            src: 0,
            examples: 0,
            uses: |f: &File| {
                f.has_word("draft::physical")
                    || PHYSICAL_DRAFT_OUTPUTS.iter().any(|w| f.has_word(w))
            },
        },
        Entry {
            name: "Package evaluate path",
            src: 5,
            examples: 0,
            uses: uses_package_evaluate,
        },
        Entry {
            name: "DraftCapability (shared with the frozen path)",
            src: 7,
            examples: 2,
            uses: |f: &File| f.has_word("DraftCapability"),
        },
        Entry {
            name: "DraftLanguage::bind (shared with the frozen path)",
            src: 1,
            examples: 2,
            uses: uses_draft_language_bind,
        },
    ])
}
