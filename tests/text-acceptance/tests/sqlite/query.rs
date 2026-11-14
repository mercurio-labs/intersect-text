#![cfg(not(target_arch = "wasm32"))]
use super::Store;
use intersect_model::core::{
    query::*,
    storage::{self, ModelStore, ModelView},
};

use intersect_model::query::PackageDerived;
use intersect_model::sqlite::SqliteStore;
use intersect_text::{EvaluationLimits, Source, evaluate_into};
use serde_json::json;

store_test! {
pub fn memory_sqlite_and_compatibility_query_results_agree_across_edits_and_rollback() {
    let p = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let sources = [
        Source {
            uri: "catalog.library".into(),
            text: include_str!("../../../../fixtures/library/catalog.library").into(),
        },
        Source {
            uri: "loans.library".into(),
            text: include_str!("../../../../fixtures/library/loans.library").into(),
        },
    ];
    let mut memory = crate::stores::store::<Store>(&p.schema).unwrap();
    let mut sqlite = SqliteStore::in_memory().unwrap();
    assert!(
        evaluate_into(&p, &sources, EvaluationLimits::native(), &mut memory)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    assert!(
        evaluate_into(&p, &sources, EvaluationLimits::native(), &mut sqlite)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    let plans = [
        json!({"schema_uri":p.schema.uri,"schema_revision":p.schema.revision,"from":"BookCopy","select":[{"name":"name","expression":{"op":"field","path":"name"}},{"name":"available","expression":{"op":"field","path":"available"}}],"order_by":[{"expression":{"op":"field","path":"name"},"descending":true}],"limit":1}),
        json!({"schema_uri":p.schema.uri,"schema_revision":p.schema.revision,"from":"Loan","filter":{"op":"compare","left":{"op":"field","path":"name"},"comparison":"eq","right":{"op":"literal","value":"first"}},"select":[{"name":"title","expression":{"op":"field","path":"copy.title"}},{"name":"member","expression":{"op":"field","path":"member.name"}}],"limit":50}),
        json!({"schema_uri":p.schema.uri,"schema_revision":p.schema.revision,"from":"Library","filter":{"op":"all","path":"copies","predicate":{"op":"compare","left":{"op":"field","path":"available"},"comparison":"eq","right":{"op":"literal","value":true}}},"select":[{"name":"name","expression":{"op":"field","path":"name"}}],"limit":50}),
    ];
    let parameters = Parameters::new();
    let provider = PackageDerived(&p);
    let mut outputs = Vec::new();
    for store in [&mut memory as &mut dyn ModelStore, &mut sqlite] {
        let token = store.begin().unwrap();
        for (step, changed) in [false, true, false].into_iter().enumerate() {
            if changed {
                storage::edit(
                    store,
                    &p.schema,
                    &[(
                        "loans.library#1".into(),
                        "copy".into(),
                        json!("catalog.library#2"),
                    )],
                )
                .unwrap();
            } else if step == 2 {
                store.rollback(&token).unwrap();
            }
            let exported = storage::export(store).unwrap();
            let view = ModelView::new(&exported).unwrap();
            let mut batch = Vec::new();
            for plan in &plans {
                let plan: QueryPlan = serde_json::from_value(plan.clone()).unwrap();
                let actual = plan
                    .execute(
                        &p.schema,
                        store,
                        &parameters,
                        &provider,
                        QueryLimits::default(),
                    )
                    .unwrap();
                let facade = plan
                    .execute(
                        &p.schema,
                        &view,
                        &parameters,
                        &provider,
                        QueryLimits::default(),
                    )
                    .unwrap();
                assert_eq!(actual.rows, facade.rows);
                assert_eq!(actual.has_more, facade.has_more);
                batch.push((actual.rows, actual.has_more));
            }
            outputs.push(batch);
        }
    }
    assert_eq!(outputs[0], outputs[3]);
    assert_eq!(outputs[1], outputs[4]);
    assert_eq!(outputs[2], outputs[5]);
    assert_eq!(outputs[0], outputs[2]);
    assert_ne!(outputs[0], outputs[1]);
}
}
