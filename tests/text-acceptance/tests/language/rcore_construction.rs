use intersect_model::core::model::Model;
use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    rcore::resolve_structure,
};
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{Source, evaluate};
use serde_json::json;

const STRUCTURE: &str = include_str!("../../../../fixtures/rcore/web/specialization.rcore");
const GRAMMAR: &str = include_str!("../../../../fixtures/rcore/web/specialization.xtext");
fn package(grammar: &str) -> Package {
    intersect_text::compiler::compile_rcore_language(
        STRUCTURE,
        grammar,
        r#"rcore 1; package emptyRules version "1";"#,
    )
    .unwrap()
}

fn sources(relations: &str) -> Vec<Source> {
    vec![
        Source {
            uri: "types.demo".into(),
            text: include_str!("../../../../fixtures/rcore/web/types.demo").into(),
        },
        Source {
            uri: "relations.demo".into(),
            text: relations.into(),
        },
    ]
}
const RELATIONS: &str = include_str!("../../../../fixtures/rcore/web/relations.demo");

// Expected rows are authored independently; no parser output feeds direct construction.
fn direct(p: &Package, target: &str) -> Model {
    let mut model = Model::default();
    let mut edits = Vec::new();
    for (id, class, name) in [
        ("types.demo#0", "Model", "Types"),
        ("types.demo#1", "Type", "A"),
        ("types.demo#2", "Type", "B"),
        ("types.demo#3", "Type", "C"),
        ("types.demo#4", "Type", "D"),
        ("relations.demo#0", "Model", "Relations"),
        ("relations.demo#1", "Specialization", "AB"),
        ("relations.demo#2", "Specialization", "BC"),
    ] {
        model
            .create(
                &p.schema,
                id,
                p.schema.language_class_key(class).unwrap(),
                id.split('#').next().unwrap(),
                0,
                0,
            )
            .unwrap();
        edits.push((id.into(), "name".into(), json!(name)));
    }
    for (id, field, value) in [
        (
            "types.demo#0",
            "types",
            json!([
                "types.demo#1",
                "types.demo#2",
                "types.demo#3",
                "types.demo#4"
            ]),
        ),
        (
            "relations.demo#0",
            "specializations",
            json!(["relations.demo#1", "relations.demo#2"]),
        ),
        ("relations.demo#1", "specific", json!("types.demo#1")),
        ("relations.demo#1", "general", json!("types.demo#2")),
        ("relations.demo#2", "specific", json!("types.demo#2")),
        ("relations.demo#2", "general", json!(target)),
    ] {
        edits.push((id.into(), field.into(), value));
    }
    model.edit(&p.schema, &edits).unwrap();
    model.validate(&p.schema).unwrap();
    model
}
#[test]
fn xtext_constructs_same_named_containment_and_cross_resource_links() {
    let p = package(GRAMMAR);
    for (text, target) in [
        (RELATIONS.to_string(), "types.demo#3"),
        (RELATIONS.replace("BC B to C", "BC B to D"), "types.demo#4"),
    ] {
        let parsed = evaluate(&p, &sources(&text));
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        parsed.model.validate(&p.schema).unwrap();
        let direct = direct(&p, target);
        assert_eq!(parsed.model.schema_revision, direct.schema_revision);
        assert_eq!(parsed.model.elements.len(), direct.elements.len());
        for (a, b) in parsed.model.elements.iter().zip(&direct.elements) {
            assert_eq!(
                (&a.id, &a.class, &a.type_identity, &a.resource, &a.values),
                (&b.id, &b.class, &b.type_identity, &b.resource, &b.values)
            );
            // Real text positions remain distinct from source-less direct construction.
            assert!(a.end > a.start);
            assert_eq!((b.start, b.end), (0, 0));
        }
        assert_eq!(parsed.references.len(), 4);
        assert!(parsed.references.iter().all(|r| r.uri == "relations.demo"
            && r.end > r.start
            && r.target.starts_with("types.demo#")));
    }
}
#[test]
fn edits_and_invalid_links_are_explicit() {
    let p = package(GRAMMAR);
    for invalid in [
        RELATIONS.replace("to C", "to Missing"),
        RELATIONS.replace("to C", "to Relations"),
        RELATIONS.replace("to C;", "to ;"),
    ] {
        assert!(!evaluate(&p, &sources(&invalid)).diagnostics.is_empty());
    }
    let mut ambiguous = sources(RELATIONS);
    ambiguous[0].text = ambiguous[0].text.replace("type D;", "type B;");
    assert!(!evaluate(&p, &ambiguous).diagnostics.is_empty());
    let changed = package(&GRAMMAR.replace("'type' name", "'classifier' name"));
    assert!(
        !evaluate(&changed, &sources(RELATIONS))
            .diagnostics
            .is_empty()
    );
    let mut edited = sources(RELATIONS);
    edited[0].text = edited[0].text.replace("type ", "classifier ");
    assert!(evaluate(&changed, &edited).diagnostics.is_empty());
}

#[test]
fn exact_source_bundle_runs_operation_on_text_and_direct_models() {
    use intersect_model::definitions::{
        bundle::DefinitionBundle,
        conversion::{ConversionRegistry, ReferenceContext},
        generic_types::{RuntimeType, Type},
        rcore_behavior::{
            ir_v2::{DataWire, EntrySelection, Limits, ValueWire},
            modules::SymbolId,
        },
        rcore_operations::{OperationSelection, compile_operation},
    };
    use intersect_text::compiler::compile_bundle;

    use intersect_model::{
        model_reference::ReferenceDomain,
        rcore_behavior_functions::{ExecutionContext, ExecutionLimits, ObservationLimits},
        rcore_operations::invoke,
    };
    let model = DefinitionSource::new("memory:structure.rcore", SourceKind::Rcore, STRUCTURE);
    let structural_bundle = DefinitionBundle {
        version: 1,
        sources: vec![model.clone()],
        namespaces: vec![],
    };
    let (classifiers, operations) = resolve_structure(&structural_bundle, &model.id).unwrap();
    let mut grammar =
        DefinitionSource::new("memory:specialization.xtext", SourceKind::Xtext, GRAMMAR);
    grammar.dependencies.push(model.id.clone());
    let mut semantic = DefinitionSource::new(
        "memory:scoping-behavior.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    semantic.dependencies.push(model.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:behavior.rcore",
        SourceKind::RcoreBehavior,
        include_str!("../../../../fixtures/rcore/web/specialization-behavior.rcore"),
    );
    behavior.dependencies.push(model.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![
            model.clone(),
            grammar.clone(),
            semantic.clone(),
            behavior.clone(),
        ],
        namespaces: vec![],
    };
    let types: Vec<_> = classifiers
        .iter()
        .map(|c| {
            RuntimeType::new(Type::Classifier {
                declaration: c.declaration.clone(),
                arguments: vec![],
                raw: false,
            })
            .unwrap()
        })
        .collect();
    let p = compile_bundle(BundleCompilation {
        materialized_types: types.clone(),
        bundle: bundle.clone(),
        ecore: model.id.clone(),
        namespace: "urn:intersect:rcore:web-specialization".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: semantic.id.clone(),
    })
    .unwrap();
    assert_eq!(p.compilation.as_ref().unwrap().bundle, bundle);
    let receiver = RuntimeType::new(Type::Classifier {
        declaration: classifiers[1].declaration.clone(),
        arguments: vec![],
        raw: false,
    })
    .unwrap();
    let operation = compile_operation(
        &bundle,
        &behavior.id,
        &OperationSelection {
            receiver: receiver.clone(),
            declaration: operations[0].declaration.clone(),
            materialized_types: types.clone(),
            implementation: EntrySelection {
                symbol: SymbolId {
                    source: behavior.id.clone(),
                    declaration: 1,
                },
                types: vec![],
                arguments: vec![],
            },
        },
        &Limits::default(),
    )
    .unwrap();
    for (text, target, expected) in [
        (
            RELATIONS.to_string(),
            "types.demo#3",
            vec!["types.demo#1", "types.demo#2", "types.demo#3"],
        ),
        (
            RELATIONS.replace("BC B to C", "BC B to D"),
            "types.demo#4",
            vec!["types.demo#1", "types.demo#2", "types.demo#4"],
        ),
    ] {
        let parsed = evaluate(&p, &sources(&text));
        assert!(parsed.diagnostics.is_empty());
        let mut reports = vec![];
        for instance in [parsed.model, direct(&p, target)] {
            let reference = ReferenceContext {
                bundle: DefinitionBundle {
                    version: 1,
                    sources: vec![model.clone()],
                    namespaces: vec![],
                },
                entry: model.id.clone(),
                namespace: "urn:intersect:rcore:web-specialization".into(),
                conversions: vec![],
                materialized_types: types.clone(),
                applied_conversions: vec![],
                constructors: vec![],
                model: serde_json::to_value(instance).unwrap(),
            };
            let payload = ReferenceDomain::new_runtime(
                receiver.clone(),
                reference.clone(),
                &ConversionRegistry::default(),
            )
            .unwrap()
            .bind("types.demo#1")
            .unwrap();
            let program = operation.executable().program();
            let argument = ValueWire {
                ty: program.functions[program.entry.0 as usize].parameters[0],
                data: DataWire::Model(payload),
            };
            let references = [reference];
            let result = invoke(
                &operation,
                &[argument],
                &ExecutionContext {
                    bundle: &bundle,
                    conversions: None,
                    references: &references,
                },
                &ExecutionLimits {
                    observation: ObservationLimits {
                        enabled: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            let DataWire::Collection(values) = result.value.data else {
                panic!("operation did not return a collection")
            };
            let ids: Vec<_> = values
                .iter()
                .map(|v| {
                    let DataWire::Model(m) = &v.data else {
                        panic!("non-model result")
                    };
                    m["element"].as_str().unwrap()
                })
                .collect();
            assert_eq!(ids, expected);
            let report = result.observation.unwrap();
            assert!(report.execution_complete && report.events_complete);
            assert!(!report.cache_dependencies_complete);
            assert_eq!(report.validation_admissions.len(), 1);
            // Context hashes differ because real text positions are preserved.
            reports.push(
                report
                    .semantic_reads
                    .iter()
                    .map(|r| {
                        (
                            serde_json::to_value(r.kind).unwrap(),
                            r.element.clone(),
                            r.member.clone(),
                            r.source.clone(),
                            r.cursor,
                            r.matched.clone(),
                            r.value.clone(),
                        )
                    })
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(reports[0], reports[1]);
    }
}
