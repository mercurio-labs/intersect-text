use super::Store;
use intersect_model::core::storage::{self, Construction, ModelStore};
use intersect_model::initialization::PackageInitializers;
use intersect_text::grammar::Profile;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::json;
const ECORE: &str = include_str!("../../../../fixtures/xtext/simple-actions/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/simple-actions/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/simple-actions/model-behavior.rcore");
fn compile(xtext: &str) -> intersect_text::package::Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        xtext,
        RCORE_BEHAVIOR,
        Profile::NativeV1,
    )
    .unwrap()
}
fn sources() -> Vec<Source> {
    vec![
        Source {
            uri: "actions.kerml".into(),
            text: include_str!("../../../../fixtures/xtext/simple-actions/example.kerml").into(),
        },
        Source {
            uri: "more.kerml".into(),
            text: include_str!("../../../../fixtures/xtext/simple-actions/more.kerml").into(),
        },
    ]
}
store_test! {
pub fn actions_select_concrete_classes_before_initializers_and_match_direct_construction() {
    let package = compile(XTEXT);
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR
        )
        .is_err()
    );
    assert_eq!(package.sources["xtext"], XTEXT);
    let grammar = package.grammar.as_ref().unwrap();
    assert_eq!(grammar.rules["Type"].class, "Element");
    assert_eq!(grammar.rules["Type"].construction_class(), "Type");
    let input = sources();
    let result = evaluate(&package, &input);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 6);
    for (index, id, class, name) in [
        (0, "actions.kerml#0", "Model", "Demo"),
        (1, "actions.kerml#1", "Type", "Vehicle"),
        (2, "actions.kerml#2", "Type", "Wheel"),
        (3, "actions.kerml#3", "Feature", "wheel"),
        (4, "more.kerml#0", "Model", "Library"),
        (5, "more.kerml#1", "Type", "Spare"),
    ] {
        let element = &result.model.elements[index];
        assert_eq!(element.id, id);
        assert_eq!(element.class, class);
        assert_eq!(element.values["name"], name);
        assert_eq!(element.type_identity, package.schema.identity(class));
        assert_eq!(element.initialization.is_some(), class == "Type");
    }
    assert_eq!(result.model.elements[1].values["origin"], "authored");
    assert_eq!(result.model.elements[2].values["origin"], "constructor");
    assert_eq!(result.model.elements[3].values["target"], "actions.kerml#2");
    assert_eq!(result.references.len(), 1);
    assert_eq!(
        &input[0].text[result.references[0].start..result.references[0].end],
        "Wheel"
    );
    let initializers = PackageInitializers::new(&package).unwrap();
    let mut direct = crate::stores::store::<Store>(&package.schema).unwrap();
    storage::atomic(&mut direct, |store| {
        for element in &result.model.elements {
            if element.class == "Type" {
                initializers.create_installed(
                    store,
                    &package.schema,
                    "Type",
                    Construction {
                        id: &element.id,
                        resource: &element.resource,
                        authored: &[],
                    },
                    None,
                    &mut intersect_model::core::query::QueryBudget::new(1000),
                )?;
                store.set_span(&element.id, element.start, element.end)?;
            } else {
                store.create_element(
                    &package.schema,
                    &element.id,
                    &element.class,
                    &element.resource,
                    element.start,
                    element.end,
                )?;
            }
        }
        for element in &result.model.elements {
            storage::set_with_opposite(
                store,
                &package.schema,
                &element.id,
                "name",
                element.values["name"].clone(),
            )?;
        }
        for (id, field, value) in [
            (
                "actions.kerml#0",
                "members",
                json!(["actions.kerml#1", "actions.kerml#2", "actions.kerml#3"]),
            ),
            ("more.kerml#0", "members", json!(["more.kerml#1"])),
            ("actions.kerml#1", "origin", json!("authored")),
            ("actions.kerml#3", "target", json!("actions.kerml#2")),
        ] {
            storage::set_with_opposite(store, &package.schema, id, field, value)?;
        }
        Ok(())
    })
    .unwrap();
    storage::validate(&direct, &package.schema).unwrap();
    let direct = storage::export(&direct).unwrap();
    for (parsed, constructed) in result.model.elements.iter().zip(&direct.elements) {
        assert_eq!(parsed, constructed);
    }
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(evaluate(&replay, &input).model, result.model);
}
}
control_test! {
pub fn grouped_leading_actions_and_empty_action_roots_work() {
    let grouped = compile(&XTEXT.replace("{k::Type}", "(({k::Type}))"));
    let result = evaluate(&grouped, &sources());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let header = XTEXT.split("Model returns").next().unwrap();
    let empty = compile(&format!("{header}Model returns k::Model: {{k::Model}};"));
    let result = evaluate(
        &empty,
        &[Source {
            uri: "empty".into(),
            text: String::new(),
        }],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    assert_eq!(result.model.elements[0].id, "empty#0");
    assert_eq!(
        (result.model.elements[0].start, result.model.elements[0].end),
        (0, 0)
    );
}
}
control_test! {
pub fn unsupported_action_placements_types_and_nullable_recursion_reject() {
    for replacement in [
        "{k::Element}",
        "{k::Model}",
        "{k::Missing}",
        "({k::Type})?",
        "({k::Type})*",
        "{k::Type} {k::Type}",
        "'before' {k::Type}",
        "{k::Type.name=current}",
        "{k::Type.name+=current}",
    ] {
        assert!(
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
                ECORE,
                &XTEXT.replace("{k::Type}", replacement),
                RCORE_BEHAVIOR,
                Profile::NativeV1
            )
            .is_err(),
            "{replacement}"
        );
    }
    let interface = ECORE.replace(
        "name=\"Type\" eSuperTypes",
        "name=\"Type\" interface=\"true\" eSuperTypes",
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            &interface,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV1
        )
        .is_err()
    );
    let fragment = XTEXT.replace(
        "fragment Identification returns k::Element:",
        "fragment Identification returns k::Element: {k::Type}",
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &fragment,
            RCORE_BEHAVIOR,
            Profile::NativeV1
        )
        .is_err()
    );
    let header = XTEXT.split("Model returns").next().unwrap();
    let nullable = format!(
        "{header}Model returns k::Model: {{k::Model}} members+=Empty*; Empty returns k::Element: {{k::Type}};"
    );
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &nullable,
            RCORE_BEHAVIOR,
            Profile::NativeV1
        )
        .is_err()
    );
    let recursive = format!("{header}Model returns k::Model: {{k::Model}} members+=Model;");
    let recursive_schema = ECORE.replacen("eType=\"#//Element\"", "eType=\"#//Model\"", 1);
    let error = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        &recursive_schema,
        &recursive,
        RCORE_BEHAVIOR,
        Profile::NativeV1,
    )
    .unwrap_err();
    assert!(format!("{error:?}").contains("Left recursion"), "{error:?}");
}
}
control_test! {
pub fn action_construction_preserves_rollback_ordinals_and_object_limits() {
    let package = compile(XTEXT);
    let source = Source {
        uri: "rollback".into(),
        text: "package Demo { type Wheel; feature wheel : Wheel; type Spare; }".into(),
    };
    let mut limits = EvaluationLimits::native();
    limits.elements_per_resource = Some(4);
    let result = evaluate_with_limits(&package, std::slice::from_ref(&source), limits);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 4);
    assert_eq!(result.model.elements[3].id, "rollback#3");
    limits.elements_per_resource = Some(3);
    let rejected = evaluate_with_limits(&package, std::slice::from_ref(&source), limits);
    assert!(!rejected.diagnostics.is_empty());
    assert!(rejected.model.elements.is_empty());
    let rejected = evaluate(
        &package,
        &[Source {
            uri: "rollback".into(),
            text: source.text.replace("type Spare;", "type Spare origin ;"),
        }],
    );
    assert!(!rejected.diagnostics.is_empty());
    assert!(rejected.model.elements.is_empty());
}
}

control_test! {
pub fn action_execution_keeps_token_step_and_nesting_boundaries() {
    let package = compile(XTEXT);
    let source = Source {
        uri: "limits".into(),
        text: "package Demo { type Wheel; feature wheel : Wheel; type Spare; }".into(),
    };
    let input = std::slice::from_ref(&source);
    let mut limits = EvaluationLimits::native();
    let tokens = intersect_text::grammar::lex(&source.text)
        .unwrap()
        .len();
    limits.tokens_per_resource = Some(tokens);
    assert!(
        evaluate_with_limits(&package, input, limits)
            .diagnostics
            .is_empty()
    );
    limits.tokens_per_resource = Some(tokens - 1);
    assert!(
        evaluate_with_limits(&package, input, limits)
            .model
            .elements
            .is_empty()
    );
    limits = EvaluationLimits::native();
    limits.nesting = 4;
    assert!(
        evaluate_with_limits(&package, input, limits)
            .diagnostics
            .is_empty()
    );
    limits.nesting = 3;
    let rejected = evaluate_with_limits(&package, input, limits);
    assert!(
        rejected
            .diagnostics
            .iter()
            .any(|d| d.message.contains("nesting"))
    );
    assert!(rejected.model.elements.is_empty());
    let mut low = 0;
    let mut high = 100_000;
    while low < high {
        let middle = (low + high) / 2;
        limits = EvaluationLimits::native();
        limits.steps_per_resource = Some(middle);
        if evaluate_with_limits(&package, input, limits)
            .diagnostics
            .is_empty()
        {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    assert!(low > 0 && low < 100_000);
    limits.steps_per_resource = Some(low);
    assert!(
        evaluate_with_limits(&package, input, limits)
            .diagnostics
            .is_empty()
    );
    limits.steps_per_resource = Some(low - 1);
    let rejected = evaluate_with_limits(&package, input, limits);
    assert!(!rejected.diagnostics.is_empty());
    assert!(rejected.model.elements.is_empty());
}
}
