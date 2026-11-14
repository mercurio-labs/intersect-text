//! Parsing-to-generated-binding integration; generated bindings remain in Model.
use intersect_model::core::{
    api::{BasicModel, TransactionalModel},
    query::{NoDerivedValues, Parameters, QueryBudget, QueryLimits},
    storage::{self, ModelRead},
};
use intersect_model::query::PackageDerived;
use intersect_rust_bindings_tests::library::{self, BookCopy, Loan};
use intersect_text::{Source, compiler::compile_rcore_language as compile, evaluate_stored};
use serde_json::json;

#[test]
fn generated_queries_and_derived_getters_use_existing_providers() -> Result<(), String> {
    let package = compile(
        library::RCORE_SOURCE.ok_or("Missing source")?,
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .map_err(|e| format!("{e:?}"))?;
    let parsed = evaluate_stored(
        &package,
        &[
            Source {
                uri: "catalog.library".into(),
                text: include_str!("../../../../fixtures/library/catalog.library").into(),
            },
            Source {
                uri: "loans.library".into(),
                text: include_str!("../../../../fixtures/library/loans.library").into(),
            },
        ],
        intersect_text::EvaluationLimits::native(),
    )?;
    assert!(parsed.evaluation.diagnostics.is_empty());
    let provider = PackageDerived(&package);
    let query = BookCopy::query_binding(&package.schema)?
        .query()
        .filter(BookCopy::field_available(&package.schema)?.eq(true))
        .select((
            BookCopy::field_name(&package.schema)?,
            BookCopy::field_title(&package.schema)?,
        ));
    let result = query.execute(
        &package.schema,
        &parsed.store,
        &Parameters::new(),
        &provider,
        QueryLimits::default(),
    )?;
    assert_eq!(
        result.rows,
        vec![(
            "earthsea".to_owned(),
            Some("A Wizard of Earthsea".to_owned())
        )]
    );
    let ids = parsed
        .store
        .query(storage::Query::Value("name", &json!("earthsea")))?;
    let copy = BookCopy::bind(&parsed.store, ids.first().ok_or("Missing copy")?)?;
    assert_eq!(
        copy.available(&parsed.store, &provider, &mut QueryBudget::new(100))?,
        Some(true)
    );
    assert!(
        copy.available(&parsed.store, &NoDerivedValues, &mut QueryBudget::new(100))
            .is_err()
    );
    assert!(
        copy.available(&parsed.store, &provider, &mut QueryBudget::new(0))
            .is_err()
    );
    assert!(copy.is_set_available(&parsed.store, &provider, &mut QueryBudget::new(100))?);
    assert!(
        copy.is_set_available(&parsed.store, &NoDerivedValues, &mut QueryBudget::new(100))
            .is_err()
    );
    assert!(
        copy.is_set_available(&parsed.store, &provider, &mut QueryBudget::new(0))
            .is_err()
    );
    let owner = copy.library(&parsed.store)?.ok_or("Missing owner")?;
    assert_eq!(
        owner.copy_count(&parsed.store, &provider, &mut QueryBudget::new(100))?,
        Some(2)
    );
    let loans = parsed
        .store
        .query(storage::Query::Value("name", &json!("first")))?;
    let loan = Loan::bind(&parsed.store, loans.first().ok_or("Missing loan")?)?;
    let borrower = loan.member(&parsed.store)?;
    assert_eq!(borrower.name(&parsed.store)?, "ada");
    let previous = loan.copy(&parsed.store)?;
    assert_eq!(previous.name(&parsed.store)?, "dune");
    let mut model = TransactionalModel::new(BasicModel::from_store(
        package.schema.clone(),
        parsed.store,
    )?);
    model.transaction(|edit| {
        copy.set_title(edit, Some("Edited parsed object"))?;
        loan.set_copy(edit, &copy)?;
        loan.set_member(edit, &borrower)
    })?;
    assert_eq!(copy.title(&model)?, Some("Edited parsed object".into()));
    assert!(!copy.is_set_available(&model, &provider, &mut QueryBudget::new(100))?);
    assert_eq!(
        copy.available(&model, &provider, &mut QueryBudget::new(100))?,
        Some(false)
    );
    assert_eq!(
        previous.available(&model, &provider, &mut QueryBudget::new(100))?,
        Some(true)
    );
    Ok(())
}
