use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::compiler;
use intersect_text::distribution::ordinary::{self as distribution, PackageMetadata};
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
const ECORE: &str = include_str!("../../../../fixtures/xtext/current-actions/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/current-actions/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/current-actions/model-behavior.rcore");
fn sources(text: &str) -> Vec<Source> {
    vec![Source {
        uri: "policy.kerml".into(),
        text: text.into(),
    }]
}
#[test]
fn admitted_large_sources_execute_and_replay_without_widening_model_limits() {
    // Independent enlarged teaching input; upstream and original fixtures are unchanged.
    let ecore = DefinitionSource::new(
        "memory:large.rcore",
        SourceKind::Rcore,
        format!(
            "{}/*{}*/",
            intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap(),
            "x".repeat(300_000)
        ),
    );
    let mut grammar = XTEXT.to_owned();
    for n in 0..275 {
        grammar.push_str(&format!("\nPadding{n} returns k::Literal: value=INT;"));
    }
    let mut xtext = DefinitionSource::new("memory:large.xtext", SourceKind::Xtext, grammar);
    let mut rcore_behavior = DefinitionSource::new(
        "memory:model-behavior.rcore",
        SourceKind::RcoreBehavior,
        RCORE_BEHAVIOR,
    );
    xtext.dependencies.push(ecore.id.clone());
    rcore_behavior.dependencies.push(ecore.id.clone());
    let policy = CompilationSourcePolicy {
        structure_bytes: ecore.text.len(),
        xtext_bytes: xtext.text.len(),
        xtext_declarations: 282,
    };
    let compilation = BundleCompilation {
        ecore: ecore.id.clone(),
        xtext: Some(xtext.id.clone()),
        rcore_behavior: rcore_behavior.id.clone(),
        namespace: "urn:intersect:xtext:current-actions:1".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore, xtext, rcore_behavior],
            namespaces: vec![],
        },
    };
    assert!(compiler::compile_bundle_with_profile(compilation.clone(), Profile::NativeV5).is_err());
    let package = compiler::compile_bundle_with_source_policy(
        compilation.clone(),
        Profile::NativeV5,
        policy,
        policy,
    )
    .unwrap();
    let grammar = package.grammar.as_ref().unwrap();
    assert_eq!(grammar.rules.len() + grammar.terminals.len(), 282);
    let baseline =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV5,
        )
        .unwrap();
    let model = sources("Demo = 1 + 2 - 3;");
    let expected = evaluate(&baseline, &model);
    let actual = evaluate(&package, &model);
    assert!(expected.diagnostics.is_empty());
    assert!(actual.diagnostics.is_empty(), "{:?}", actual.diagnostics);
    assert_eq!(actual.model.elements.len(), 6);
    // The new source has its own declaration identities; values and construction
    // must agree without making different source revisions interchangeable.
    let retained = &compilation.ecore;
    let mut expected_elements = expected.model.elements.clone();
    for element in &mut expected_elements {
        let index = match element.class.as_str() {
            "Model" => 0,
            "Literal" => 2,
            "OperatorExpression" => 3,
            other => panic!("Unexpected fixture class {other}"),
        };
        let identity = format!("{}@{}#{index}", retained.uri, retained.sha256);
        assert_ne!(element.type_identity, identity);
        element.type_identity = identity;
    }
    assert_eq!(actual.model.elements, expected_elements);
    for limit in [5, 6] {
        let result = evaluate_with_limits(
            &package,
            &model,
            EvaluationLimits {
                elements_per_resource: Some(limit),
                ..EvaluationLimits::native()
            },
        );
        if limit == 6 {
            assert!(result.diagnostics.is_empty());
            assert_eq!(result.model.elements.len(), 6);
        } else {
            assert!(!result.diagnostics.is_empty());
            assert!(result.model.elements.is_empty());
        }
    }
    let failed = evaluate(&package, &sources("Demo = 1 +;"));
    assert!(!failed.diagnostics.is_empty());
    assert!(failed.model.elements.is_empty());
    let replay = compiler::recompile_with_source_policy(&package, policy).unwrap();
    assert_eq!(
        serde_json::to_value(evaluate(&replay, &model).model.elements).unwrap(),
        serde_json::to_value(&actual.model.elements).unwrap()
    );
    let archive = distribution::build_bundle_with_source_policy(
        compilation,
        PackageMetadata {
            package_id: "example.large.current".into(),
            release: "1".into(),
            display_name: "Large source construction".into(),
            description: String::new(),
        },
        Profile::NativeV5,
        policy,
        policy,
    )
    .unwrap();
    let mut files = archive.files;
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&archive.manifest).unwrap(),
    );
    assert!(distribution::import_files(files.clone()).is_err());
    let loaded = distribution::import_files_with_source_policy(files, policy).unwrap();
    let after = evaluate(&loaded.package, &model);
    assert!(after.diagnostics.is_empty());
    assert_eq!(
        serde_json::to_value(after.model.elements).unwrap(),
        serde_json::to_value(actual.model.elements).unwrap()
    );
}
#[test]
fn enlarged_generic_sources_keep_the_same_limit_through_behavior_and_materialization() {
    use serde_json::{Value, json};
    fn replace_identity(value: &mut Value, old: &Value, new: &Value) {
        if value == old {
            *value = new.clone();
        } else {
            match value {
                Value::Array(values) => values
                    .iter_mut()
                    .for_each(|v| replace_identity(v, old, new)),
                Value::Object(values) => values
                    .values_mut()
                    .for_each(|v| replace_identity(v, old, new)),
                _ => {}
            }
        }
    }
    let mut wire: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/ecore-generics/language-bundle-canonical.json"
    ))
    .unwrap();
    for (index, suffix) in [
        (0, format!("/*{}*/", "x".repeat(260_000))),
        (
            2,
            "\nfn valueName(x:m.Value)->Option<String> = x.name;".to_owned(),
        ),
    ] {
        let old = wire["bundle"]["sources"][index]["id"].clone();
        let mut source: DefinitionSource =
            serde_json::from_value(wire["bundle"]["sources"][index].clone()).unwrap();
        let dependencies = source.dependencies.clone();
        source = DefinitionSource::new(
            &source.id.uri,
            source.kind,
            format!("{}{suffix}", source.text),
        );
        source.dependencies = dependencies;
        let new = json!(source.id);
        wire["bundle"]["sources"][index] = json!(source);
        replace_identity(&mut wire, &old, &new);
    }
    let input: BundleCompilation = serde_json::from_value(wire).unwrap();
    let policy = CompilationSourcePolicy {
        structure_bytes: input.bundle.sources[0].text.len(),
        ..Default::default()
    };
    assert!(compiler::compile_bundle(input.clone()).is_err());
    assert!(
        compiler::compile_bundle_with_source_policy(
            input.clone(),
            Profile::default(),
            policy,
            CompilationSourcePolicy::default()
        )
        .is_err()
    );
    let package =
        compiler::compile_bundle_with_source_policy(input, Profile::default(), policy, policy)
            .unwrap();
    let model = sources("root { value v; box v; }");
    let evaluated = evaluate(&package, &model);
    assert!(
        evaluated.diagnostics.is_empty(),
        "{:?}",
        evaluated.diagnostics
    );
    assert_eq!(evaluated.model.elements.len(), 3);
    assert!(compiler::recompile(&package).is_err());
    let replay = compiler::recompile_with_source_policy(&package, policy).unwrap();
    assert_eq!(evaluate(&replay, &model).model, evaluated.model);
}

#[test]
fn enlarged_enum_and_datatype_sources_use_the_admitted_limit_in_behavior() {
    let model = DefinitionSource::new(
        "memory:large-values.rcore",
        SourceKind::Rcore,
        format!(
            "rcore 1; package values version \"1\"; metamodel \"urn:large-values\" prefix values {{
            class Item {{ attr mode: Mode [0,1]; attr count: Count [0,1]; }}
            enum Mode {{ First = 0; Second = 1; }}
            datatype Count instanceClass \"int\" {{ }}
        }} /*{}*/",
            "x".repeat(260_000)
        ),
    );
    let mut behavior = DefinitionSource::new(
        "memory:large-values-behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package rules version \"1\"; import model \"urn:large-values\" as m;
         fn mode(x:m.Mode) -> m.Mode = x;
         fn count(x:m.Count) -> m.Count = x;",
    );
    behavior.dependencies.push(model.id.clone());
    let policy = CompilationSourcePolicy {
        structure_bytes: model.text.len(),
        ..Default::default()
    };
    let input = BundleCompilation {
        ecore: model.id.clone(),
        xtext: None,
        rcore_behavior: behavior.id.clone(),
        namespace: "urn:large-values".into(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, behavior],
            namespaces: vec![],
        },
    };
    assert!(compiler::compile_bundle(input.clone()).is_err());
    let package =
        compiler::compile_bundle_with_source_policy(input, Profile::default(), policy, policy)
            .unwrap();
    assert!(compiler::recompile(&package).is_err());
    compiler::recompile_with_source_policy(&package, policy).unwrap();
}
