//! Ordinary imported-source controls against the frozen independent observations.
use intersect_model::definitions::bundle::{
    DefinitionBundle, DefinitionSource, NamespaceBinding, SourceId, SourceKind,
};
use intersect_text::package::BundleCompilation;
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) fn input(case: &Value) -> BundleCompilation {
    let models = case["metamodels"].as_array().unwrap();
    let mut sources = models
        .iter()
        .map(|model| {
            let source = DefinitionSource::new(
                model["uri"].as_str().unwrap(),
                SourceKind::Ecore,
                model["source"].as_str().unwrap(),
            );
            assert_eq!(source.id.sha256, model["sha256"].as_str().unwrap());
            source
        })
        .collect::<Vec<_>>();
    let model_ids = sources
        .iter()
        .map(|source| source.id.clone())
        .collect::<Vec<_>>();
    // The registered oracle context contains every pinned provider. Retain that
    // same explicit closed structure context even when two imported packages
    // have no feature-reference edge between their classifiers.
    for (index, source) in sources.iter_mut().enumerate() {
        source.dependencies.extend(
            model_ids
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, id)| id.clone()),
        );
    }
    let namespaces = models
        .iter()
        .zip(&model_ids)
        .map(|(model, id)| NamespaceBinding {
            namespace: model["namespace"].as_str().unwrap().into(),
            source: id.clone(),
        })
        .collect();
    let mut ids = BTreeMap::<String, SourceId>::new();
    for grammar in case["sources"].as_array().unwrap() {
        let mut source = DefinitionSource::new(
            grammar["uri"].as_str().unwrap(),
            SourceKind::Xtext,
            grammar["source"].as_str().unwrap(),
        );
        assert_eq!(source.id.sha256, grammar["sha256"].as_str().unwrap());
        source.dependencies = model_ids.clone();
        for dependency in grammar["dependencies"].as_array().unwrap() {
            source
                .dependencies
                .push(ids[dependency.as_str().unwrap()].clone());
        }
        ids.insert(grammar["id"].as_str().unwrap().into(), source.id.clone());
        sources.push(source);
    }
    let mut behavior = DefinitionSource::new(
        format!(
            "memory:/ordinary-parameters/{}/behavior.rcore",
            case["id"].as_str().unwrap()
        ),
        SourceKind::RcoreBehavior,
        "rcore 1; package ordinary version \"1\";",
    );
    behavior.dependencies = model_ids.clone();
    let original = BundleCompilation {
        ecore: model_ids[0].clone(),
        namespace: models[0]["namespace"].as_str().unwrap().into(),
        xtext: Some(ids[case["entry"].as_str().unwrap()].clone()),
        rcore_behavior: behavior.id.clone(),
        materialized_types: vec![],
        bundle: DefinitionBundle {
            version: 1,
            sources: {
                sources.push(behavior);
                sources
            },
            namespaces,
        },
    };
    let targets = model_ids
        .iter()
        .map(|id| (id.clone(), id.uri.replace(".ecore", ".rcore")))
        .collect();
    let report =
        intersect_model::migration::migrate_ecore_bundle(original.bundle.clone(), &targets)
            .unwrap();
    intersect_text::migration::remap_compilation(&report, &original)
        .unwrap_or_else(|error| panic!("{}: {error}: {:?}", case["id"], report.reports))
}
