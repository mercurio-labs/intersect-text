//! The same exact source/model controls run natively and in raw WASM.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::NamespaceBinding;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar::SourceLimits;
use intersect_text::inference::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-composition/cases.json"
    ))
    .unwrap()
}
pub(crate) fn input(case: &Value) -> InferenceRequest {
    let original = DefinitionSource::new(
        "memory:composition-import.ecore",
        SourceKind::Ecore,
        include_str!("../../../fixtures/xtext/metamodel-inference/imported.ecore"),
    );
    let imported =
        intersect_model::migration::migrate_ecore(original, "memory:composition-import.rcore")
            .unwrap()
            .converted
            .unwrap();
    let mut sources = vec![];
    let mut ids = BTreeMap::<String, SourceId>::new();
    for source in case["sources"].as_array().unwrap() {
        let name = source["id"].as_str().unwrap();
        let mut definition = DefinitionSource::new(
            format!("memory:{}/{name}.xtext", case["id"].as_str().unwrap()),
            SourceKind::Xtext,
            source["source"].as_str().unwrap(),
        );
        definition.dependencies.push(imported.id.clone());
        for dependency in source["dependencies"].as_array().unwrap() {
            definition
                .dependencies
                .push(ids[dependency.as_str().unwrap()].clone());
        }
        ids.insert(name.into(), definition.id.clone());
        sources.push(definition);
    }
    let grammar = ids[case["entry"].as_str().unwrap()].clone();
    let namespaces = vec![NamespaceBinding {
        namespace: "urn:intersect:inference:imported".into(),
        source: imported.id.clone(),
    }];
    sources.push(imported);
    InferenceRequest {
        version: 1,
        bundle: DefinitionBundle {
            version: 1,
            sources,
            namespaces,
        },
        grammar,
    }
}
pub fn frozen_matrix() {
    let cases = cases();
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inference-composition/observations.json"
    ))
    .unwrap();
    assert_eq!(cases.len(), 31);
    assert_eq!(rows.len(), cases.len());
    let mut failures = vec![];
    let mut accepted = 0;
    for (case, row) in cases.iter().zip(&rows) {
        assert_eq!(case["id"], row["id"]);
        let request = input(case);
        match infer_metamodels(&request.bundle, &request.grammar, SourceLimits::default()) {
            Ok(report) => {
                if row["accepted"] != true {
                    failures.push(format!("{} incorrectly accepted", case["id"]));
                    continue;
                }
                accepted += 1;
                assert_eq!(
                    report.bundle.sources[..request.bundle.sources.len()],
                    request.bundle.sources
                );
                assert_eq!(
                    report.grammars.len(),
                    row["sources"].as_array().unwrap().len()
                );
                for (grammar, expected) in report
                    .grammars
                    .iter()
                    .zip(row["sources"].as_array().unwrap())
                {
                    assert_eq!(grammar.name, expected["grammar"]);
                    let packages = expected["metamodels"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|p| p["generated"] == true)
                        .map(|p| json!({"alias":p["alias"],"package":p["package"]}))
                        .collect::<Vec<_>>();
                    super::controls::materialized_projection(
                        &report.schema,
                        &grammar.metamodels,
                        &grammar.generated_sources,
                        &json!({"generated_packages":packages}),
                    );
                    let actual = super::controls::projection(&grammar.metamodels);
                    let expected_packages =
                        super::controls::expected(&json!({"generated_packages":packages}));
                    if actual != expected_packages {
                        failures.push(format!(
                            "{} / {} differs\nactual {actual}\nexpected {expected_packages}",
                            case["id"], grammar.name
                        ));
                    }
                    for rule in expected["rules"].as_array().unwrap() {
                        let binding = &grammar.rules[rule["name"].as_str().unwrap()];
                        assert_eq!(binding.declaration.source, grammar.source);
                        assert!(binding.declaration.text(&report.bundle).is_ok());
                        if json!(binding.target.as_ref().unwrap().identity()) != rule["type"] {
                            failures.push(format!(
                                "{} / {} rule {} target {} != {}",
                                case["id"],
                                grammar.name,
                                rule["name"],
                                binding.target.as_ref().unwrap().identity(),
                                rule["type"]
                            ));
                        }
                    }
                }
                for origin in &report.inherited_origins {
                    assert!(origin.origin.span.text(&report.bundle).is_ok());
                    assert!(report.bundle.source(&origin.declaration.source).is_ok());
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
    assert_eq!(accepted, 20);
}

fn execute(language: &InferredLanguage, text: &str) -> intersect_text::Evaluation {
    let result = intersect_text::evaluate(
        language.package(),
        &[intersect_text::Source {
            uri: "memory:composition-model.test".into(),
            text: text.into(),
        }],
    );
    assert!(
        result.diagnostics.is_empty(),
        "{text}: {:?}",
        result.diagnostics
    );
    result
}
pub fn execution_and_replay() {
    let scenarios = [
        (
            "inherited_model_call",
            "entry node Alpha node Beta",
            "base",
            "Node",
            "Alpha",
        ),
        (
            "inherited_call_without_import",
            "entry node Alpha",
            "base",
            "Node",
            "Alpha",
        ),
        (
            "reuse_parent_return",
            "entry node Alpha",
            "base",
            "Node",
            "Alpha",
        ),
        (
            "override_explicit_parent_type",
            "entry item Alpha",
            "base",
            "Node",
            "Alpha",
        ),
        (
            "override_implicit_child_type",
            "item Alpha",
            "child",
            "Node",
            "Alpha",
        ),
        (
            "new_subclass_of_parent",
            "special Alpha",
            "child",
            "Special",
            "Alpha",
        ),
        (
            "subclass_with_new_feature",
            "special Alpha 7",
            "child",
            "Special",
            "Alpha",
        ),
        (
            "explicit_super_model_call",
            "entry node Alpha",
            "base",
            "Node",
            "Alpha",
        ),
        ("inherited_enum", "red", "child", "Entry", ""),
        ("override_imported_enum", "warm", "child", "Entry", ""),
        ("override_terminal", "@alpha", "child", "Entry", "@alpha"),
        (
            "import_only_child",
            "entry node Alpha",
            "base",
            "Node",
            "Alpha",
        ),
        (
            "three_level_generated_packages",
            "node Alpha part Beta",
            "base",
            "Node",
            "Alpha",
        ),
        (
            "same_short_classifier_names",
            "node Alpha own Beta",
            "base",
            "Node",
            "Alpha",
        ),
        ("shared_import_alias", "node Alpha", "base", "Node", "Alpha"),
        (
            "aliased_generated_only",
            "node Alpha",
            "base",
            "Node",
            "Alpha",
        ),
        (
            "new_cross_reference_to_parent",
            "node Alpha use Alpha use Alpha",
            "base",
            "Node",
            "Alpha",
        ),
        (
            "parent_imports_external",
            "ext Alpha ext Beta",
            "original",
            "External",
            "Alpha",
        ),
        (
            "child_subclasses_original_import",
            "special Alpha 7",
            "child",
            "Special",
            "Alpha",
        ),
        (
            "same_name_imports_explicit_aliases",
            "base Alpha original Beta",
            "base",
            "External",
            "Alpha",
        ),
    ];
    let cases = cases();
    let mut failures = vec![];
    for (name, text, owner, class, value) in scenarios {
        let case = cases.iter().find(|c| c["id"] == name).unwrap();
        let request = input(case);
        let language = match request.compile(SourceLimits::default()) {
            Ok(language) => language,
            Err(errors) => {
                failures.push(format!("{name}: {errors:?}"));
                continue;
            }
        };
        let result = execute(&language, text);
        super::archives::model_roundtrip(&language, text, &result);
        let namespace = if owner == "original" {
            "urn:intersect:inference:imported".into()
        } else {
            format!("urn:intersect:inference:composition:{name}:{owner}")
        };
        let source = &language
            .definitions()
            .bundle
            .namespaces
            .iter()
            .find(|n| n.namespace == namespace)
            .unwrap()
            .source;
        let key = language
            .package()
            .schema
            .class_key(source, &namespace, class)
            .unwrap();
        let selected = result
            .model
            .elements
            .iter()
            .find(|object| {
                object.class == key
                    && (value.is_empty() || object.values.get("name") == Some(&json!(value)))
            })
            .unwrap();
        if name.contains("subclass") && name != "new_subclass_of_parent" {
            assert_eq!(selected.values["extra"], 7);
        }
        if name == "same_short_classifier_names" {
            let other = result
                .model
                .elements
                .iter()
                .find(|e| e.values.get("name") == Some(&json!("Beta")))
                .unwrap();
            assert_ne!(other.class, key);
            assert_eq!(
                language.package().schema.declarations[&other.class].namespace,
                format!("urn:intersect:inference:composition:{name}:child")
            );
        }
        if name == "same_name_imports_explicit_aliases" {
            let other = result
                .model
                .elements
                .iter()
                .find(|e| e.values.get("name") == Some(&json!("Beta")))
                .unwrap();
            assert_ne!(other.class, key);
            assert_eq!(
                language.package().schema.declarations[&other.class].namespace,
                "urn:intersect:inference:imported"
            );
        }
        if name == "new_cross_reference_to_parent" {
            let root = result
                .model
                .elements
                .iter()
                .find(|e| e.values.contains_key("uses"))
                .unwrap();
            assert_eq!(root.values["uses"], json!([selected.id, selected.id]));
        }
        if value.is_empty() {
            let enum_value: intersect_model::definitions::enums::EnumValue =
                serde_json::from_value(selected.values["shade"].clone()).unwrap();
            let enumeration = language
                .package()
                .schema
                .enumerations
                .values()
                .find(|e| e.name() == "Shade")
                .unwrap();
            assert_eq!(
                enum_value,
                enumeration
                    .value_by_id(&enumeration.by_name("RED").unwrap().declaration)
                    .unwrap()
            );
        }
        let replay: InferenceRequest =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        let replay = replay.compile(SourceLimits::default()).unwrap();
        assert_eq!(
            serde_json::to_value(language.package()).unwrap(),
            serde_json::to_value(replay.package()).unwrap()
        );
        assert_eq!(result.model, execute(&replay, text).model);
        let restored =
            serde_json::from_value(serde_json::to_value(language.package()).unwrap()).unwrap();
        let restored = intersect_text::recompile(&restored).unwrap();
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(language.package()).unwrap()
        );
        let restored_result = intersect_text::evaluate(
            &restored,
            &[intersect_text::Source {
                uri: result.model.elements[0].resource.clone(),
                text: text.into(),
            }],
        );
        assert!(
            restored_result.diagnostics.is_empty(),
            "{name}: {:?}",
            restored_result.diagnostics
        );
        assert_eq!(result.model, restored_result.model);

        // A child source revision changes its own definitions, while exact
        // generated parent identities remain stable and original inputs remain intact.
        let mut revised = request.clone();
        let old = revised.grammar.clone();
        let index = revised
            .bundle
            .sources
            .iter()
            .position(|s| s.id == old)
            .unwrap();
        let mut child = DefinitionSource::new(
            old.uri,
            SourceKind::Xtext,
            format!("{}\n// child revision", revised.bundle.sources[index].text),
        );
        child.dependencies = revised.bundle.sources[index].dependencies.clone();
        revised.grammar = child.id.clone();
        revised.bundle.sources[index] = child;
        let revised = revised.compile(SourceLimits::default()).unwrap();
        for (before, after) in language
            .definitions()
            .grammars
            .iter()
            .zip(&revised.definitions().grammars)
            .take(language.definitions().grammars.len() - 1)
        {
            assert_eq!(before.generated_sources, after.generated_sources);
        }
        let mut missing = request.clone();
        let child = missing
            .bundle
            .sources
            .iter_mut()
            .find(|s| s.id == missing.grammar)
            .unwrap();
        child
            .dependencies
            .retain(|id| request.bundle.source(id).unwrap().kind != SourceKind::Xtext);
        assert!(missing.compile(SourceLimits::default()).is_err());
        assert_eq!(result.model, execute(&language, text).model);
    }
    let case = cases
        .iter()
        .find(|c| c["id"] == "inherited_model_call")
        .unwrap();
    let request = input(case);
    let mixed = cases
        .iter()
        .find(|c| c["id"] == "parent_imports_external")
        .unwrap();
    let mut selected = input(mixed);
    let original = selected
        .bundle
        .sources
        .iter()
        .find(|s| s.kind == SourceKind::Rcore)
        .unwrap();
    let shadow = DefinitionSource::new(
        "memory:explicitly-selected-shadow.rcore",
        SourceKind::Rcore,
        original.text.clone(),
    );
    selected.bundle.namespaces[0].source = shadow.id.clone();
    selected.bundle.sources.push(shadow);
    // An explicit namespace binding outside the parent's declared dependencies
    // must reject, even when an unselected same-namespace source is reachable.
    assert!(selected.compile(SourceLimits::default()).is_err());
    let mut ambiguous = request.clone();
    let mut duplicate = ambiguous.bundle.sources[0].clone();
    duplicate.id = intersect_model::definitions::bundle::SourceId::for_text(
        "memory:ambiguous-parent.xtext",
        &duplicate.text,
    );
    ambiguous
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == ambiguous.grammar)
        .unwrap()
        .dependencies
        .push(duplicate.id.clone());
    ambiguous.bundle.sources.push(duplicate);
    assert!(ambiguous.compile(SourceLimits::default()).is_err());
    assert!(
        request
            .compile(SourceLimits {
                source_bytes: 256_000,
                declarations: 5
            })
            .is_err()
    );
    let mut larger = request.clone();
    let old = larger.bundle.sources[0].id.clone();
    let mut parent = DefinitionSource::new(
        old.uri.clone(),
        SourceKind::Xtext,
        format!(
            "{}\n/*{}*/",
            larger.bundle.sources[0].text,
            "x".repeat(260_000)
        ),
    );
    parent.dependencies = larger.bundle.sources[0].dependencies.clone();
    for source in &mut larger.bundle.sources {
        for dependency in &mut source.dependencies {
            if *dependency == old {
                *dependency = parent.id.clone();
            }
        }
    }
    larger.bundle.sources[0] = parent;
    assert!(larger.compile(SourceLimits::default()).is_err());
    let limits = SourceLimits {
        source_bytes: larger.bundle.sources[0].text.len() + 1024,
        declarations: 128,
    };
    let larger = larger.compile(limits).unwrap();
    assert_eq!(execute(&larger, "entry node Alpha").model.elements.len(), 2);
    assert!(larger.recompile(limits).is_ok());
    assert!(larger.recompile(SourceLimits::default()).is_err());
    let mut a = DefinitionSource::new(
        "memory:cycle-a.xtext",
        SourceKind::Xtext,
        "grammar cycle.A with cycle.B generate a \"urn:cycle:a\" Model: 'a';",
    );
    let mut b = DefinitionSource::new(
        "memory:cycle-b.xtext",
        SourceKind::Xtext,
        "grammar cycle.B with cycle.A generate b \"urn:cycle:b\" Model: 'b';",
    );
    a.dependencies.push(b.id.clone());
    b.dependencies.push(a.id.clone());
    let cycle = InferenceRequest {
        version: 1,
        grammar: a.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![a, b],
            namespaces: vec![],
        },
    };
    let errors = cycle.compile(SourceLimits::default()).err().unwrap();
    assert!(errors[0].message.contains("Cyclic"));
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
pub fn verify() -> usize {
    frozen_matrix();
    execution_and_replay();
    2
}
