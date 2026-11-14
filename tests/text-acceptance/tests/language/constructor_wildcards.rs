#[path = "common/constructor_wildcards.rs"]
mod common;
use common::{Fixture, app, scalar};
use intersect_model::conversion::with_catalog_policies;
use intersect_model::core::model::Model;
use intersect_text::compiler::compile_bundle_with_conversions;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};
use serde_json::json;
fn consume(f: Fixture) {
    let package = with_catalog_policies(
        &f.input.bundle,
        &[],
        &f.applications,
        &f.constructors,
        |registry| {
            let package = compile_bundle_with_conversions(f.input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            let schema = &package.schema;
            let mut model = Model::default();
            model.create(
                schema,
                "h",
                schema.class_key_for_type(&f.input.materialized_types[0])?,
                "memory:model",
                0,
                0,
            )?;
            assert_eq!(
                model.elements[0].values["values"],
                json!(
                    registry
                        .applied()
                        .default_value(&f.applications[0].application)?
                )
            );
            model.edit_with_conversions(
                schema,
                &[("h".into(), "values".into(), f.value.clone())],
                registry,
            )?;
            assert_eq!(model.elements[0].values["values"], f.value);
            let before = model.clone();
            let mut wrong = f.value.clone();
            wrong["converted"]["value"]["items"][0]["converted"]["datatype"]["path"] = json!([0]);
            assert!(
                model
                    .edit_with_conversions(
                        schema,
                        &[
                            ("h".into(), "name".into(), json!("rollback")),
                            ("h".into(), "values".into(), wrong)
                        ],
                        registry
                    )
                    .is_err()
            );
            assert_eq!(model, before);
            model.validate_with_conversions(schema, registry)?;
            assert!(
                registry
                    .applied()
                    .parse(&f.applications[0].application, "[]")
                    .is_err()
            );
            let parsed = evaluate_with_conversions(
                &package,
                &[Source {
                    uri: "memory:model".into(),
                    text: "holder Demo".into(),
                }],
                EvaluationLimits::native(),
                registry,
            );
            assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
            parsed.model.validate_with_conversions(schema, registry)?;
            Ok(package)
        },
    )
    .unwrap();
    let retained: Package = serde_json::from_value(json!(package)).unwrap();
    let input = retained.compilation.clone().unwrap();
    with_catalog_policies(
        &input.bundle,
        &retained.schema.conversion_contracts,
        &retained.schema.applied_conversion_contracts,
        &retained.schema.constructor_contracts,
        |registry| {
            let rebuilt = compile_bundle_with_conversions(input.clone(), registry)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(json!(retained), json!(rebuilt));
            Ok(())
        },
    )
    .unwrap();
}
fn rejects(f: Fixture) {
    assert!(
        with_catalog_policies(
            &f.input.bundle,
            &[],
            &f.applications,
            &f.constructors,
            |_| Ok(())
        )
        .is_err()
    );
}

#[test]
fn unbounded_wildcard_installed_aliases_consume_two_actual_member_domains() {
    let bound = r##"<eBounds eClassifier="#//EList"><eTypeArguments/></eBounds>"##;
    for n in [0, 1] {
        consume(common::fixture(bound, app(3, vec![scalar(n)]), &[2, 3]));
    }
    let f = common::fixture(bound, app(3, vec![scalar(0)]), &[2, 3]);
    assert!(with_catalog_policies(&f.input.bundle, &[], &f.applications, &[], |_| Ok(())).is_err());
}
#[test]
fn upper_and_lower_wildcards_use_installed_direction_and_matching_rules() {
    let upper = r##"<eBounds eClassifier="#//Collection"><eTypeArguments><eUpperBound eClassifier="#//List"><eTypeArguments eClassifier="#//Text"/></eUpperBound></eTypeArguments></eBounds>"##;
    consume(common::fixture(
        upper,
        app(2, vec![app(2, vec![scalar(0)])]),
        &[2, 4, 5],
    ));
    rejects(common::fixture(
        upper,
        app(2, vec![app(2, vec![scalar(1)])]),
        &[2, 4, 5],
    ));
    let reverse = r##"<eBounds eClassifier="#//EList"><eTypeArguments/></eBounds>"##;
    rejects(common::fixture(reverse, app(5, vec![scalar(0)]), &[2, 5]));
    let lower = r##"<eBounds eClassifier="#//Collection"><eTypeArguments><eLowerBound eClassifier="#//EList"><eTypeArguments eClassifier="#//Text"/></eLowerBound></eTypeArguments></eBounds>"##;
    // Pinned Ecore reverses matching for lower bounds; nested directional
    // subtyping is not constructor equivalence under that matching relation.
    consume(common::fixture(
        lower,
        app(4, vec![app(3, vec![scalar(0)])]),
        &[2, 3, 4, 5],
    ));
    rejects(common::fixture(
        lower,
        app(4, vec![app(4, vec![scalar(0)])]),
        &[2, 4, 5],
    ));
    rejects(common::fixture(lower, app(4, vec![scalar(1)]), &[2, 4, 5]));
}
#[test]
fn nested_invariance_and_map_wildcard_positions_survive_actual_mutation() {
    let bound = r##"<eBounds eClassifier="#//EList"><eTypeArguments eClassifier="#//EList"><eTypeArguments eClassifier="#//Text"/></eTypeArguments></eBounds>"##;
    consume(common::fixture(
        bound,
        app(3, vec![app(3, vec![scalar(0)])]),
        &[2, 3],
    ));
    rejects(common::fixture(
        bound,
        app(3, vec![app(4, vec![scalar(0)])]),
        &[2, 3, 4],
    ));
    rejects(common::fixture(
        bound,
        app(3, vec![app(3, vec![scalar(1)])]),
        &[2, 3],
    ));
    let map = r##"<eBounds eClassifier="#//Map"><eTypeArguments><eUpperBound eClassifier="#//Text"/></eTypeArguments><eTypeArguments/></eBounds>"##;
    consume(common::fixture(
        map,
        app(7, vec![scalar(0), scalar(1)]),
        &[6, 7],
    ));
    rejects(common::fixture(
        map,
        app(7, vec![scalar(1), scalar(0)]),
        &[6, 7],
    ));
}
