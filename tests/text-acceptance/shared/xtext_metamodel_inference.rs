use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text as language;
use intersect_text::grammar::SourceLimits;
use intersect_text::inference::*;
use serde_json::{Value, json};
fn input(case: &Value) -> (DefinitionBundle, SourceId) {
    let original = DefinitionSource::new(
        "memory:inference-import.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/xtext/metamodel-inference/imported.ecore"),
    );
    let report =
        intersect_model::migration::migrate_ecore(original, "memory:inference-import.rcore")
            .unwrap();
    let imported = report.converted.unwrap();
    let mut source = DefinitionSource::new(
        format!("memory:{}.xtext", case["id"].as_str().unwrap()),
        SourceKind::Xtext,
        case["source"].as_str().unwrap(),
    );
    source.dependencies.push(imported.id.clone());
    let id = source.id.clone();
    (
        DefinitionBundle {
            version: 1,
            sources: vec![source, imported.clone()],
            namespaces: vec![intersect_model::definitions::bundle::NamespaceBinding {
                namespace: "urn:intersect:inference:imported".into(),
                source: imported.id,
            }],
        },
        id,
    )
}
pub(crate) fn projection(packages: &[Metamodel]) -> Value {
    json!(packages.iter().map(|p|json!({"uri":p.namespace,"name":p.name,"alias":p.alias,"classifiers":p.classifiers.iter().map(|c|json!({
        "name":c.id.name,"kind":if c.kind==ClassifierKind::Enumeration{"EEnum"}else{"EClass"},
        "super_types":c.parents.iter().map(TypeId::identity).collect::<Vec<_>>(),
        "features":c.features.iter().map(|f|json!({"name":f.name,"type":f.target.identity(),"reference":f.reference,"containment":f.containment,
            "lower":0,"upper":if f.many{-1}else{1}})).collect::<Vec<_>>(),
        "literals":c.literals.iter().map(|l|json!({"name":l.name,"literal":l.literal,"value":l.value})).collect::<Vec<_>>()
    })).collect::<Vec<_>>() })).collect::<Vec<_>>())
}
pub(crate) fn expected(row: &Value) -> Value {
    json!(row["generated_packages"].as_array().unwrap().iter().map(|p|{
        let package=&p["package"];json!({"uri":package["uri"],"name":package["name"],"alias":p["alias"],"classifiers":package["classifiers"].as_array().unwrap().iter().map(|c|json!({
            "name":c["name"],"kind":c["kind"],"super_types":c.get("super_types").cloned().unwrap_or(json!([])),
            "features":c.get("features").and_then(Value::as_array).into_iter().flatten().map(|f|json!({"name":f["name"],"type":f["type"],"reference":f["kind"]=="EReference",
                "containment":f.get("containment").cloned().unwrap_or(json!(false)),"lower":f["lower"],"upper":f["upper"]})).collect::<Vec<_>>(),
            "literals":c.get("literals").and_then(Value::as_array).into_iter().flatten().map(|l|json!({"name":l["name"],"literal":l["literal"],"value":l["value"]})).collect::<Vec<_>>()
        })).collect::<Vec<_>>()})
    }).collect::<Vec<_>>())
}
// Verify the generated Rcore was consumed by the shared materializer, not
// just represented in an inference report. JVM metadata and resolveProxies have
// no complete runtime counterpart here and remain outside this projection.
pub(crate) fn materialized_projection(
    schema: &intersect_model::definitions::schema::Schema,
    metamodels: &[Metamodel],
    sources: &[DefinitionSource],
    row: &Value,
) {
    for (index, package) in metamodels.iter().enumerate() {
        let expected = &row["generated_packages"][index]["package"];
        for (classifier, expected) in package
            .classifiers
            .iter()
            .zip(expected["classifiers"].as_array().unwrap())
        {
            if classifier.kind != ClassifierKind::Class {
                continue;
            }
            let key = schema
                .class_key(&sources[index].id, &package.namespace, &classifier.id.name)
                .unwrap();
            let actual = &schema.classes[key];
            assert_eq!(json!(actual.abstract_type), expected["abstract"]);
            assert_eq!(json!(actual.interface_type), expected["interface"]);
            assert_eq!(
                actual.feature_order,
                classifier
                    .features
                    .iter()
                    .map(|f| f.name.clone())
                    .collect::<Vec<_>>()
            );
            for feature in expected["features"].as_array().unwrap() {
                let actual = &actual.features[feature["name"].as_str().unwrap()];
                assert_eq!(
                    json!({"lower": actual.lower, "upper": actual.upper, "ordered":actual.ordered,
                    "unique":actual.unique,"changeable":actual.changeable,"transient":actual.transient,
                    "volatile":actual.volatile,"derived":actual.derived,"unsettable":actual.unsettable}),
                    json!({"lower":feature["lower"],"upper":feature["upper"],"ordered":feature["ordered"],
                    "unique":feature["unique"],"changeable":feature["changeable"],"transient":feature["transient"],
                    "volatile":feature["volatile"],"derived":feature["derived"],"unsettable":feature["unsettable"]})
                );
            }
        }
    }
}
pub fn frozen_matrix() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/metamodel-inference/cases.json"
    ))
    .unwrap();
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/metamodel-inference/observations.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 41);
    assert_eq!(rows.len(), cases.len());
    let mut failures = vec![];
    let mut accepted = 0;
    for (case, row) in cases.iter().zip(&rows) {
        assert_eq!(case["id"], row["id"]);
        let (bundle, id) = input(case);
        match infer_metamodels(&bundle, &id, SourceLimits::default()) {
            Ok(report) => {
                if row["accepted"] != true {
                    failures.push(format!("{} incorrectly accepted", case["id"]));
                    continue;
                }
                accepted += 1;
                assert_eq!(
                    report.bundle.sources[..bundle.sources.len()],
                    bundle.sources
                );
                materialized_projection(
                    &report.schema,
                    &report.metamodels,
                    &report.generated_sources,
                    row,
                );
                let actual = projection(&report.metamodels);
                let expected = expected(row);
                if actual != expected {
                    failures.push(format!(
                        "{} differs\nactual {actual}\nexpected {expected}",
                        case["id"]
                    ));
                }
                for origin in &report.origins {
                    assert!(origin.generated.text(&report.bundle).is_ok());
                    for original in &origin.original {
                        assert!(original.text(&report.bundle).is_ok());
                    }
                }
            }
            Err(error) => {
                if row["accepted"] == true {
                    failures.push(format!("{} rejected: {}", case["id"], error.message));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(accepted, 27);
}

fn case_named(name: &str) -> Value {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/metamodel-inference/cases.json"
    ))
    .unwrap();
    cases.into_iter().find(|case| case["id"] == name).unwrap()
}
fn language(name: &str) -> InferredLanguage {
    let (bundle, grammar) = input(&case_named(name));
    InferenceRequest {
        version: 1,
        bundle,
        grammar,
    }
    .compile(SourceLimits::default())
    .unwrap_or_else(|errors| panic!("{name}: {errors:?}"))
}
fn execute(language: &InferredLanguage, text: &str) -> language::Evaluation {
    let serialized = serde_json::to_value(language.package()).unwrap();
    let restored = serde_json::from_value(serialized.clone()).unwrap();
    let restored = language::recompile_with_source_policy(
        &restored,
        language.package().source_policy.unwrap(),
    )
    .unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), serialized);
    let result = language::evaluate(
        &restored,
        &[language::Source {
            uri: "memory:model.test".into(),
            text: text.into(),
        }],
    );
    assert!(
        result.diagnostics.is_empty(),
        "{text}: {:?}",
        result.diagnostics
    );
    super::archives::model_roundtrip(language, text, &result);
    result
}
fn object<'a>(
    language: &InferredLanguage,
    result: &'a language::Evaluation,
    namespace: &str,
    name: &str,
) -> &'a intersect_model::core::model::Element {
    let key = class_key(language, namespace, name);
    result
        .model
        .elements
        .iter()
        .find(|value| value.class == key)
        .unwrap()
}
pub fn source_execution_and_replay() {
    let scalar = language("scalar_attributes");
    assert!(
        intersect_text::grammar::compile_with_profile(
            &scalar.definitions().original.text,
            &scalar.package().schema,
            intersect_text::grammar::Profile::NativeV56
        )
        .is_err()
    );
    let mut untrusted = serde_json::to_value(scalar.request()).unwrap();
    untrusted["schema"] = serde_json::to_value(&scalar.package().schema).unwrap();
    assert!(serde_json::from_value::<InferenceRequest>(untrusted).is_err());
    let result = execute(&scalar, "Alpha enabled 42");
    let model = object(
        &scalar,
        &result,
        "urn:intersect:inference:scalar_attributes",
        "Model",
    );
    assert_eq!(model.values["name"], "Alpha");
    assert_eq!(model.values["enabled"], true);
    assert_eq!(model.values["count"], 42);
    let containment = language("containment_and_cross_reference");
    let result = execute(&containment, "node Alpha node Beta uses Alpha");
    let model = object(
        &containment,
        &result,
        "urn:intersect:inference:containment_and_cross_reference",
        "Model",
    );
    assert_eq!(model.values["nodes"].as_array().unwrap().len(), 2);
    assert_eq!(
        model.values["uses"],
        json!([model.values["nodes"][0].clone()])
    );
    let repeated = execute(&containment, "node Alpha uses Alpha uses Alpha");
    let repeated = object(
        &containment,
        &repeated,
        "urn:intersect:inference:containment_and_cross_reference",
        "Model",
    );
    assert_eq!(
        repeated.values["uses"],
        json!([repeated.values["nodes"][0], repeated.values["nodes"][0]])
    );
    let enumeration = language("enum_duplicate_literal_name");
    let result = execute(&enumeration, "rouge");
    let model = object(
        &enumeration,
        &result,
        "urn:intersect:inference:enum_duplicate_literal_name",
        "Model",
    );
    let value: intersect_model::definitions::enums::EnumValue =
        serde_json::from_value(model.values["shade"].clone()).unwrap();
    let definition = enumeration
        .package()
        .schema
        .enumerations
        .values()
        .find(|e| e.name() == "Shade")
        .unwrap();
    assert_eq!(
        value,
        definition
            .value_by_id(&definition.by_name("RED").unwrap().declaration)
            .unwrap()
    );
    let multiple = language("multiple_generated_packages");
    let result = execute(&multiple, "Alpha");
    let child = object(
        &multiple,
        &result,
        "urn:intersect:inference:auxiliary",
        "Part",
    );
    assert_eq!(child.values["name"], "Alpha");
    let imported = language("imported_existing_feature");
    let result = execute(&imported, "Alpha Beta");
    let key = class_key(&imported, "urn:intersect:inference:imported", "External");
    assert_eq!(
        result
            .model
            .elements
            .iter()
            .filter(|value| value.class == key)
            .count(),
        2
    );
    let imported_source = &imported.request().bundle.sources[1].id;
    assert_eq!(
        &imported.package().schema.declarations[key].source,
        imported_source
    );
    let actions = language("assigned_action");
    let result = execute(&actions, "1 + 2 + 3");
    let numbers = class_key(
        &actions,
        "urn:intersect:inference:assigned_action",
        "Number",
    );
    let binary = class_key(
        &actions,
        "urn:intersect:inference:assigned_action",
        "Binary",
    );
    assert_eq!(
        result
            .model
            .elements
            .iter()
            .filter(|value| value.class == numbers)
            .count(),
        3
    );
    assert_eq!(
        result
            .model
            .elements
            .iter()
            .filter(|value| value.class == binary)
            .count(),
        2
    );
    let top = language("assigned_model_type_alternative");
    assert!(top.package().schema.reference_top.is_some());
    let result = execute(&top, "a Alpha");
    assert_eq!(
        object(
            &top,
            &result,
            "urn:intersect:inference:assigned_model_type_alternative",
            "A"
        )
        .values["name"],
        "Alpha"
    );
    let request: InferenceRequest =
        serde_json::from_value(serde_json::to_value(scalar.request()).unwrap()).unwrap();
    let rebuilt = request.compile(SourceLimits::default()).unwrap();
    assert_eq!(
        serde_json::to_value(scalar.package()).unwrap(),
        serde_json::to_value(rebuilt.package()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(scalar.definitions()).unwrap(),
        serde_json::to_value(rebuilt.definitions()).unwrap()
    );
    let mut stale = request.clone();
    stale.bundle.sources[0].text.push(' ');
    assert!(stale.compile(SourceLimits::default()).is_err());
    let bad = case_named("feature_type_conflict");
    let (bundle, grammar) = input(&bad);
    assert!(
        InferenceRequest {
            version: 1,
            bundle,
            grammar
        }
        .compile(SourceLimits::default())
        .is_err()
    );
    assert_eq!(execute(&scalar, "Alpha enabled 42").model.elements.len(), 1);
    // Derived declarations bind the complete imported revision context even
    // when namespace spellings and generated class text are unchanged.
    let mut revised = request.clone();
    let prior = revised.bundle.sources[1].id.clone();
    let mut imported = DefinitionSource::new(
        prior.uri.clone(),
        SourceKind::Rcore,
        format!(
            "{}\n// distinct imported source revision",
            revised.bundle.sources[1].text
        ),
    );
    imported.dependencies = revised.bundle.sources[1].dependencies.clone();
    let new_id = imported.id.clone();
    revised.bundle.sources[1] = imported;
    for source in &mut revised.bundle.sources {
        for dependency in &mut source.dependencies {
            if *dependency == prior {
                *dependency = new_id.clone();
            }
        }
    }
    for binding in &mut revised.bundle.namespaces {
        if binding.source == prior {
            binding.source = new_id.clone();
        }
    }
    let revised = revised.compile(SourceLimits::default()).unwrap();
    assert_ne!(
        scalar.definitions().generated_sources[0].id,
        revised.definitions().generated_sources[0].id
    );
    assert_eq!(
        execute(&revised, "Alpha enabled 42").model.elements.len(),
        1
    );
    let mut reordered = request.clone();
    reordered.bundle.sources.reverse();
    assert_eq!(
        scalar.definitions().generated_sources,
        reordered
            .compile(SourceLimits::default())
            .unwrap()
            .definitions()
            .generated_sources
    );
}
pub fn larger_source_policy() {
    let mut case = case_named("scalar_attributes");
    let original = case["source"].as_str().unwrap().to_owned();
    case["source"] = json!(format!("{original}\n/*{}*/", "x".repeat(260_000)));
    let (bundle, grammar) = input(&case);
    let request = InferenceRequest {
        version: 1,
        bundle,
        grammar,
    };
    assert!(request.compile(SourceLimits::default()).is_err());
    let limits = SourceLimits {
        source_bytes: request
            .bundle
            .sources
            .iter()
            .map(|s| s.text.len())
            .max()
            .unwrap()
            + 1024,
        declarations: 128,
    };
    // The same host policy also applies to imported canonical Rcore inputs.
    let mut request = request;
    let previous = request.bundle.sources[1].id.clone();
    let mut imported = DefinitionSource::new(
        previous.uri.clone(),
        SourceKind::Rcore,
        format!(
            "{}\n/*{}*/",
            request.bundle.sources[1].text,
            "y".repeat(260_000)
        ),
    );
    imported.dependencies = request.bundle.sources[1].dependencies.clone();
    for source in &mut request.bundle.sources {
        for dependency in &mut source.dependencies {
            if *dependency == previous {
                *dependency = imported.id.clone();
            }
        }
    }
    for binding in &mut request.bundle.namespaces {
        if binding.source == previous {
            binding.source = imported.id.clone();
        }
    }
    request.bundle.sources[1] = imported;
    let limits = SourceLimits {
        source_bytes: request
            .bundle
            .sources
            .iter()
            .map(|s| s.text.len())
            .max()
            .unwrap()
            + 1024,
        ..limits
    };
    let language = request.compile(limits).unwrap();
    assert!(intersect_text::recompile(language.package()).is_err());
    let restored =
        serde_json::from_value(serde_json::to_value(language.package()).unwrap()).unwrap();
    let restored = intersect_text::recompile_with_source_policy(
        &restored,
        intersect_text::package::CompilationSourcePolicy {
            structure_bytes: limits.source_bytes,
            xtext_bytes: limits.source_bytes,
            xtext_declarations: limits.declarations,
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(language.package()).unwrap()
    );
    let result = execute(&language, "Alpha enabled 42");
    assert_eq!(result.model.elements.len(), 1);
    assert!(language.recompile(limits).is_ok());
    assert!(language.recompile(SourceLimits::default()).is_err());
}

fn class_key<'a>(language: &'a InferredLanguage, namespace: &str, name: &str) -> &'a str {
    let source = if let Some(index) = language
        .definitions()
        .metamodels
        .iter()
        .position(|p| p.namespace == namespace)
    {
        &language.definitions().generated_sources[index].id
    } else {
        &language
            .request()
            .bundle
            .namespaces
            .iter()
            .find(|binding| binding.namespace == namespace)
            .unwrap()
            .source
    };
    language
        .package()
        .schema
        .class_key(source, namespace, name)
        .unwrap()
}
pub fn verify() -> usize {
    frozen_matrix();
    source_execution_and_replay();
    larger_source_policy();
    3
}
