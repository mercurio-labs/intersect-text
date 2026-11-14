use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use serde_json::{Value, json};
fn fixture() -> (
    Value,
    std::collections::BTreeMap<Vec<usize>, intersect_model::definitions::resources::DeclarationId>,
) {
    let source = DefinitionSource::new(
        "memory:runtime-operations",
        SourceKind::Ecore,
        include_str!("../../../fixtures/ecore-generics/runtime-operations.ecore"),
    );
    let (source, ids) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    (json!({"version":1,"sources":[source],"namespaces":[]}), ids)
}
fn ty(
    ids: &std::collections::BTreeMap<
        Vec<usize>,
        intersect_model::definitions::resources::DeclarationId,
    >,
    n: usize,
    arguments: Vec<Value>,
    raw: bool,
) -> Value {
    json!({"kind":"classifier","declaration":ids[&vec![n]],"arguments":arguments,"raw":raw})
}
fn dispatch(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
#[test]
fn runtime_method_bounds_preserve_receiver_domains_and_source_replay() {
    let (bundle, ids) = fixture();
    let raw = ty(&ids, 4, vec![], true);
    let both = ty(&ids, 2, vec![], false);
    let operation = json!(ids[&vec![4, 1]]);
    let application = json!({"receiver":raw,"operation":operation,"arguments":[both]});
    let report = dispatch(
        json!({"method":"ecore/runtime-operation-types","bundle":bundle,
        "receiver":raw,"operation":operation,"arguments":[both]}),
    );
    assert_eq!(report["ok"], true, "{report}");
    assert_eq!(report["executable"], false);
    assert_eq!(report["selection_verified"], false);
    assert_eq!(report["report"]["projection"]["receiver"], raw);
    let bad = dispatch(
        json!({"method":"ecore/runtime-operation-types","bundle":bundle,
        "receiver":raw,"operation":operation,"arguments":[ty(&ids,3,vec![],false)]}),
    );
    assert_eq!(bad["ok"], false, "{bad}");
    let materialized = dispatch(json!({"method":"ecore/materialize","bundle":bundle,
        "types":[raw,both],"instances":[{"id":"raw","type":raw}],
        "runtime_operation_applications":[application]}));
    assert_eq!(materialized["ok"], true, "{materialized}");
    assert_eq!(
        materialized["runtime_operation_applications"][0]["report"],
        report["report"]
    );
    let replay = dispatch(materialized["operation_replay"].clone());
    assert_eq!(replay["schema"], materialized["schema"]);
    assert_eq!(
        replay["runtime_operation_applications"],
        materialized["runtime_operation_applications"]
    );
    assert!(
        intersect_web::dispatch(
            &json!({"method":"ecore/generic-operation-types","bundle":bundle,
        "receiver":raw,"operation":operation,"arguments":[both]})
            .to_string()
        )
        .is_err()
    );
}
#[test]
fn owner_projection_selection_rejects_omitted_foreign_and_stale_owners() {
    let (bundle, ids) = fixture();
    let receiver = ty(&ids, 8, vec![], false);
    let operation = json!(ids[&vec![4, 1]]);
    let base = json!({"method":"ecore/runtime-operation-types","bundle":bundle,
        "receiver":receiver,"operation":operation,"arguments":[ty(&ids,2,vec![],false)]});
    assert_eq!(dispatch(base.clone())["ok"], false);
    let members = dispatch(
        json!({"method":"ecore/runtime-member-types","bundle":bundle,"receiver":receiver}),
    );
    assert_eq!(members["ok"], true, "{members}");
    let owners: Vec<_> = members["report"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|member| member["source"]["declaration"] == operation)
        .map(|member| member["source"]["owner"].clone())
        .collect();
    assert_eq!(owners.len(), 2);
    for owner in owners {
        let mut selected = base.clone();
        selected["owner"] = owner.clone();
        assert_eq!(dispatch(selected.clone())["ok"], true, "{selected}");
        selected["owner"]["declaration"]["source"]["sha256"] = json!("0".repeat(64));
        assert_eq!(dispatch(selected)["ok"], false);
    }
    let mut foreign = base;
    foreign["owner"] = ty(&ids, 3, vec![], false);
    assert_eq!(dispatch(foreign)["ok"], false);
}
