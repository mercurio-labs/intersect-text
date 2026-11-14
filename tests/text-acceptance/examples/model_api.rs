//! Handwritten Library facade over basic, transactional, and versioned APIs.
//! Definitions are compiled from the existing fixtures; this Rust is not generated.
use intersect_model::core::api::{BasicModel, ModelEdit, TransactionalModel, VersionedModel};
use intersect_model::core::storage::{self, ModelRead, StorageResult};
use intersect_text::package::Package;
use intersect_text::{StoredEvaluation, apply_stored_semantics};
use serde_json::{Value, json};

const LIBRARY: &str = "catalog.library#library";
const COPY: &str = "catalog.library#dune";
const MEMBER: &str = "catalog.library#ada";
const LOAN: &str = "loans.library#first";
const DUPLICATE: &str = "loans.library#duplicate";

// A tiny external-domain-style helper. Every editing capability uses this implementation.
struct BookCopy {
    id: String,
}
impl BookCopy {
    fn create(model: &mut impl ModelEdit, id: &str, name: &str) -> StorageResult<Self> {
        model.create(id, "BookCopy", "catalog.library")?;
        model.set(id, "name", json!(name))?;
        Ok(Self { id: id.into() })
    }

    fn set_title(&self, model: &mut impl ModelEdit, title: &str) -> StorageResult<()> {
        model.set(&self.id, "title", json!(title))
    }
}

fn lend(model: &mut impl ModelEdit, id: &str, name: &str) -> StorageResult<()> {
    model.create(id, "Loan", "loans.library")?;
    model.set_many(&[
        (id, "name", json!(name)),
        (id, "copy", json!(COPY)),
        (id, "member", json!(MEMBER)),
    ])
}

fn semantics(package: &Package, model: &impl ModelRead) -> StorageResult<StoredEvaluation> {
    let mut results = StoredEvaluation::default();
    apply_stored_semantics(package, model, &mut results)?;
    if !results.diagnostics.is_empty() {
        return Err(format!(
            "Library validation rejected the candidate: {:?}",
            results.diagnostics
        ));
    }
    Ok(results)
}

fn check_value(
    model: &impl ModelRead,
    id: &str,
    feature: &str,
    expected: Value,
) -> StorageResult<()> {
    let actual = model.value(id, feature)?;
    if actual != Some(expected.clone()) {
        return Err(format!(
            "Unexpected {id}.{feature}: wanted {expected}, got {actual:?}"
        ));
    }
    Ok(())
}

fn check_available(results: &StoredEvaluation, expected: bool) -> StorageResult<()> {
    let actual = results.derived.get(COPY).and_then(|v| v.get("available"));
    if actual != Some(&json!(expected)) {
        return Err(format!(
            "Unexpected derived availability: wanted {expected}, got {actual:?}"
        ));
    }
    Ok(())
}

fn rejected(result: StorageResult<()>, reason: &str) -> StorageResult<()> {
    match result {
        Err(error) if error.contains(reason) => Ok(()),
        Err(error) => Err(format!(
            "Expected rejection containing '{reason}', got: {error}"
        )),
        Ok(()) => Err(format!(
            "Expected rejection containing '{reason}', but edit succeeded"
        )),
    }
}

fn unchanged(
    model: &impl ModelRead,
    before: &intersect_model::core::model::Model,
) -> StorageResult<()> {
    if &storage::export(model)? != before {
        return Err("Rejected operation leaked changes into the committed model".into());
    }
    Ok(())
}

fn main() -> StorageResult<()> {
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../fixtures/library/library.ecore"),
        include_str!("../../../fixtures/library/library.xtext"),
        include_str!("../../../fixtures/library/library-behavior.rcore"),
    )
    .map_err(|error| format!("Could not compile Library definitions: {error:?}"))?;

    // Plain construction permits drafts with missing required values.
    let mut basic = BasicModel::new(package.schema.clone())?;
    basic.create(LIBRARY, "Library", "catalog.library")?;
    rejected(basic.validate(), "Multiplicity violation")?;
    basic.set(LIBRARY, "name", json!("Catalog"))?;
    println!("BASIC: a draft can omit a required name; explicit validation detects it.");

    let copy = BookCopy::create(&mut basic, COPY, "dune")?;
    copy.set_title(&mut basic, "Dune")?;
    basic.create(MEMBER, "Member", "catalog.library")?;
    basic.set(MEMBER, "name", json!("ada"))?;
    basic.set_many(&[
        (LIBRARY, "copies", json!([COPY])),
        (LIBRARY, "members", json!([MEMBER])),
    ])?;
    basic.validate()?;
    check_value(&basic, COPY, "title", json!("Dune"))?;
    check_value(&basic, COPY, "edition", json!(1))?;
    check_value(&basic, COPY, "library", json!(LIBRARY))?;
    check_available(&semantics(&package, &basic)?, true)?;
    println!("BASIC: Dune, edition=1; containment/opposite linked; available=true.");

    // The exact same typed helper edits a checked transaction context.
    let mut managed = TransactionalModel::new(basic);
    managed.transaction_with(
        |edit| {
            copy.set_title(edit, "Dune, annotated")?;
            lend(edit, LOAN, "first")?;
            edit.set(LIBRARY, "loans", json!([LOAN]))?;
            check_value(edit, COPY, "title", json!("Dune, annotated"))?;
            Ok(())
        },
        |edit| semantics(&package, edit).map(|_| ()),
    )?;
    check_value(&managed, COPY, "title", json!("Dune, annotated"))?;
    check_value(&managed, LOAN, "copy", json!(COPY))?;
    check_available(&semantics(&package, &managed)?, false)?;
    println!("TRANSACTION: title and loan committed together; available=false.");

    // Callback failure rolls back earlier successful operations in the batch.
    let committed = storage::export(&managed)?;
    rejected(
        managed.transaction(|edit| {
            copy.set_title(edit, "Discard this title")?;
            edit.set(COPY, "edition", json!(9))?;
            Err("Demonstration cancellation".into())
        }),
        "Demonstration cancellation",
    )?;
    unchanged(&managed, &committed)?;
    println!("ROLLBACK: callback error preserved the complete committed model.");

    // Setters allow the draft, but required lower bounds are enforced at commit.
    rejected(
        managed.transaction(|edit| {
            copy.set_title(edit, "Also discard this title")?;
            edit.set(COPY, "name", Value::Null)?;
            Ok(())
        }),
        "Multiplicity violation",
    )?;
    unchanged(&managed, &committed)?;
    println!("ROLLBACK: missing required name failed structural commit validation.");

    // The duplicate is structurally valid; the Library package owns this rule.
    rejected(
        managed.transaction_with(
            |edit| {
                copy.set_title(edit, "Discard duplicate-loan proposal")?;
                lend(edit, DUPLICATE, "duplicate")?;
                edit.set(LIBRARY, "loans", json!([LOAN, DUPLICATE]))?;
                Ok(())
            },
            |edit| semantics(&package, edit).map(|_| ()),
        ),
        "A book copy cannot have more than one active loan.",
    )?;
    unchanged(&managed, &committed)?;
    check_available(&semantics(&package, &managed)?, false)?;
    println!("ROLLBACK: Library semantic rule rejected a duplicate active loan.");
    // Versioning is another optional owner. BasicModel has no versioning methods.
    let mut versioned = VersionedModel::new(managed.into_basic())?;
    let baseline = versioned.snapshot()?;
    let mut proposed = versioned.fork(&baseline)?;
    let mut sibling = versioned.fork(&baseline)?;
    copy.set_title(&mut proposed, "Dune, revised")?;
    copy.set_title(&mut sibling, "Dune, alternate")?;
    check_value(&baseline, COPY, "title", json!("Dune, annotated"))?;
    check_value(&proposed, COPY, "title", json!("Dune, revised"))?;
    check_value(&sibling, COPY, "title", json!("Dune, alternate"))?;
    unchanged(&versioned.snapshot()?, &committed)?;
    println!("FORKS: two private titles; the snapshot and published model stay unchanged.");

    let published_version = versioned.commit_with(proposed, |candidate| {
        semantics(&package, candidate).map(|_| ())
    })?;
    let published = versioned.snapshot()?;
    if published.version() != published_version || published.version() == baseline.version() {
        return Err("Commit did not publish a distinct model version".into());
    }
    check_value(&published, COPY, "title", json!("Dune, revised"))?;
    check_value(&baseline, COPY, "title", json!("Dune, annotated"))?;
    check_value(&sibling, COPY, "title", json!("Dune, alternate"))?;
    check_available(&semantics(&package, &published)?, false)?;
    let published_model = storage::export(&published)?;
    println!("VERSION: validated title published; the older snapshot remains pinned.");

    rejected(versioned.commit(sibling).map(|_| ()), "stale")?;
    unchanged(&versioned.snapshot()?, &published_model)?;
    println!("VERSION REJECTED: stale sibling cannot replace the newer model.");

    let mut invalid = versioned.fork(&published)?;
    copy.set_title(&mut invalid, "Discard versioned duplicate-loan proposal")?;
    lend(&mut invalid, DUPLICATE, "duplicate")?;
    invalid.set(LIBRARY, "loans", json!([LOAN, DUPLICATE]))?;
    invalid.validate()?; // Structural validity does not establish domain validity.
    rejected(
        versioned
            .commit_with(invalid, |candidate| {
                semantics(&package, candidate).map(|_| ())
            })
            .map(|_| ()),
        "A book copy cannot have more than one active loan.",
    )?;
    let final_snapshot = versioned.snapshot()?;
    unchanged(&final_snapshot, &published_model)?;
    if final_snapshot.version() != published_version {
        return Err("Rejected candidate changed the published version".into());
    }
    unchanged(&baseline, &committed)?;
    check_available(&semantics(&package, &final_snapshot)?, false)?;
    println!("VERSION REJECTED: duplicate active loan; published state and version preserved.");
    println!("VERIFIED: one committed loan, original identity, available=false.");
    Ok(())
}
