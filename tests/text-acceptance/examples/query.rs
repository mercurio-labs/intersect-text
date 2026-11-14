//! Handwritten Library marker and native typed-query example.
use intersect_model::core::query::{Binding, Entity, Parameters, QueryLimits};

use intersect_model::query::PackageDerived;
use intersect_text::{EvaluationLimits, Source, evaluate_stored};

enum BookCopy {}
impl Entity for BookCopy {
    const CLASS: &'static str = "BookCopy";
    const SCHEMA_URI: &'static str = "urn:intersect:example:library:1";
}
fn main() -> Result<(), String> {
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../fixtures/library/library.ecore"),
        include_str!("../../../fixtures/library/library.xtext"),
        include_str!("../../../fixtures/library/library-behavior.rcore"),
    )
    .map_err(|e| format!("{e:?}"))?;
    let sources = [
        Source {
            uri: "catalog.library".into(),
            text: include_str!("../../../fixtures/library/catalog.library").into(),
        },
        Source {
            uri: "loans.library".into(),
            text: include_str!("../../../fixtures/library/loans.library").into(),
        },
    ];
    let model = evaluate_stored(&package, &sources, EvaluationLimits::native())?;
    if !model.evaluation.diagnostics.is_empty() {
        return Err(format!("{:?}", model.evaluation.diagnostics));
    }
    let copy = Binding::<BookCopy>::new(&package.schema)?;
    let name = copy.field::<String>("name")?;
    let query = copy
        .query()
        .filter(copy.field::<Option<bool>>("available")?.eq(true))
        .select((name.clone(), copy.field::<Option<String>>("title")?))
        .order_by(name.ascending())
        .limit(50);
    let result = query.execute(
        &package.schema,
        &model.store,
        &Parameters::new(),
        &PackageDerived(&package),
        QueryLimits::default(),
    )?;
    for (name, title) in result.rows {
        println!("{name}: {}", title.as_deref().unwrap_or("(untitled)"));
    }
    Ok(())
}
