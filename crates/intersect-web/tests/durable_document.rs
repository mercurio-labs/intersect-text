//! Public production dispatcher consumer; no alternative parser or recovered-model publication.
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
fn compilation() -> Value {
    let cases: Value = serde_json::from_str(include_str!(
        "../../intersect-text/src/document_tests/cases.json"
    ))
    .unwrap();
    cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "reference_adds_scope")
        .unwrap()["compilation"]
        .clone()
}
pub fn actual_public_batch_analysis_quarantines_invalid_edits_and_preserves_legacy_profile_boundary()
 {
    let inputs = compilation();
    let built = call(
        json!({"method":"compile_bundle","xtext_profile":"intersect.xtext.native.v65","compilation":inputs}),
    );
    assert_eq!(built["ok"], true);
    let sources = json!([{"uri":"memory:public-document/a","text":"type:a;use:f=b;"},{"uri":"memory:public-document/b","text":"type:b;use:g=a;"}]);
    let valid =
        call(json!({"method":"document/analyze","package":built["package"],"sources":sources}));
    assert!(valid["valid_model"].is_object());
    assert!(valid["recovered"].is_null());
    assert_eq!(valid["snapshots"].as_array().unwrap().len(), 2);
    assert_eq!(
        valid["valid_model"],
        call(json!({"method":"evaluate","package":built["package"],"sources":sources}))
    );
    let mut edited = sources.clone();
    edited[1]["text"] = json!("type:b;use:g=;");
    let invalid =
        call(json!({"method":"document/analyze","package":built["package"],"sources":edited}));
    assert!(invalid["valid_model"].is_null());
    assert!(invalid["recovered"]["objects"].is_array());
    assert!(invalid["recovered"]["elements"].is_null());
    assert!(
        invalid["snapshots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| !s["syntax_issues"].as_array().unwrap().is_empty())
    );
    let old = call(
        json!({"method":"compile_bundle","xtext_profile":"intersect.xtext.native.v64","compilation":inputs}),
    );
    assert_eq!(old["ok"], true);
    let error = intersect_web::dispatch(
        &json!({"method":"document/analyze","package":old["package"],"sources":sources})
            .to_string(),
    )
    .unwrap_err();
    assert!(error.contains("NativeV65"));
    let mut forged = built["package"].clone();
    forged["grammar"]["rules"]["Model"]["body"]["original_range"] = json!({"start":0,"end":0});
    match intersect_web::dispatch(
        &json!({"method":"document/analyze","package":forged,"sources":sources}).to_string(),
    ) {
        Err(_) => {}
        Ok(value) => assert_eq!(
            serde_json::from_str::<Value>(&value.to_string()).unwrap(),
            valid
        ),
    };
}

pub fn source_and_rehashed_archive_caches_cannot_authorize_recovery_or_node_origins() {
    use intersect_model::definitions::bundle::SourceId;
    let compilation = compilation();
    let exported = call(
        json!({"method":"package/export","compilation":compilation,"xtext_profile":"intersect.xtext.native.v65","metadata":{"package_id":"document.authority","release":"1","display_name":"Document authority","description":"Original source authority controls"}}),
    );
    let package = exported["package"].clone();
    let mut files = exported["files"].as_object().unwrap().clone();
    files.insert(
        "manifest.json".into(),
        json!(exported["manifest"].to_string()),
    );
    assert_eq!(
        call(json!({"method":"package/import","files":files}))["package"],
        package
    );
    let sources = json!([{"uri":"memory:document-archive/input","text":"type:a;use:f=;"}]);
    let expected = call(json!({"method":"document/analyze","package":package,"sources":sources}));
    assert!(expected["valid_model"].is_null());
    assert!(expected["recovered"].is_object());
    for field in ["origin", "recovery", "declaration"] {
        let mut forged = files.clone();
        let mut cached: Value =
            serde_json::from_str(forged["package.json"].as_str().unwrap()).unwrap();
        match field {
            "origin" => {
                cached["grammar"]["rules"]["Model"]["body"]["original_range"] =
                    json!({"start":0,"end":0})
            }
            "recovery" => {
                cached["grammar"]["parser_prediction"]["decisions"]["@document-recovery"]["entries"] =
                    json!([0])
            }
            _ => {
                cached["grammar"]["declaration_sources"]["Model"]["source"]["sha256"] =
                    json!("0".repeat(64))
            }
        };
        assert_ne!(cached, package);
        assert_eq!(cached["compilation"], compilation);
        assert_eq!(
            call(json!({"method":"document/analyze","package":cached,"sources":sources})),
            expected,
            "source replay must ignore forged observation/recovery caches"
        );
        let text = cached.to_string();
        let mut manifest: Value =
            serde_json::from_str(forged["manifest.json"].as_str().unwrap()).unwrap();
        manifest["files"]["package.json"] =
            json!({"bytes":text.len(),"sha256":SourceId::for_text("ignored",&text).sha256});
        forged.insert("package.json".into(), json!(text));
        forged.insert("manifest.json".into(), json!(manifest.to_string()));
        let error =
            intersect_web::dispatch(&json!({"method":"package/import","files":forged}).to_string())
                .unwrap_err();
        assert!(
            error.contains("Archived compiled package differs from freshly compiled definitions"),
            "{field}: {error}"
        );
    }
}
pub fn actual_typed_links_and_utf16_error_carriers_share_the_public_engine() {
    use intersect_text::package::Package;
    use intersect_text::{
        EvaluationLimits, Source,
        document::{AnalysisLimits, DocumentLanguage},
    };
    let built = call(
        json!({"method":"compile_bundle","xtext_profile":"intersect.xtext.native.v65","compilation":compilation()}),
    );
    let package: Package = serde_json::from_value(built["package"].clone()).unwrap();
    let language =
        DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
            .unwrap();
    let source = Source {
        uri: "memory:public-typed-document/input".into(),
        text: "type:a;use:f=a;".into(),
    };
    let valid = language
        .analyze(&source, None, EvaluationLimits::workbench())
        .unwrap();
    let model = &valid.valid_model.as_ref().unwrap().model;
    let snapshot = &valid.snapshot;
    let node = snapshot
        .document()
        .handle(intersect_model::core::text_document::NodeId(0))
        .unwrap();
    assert_eq!(
        snapshot
            .linked_object(&node, model, &package.schema)
            .unwrap()
            .unwrap()
            .resource,
        source.uri
    );
    assert_eq!(
        snapshot
            .parse_stage()
            .references
            .iter()
            .filter(|r| r.active)
            .count(),
        1
    );
    for feature in snapshot.feature_nodes() {
        let owner = &snapshot.semantic_objects()[feature.owner_object];
        assert_eq!(
            package
                .schema
                .feature_by_id(&owner.class, &feature.identity)
                .unwrap()
                .feature
                .name,
            feature.feature
        );
    }
    let second = language
        .analyze(&source, None, EvaluationLimits::workbench())
        .unwrap();
    let foreign = second.snapshot.document().handle(node.node).unwrap();
    assert!(
        snapshot
            .linked_object(&foreign, model, &package.schema)
            .is_err()
    );
    let mut stale = model.clone();
    stale
        .resources
        .get_mut(&source.uri)
        .unwrap()
        .content_revision += 1;
    assert!(
        snapshot
            .linked_object(&node, &stale, &package.schema)
            .is_err()
    );
    let invalid = Source {
        uri: source.uri,
        text: "type:a;use:f=😀;".into(),
    };
    let analysis = language
        .analyze(&invalid, None, EvaluationLimits::workbench())
        .unwrap();
    assert!(analysis.valid_model.is_none());
    assert!(analysis.recovered.is_some());
    assert_eq!(
        analysis
            .snapshot
            .document()
            .units_in(intersect_model::core::text_document::TextRange {
                start: 0,
                end: invalid.text.encode_utf16().count()
            })
            .unwrap()
            .collect::<Vec<_>>(),
        invalid.text.encode_utf16().collect::<Vec<_>>()
    );
    assert!(
        analysis
            .snapshot
            .syntax_issues()
            .iter()
            .any(|i| i.message_utf16.is_some())
    );
    let diagnostics = analysis
        .snapshot
        .syntax_diagnostics()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(!diagnostics.is_empty());
    assert!(
        diagnostics
            .iter()
            .all(|d| d.source == analysis.snapshot.document().source())
    );
}
#[path = "durable_rgb.rs"]
mod durable_rgb;

#[path = "support/ordinary_catalog_archive.rs"]
mod ordinary_catalog_archive;

pub fn verify_all() {
    let legacy: Value =
        serde_json::from_str(include_str!("fixtures/legacy_document_tail.json")).unwrap();
    assert_eq!(
        call(legacy["request"].clone()),
        legacy["result"],
        "exact original V63 trailing-input rejection and whole model"
    );
    ordinary_catalog_archive::verify_ordinary_catalog_archives();
    durable_rgb::verify_typed_rgb_documents();
    source_rebuilt_node_links_check_current_context_and_never_import_observed_authority();
    document_analysis_explicit_bundle_uses_current_nominal_provider_context();
    actual_public_batch_analysis_quarantines_invalid_edits_and_preserves_legacy_profile_boundary();
    source_and_rehashed_archive_caches_cannot_authorize_recovery_or_node_origins();
    actual_typed_links_and_utf16_error_carriers_share_the_public_engine();
}
#[cfg(test)]
mod controls {
    #[test]
    fn batch_publication() {
        super::actual_public_batch_analysis_quarantines_invalid_edits_and_preserves_legacy_profile_boundary();
    }
    #[test]
    fn source_archive_authority() {
        super::source_and_rehashed_archive_caches_cannot_authorize_recovery_or_node_origins();
    }
    #[test]
    fn typed_links_and_utf16() {
        super::actual_typed_links_and_utf16_error_carriers_share_the_public_engine();
    }
    #[test]
    fn shared_group_aggregate() {
        super::verify_all();
    }
}

pub fn document_analysis_explicit_bundle_uses_current_nominal_provider_context() {
    use intersect_model::definitions::compiler::standard_datatype;
    let datatype = standard_datatype("EJavaObject").unwrap();
    let request = json!({"method":"compile_rcore","xtext_profile":"intersect.xtext.native.v65","rcore":r##"rcore 1; package paint version "1"; metamodel "urn:paint" prefix p { class Paint { attr color: external "http://www.eclipse.org/emf/2002/Ecore#//EJavaObject" [0,1] default "#123abc"; } }"##,"rcore_behavior":"rcore 1; package emptyRules version \"1\";","xtext":r#"grammar document.provider.Paint hidden(WS)
import "urn:paint" as p
import "http://www.eclipse.org/emf/2002/Ecore" as ecore
Paint returns p::Paint: 'paint' color=STRING?;
terminal STRING returns ecore::EString: '"' !'"'* '"';
terminal WS returns ecore::EString: ' '+;
"#,"conversions":[{"datatype":datatype.declaration(),"provider":{"name":"example.rgb","version":"1"},"serializable":true}]});
    let original = request;
    let rejected = call(original.clone());
    assert_eq!(rejected["ok"], false);
    assert!(
        rejected["diagnostics"]
            .to_string()
            .contains("fresh language-owned authored-source validation")
    );
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    let model = DefinitionSource::new(
        "memory:model.rcore",
        SourceKind::Rcore,
        original["rcore"].as_str().unwrap(),
    );
    let mut grammar = DefinitionSource::new(
        "memory:document-provider-v2/grammar.xtext",
        SourceKind::Xtext,
        original["xtext"].as_str().unwrap(),
    );
    grammar.dependencies.push(model.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:document-provider-v2/empty.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package emptyRules version "1";"#,
    );
    behavior.dependencies.push(model.id.clone());
    let inputs = BundleCompilation {
        ecore: model.id.clone(),
        namespace: "urn:paint".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: behavior.id.clone(),
        materialized_types: Vec::new(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, grammar, behavior],
            namespaces: Vec::new(),
        },
    };
    let mut inherited = inputs.clone();
    let mut base = DefinitionSource::new(
        "memory:document-provider-v3/base.xtext",
        SourceKind::Xtext,
        r#"grammar document.provider.Base
import "urn:paint" as p
import "http://www.eclipse.org/emf/2002/Ecore" as ecore
terminal STRING returns ecore::EString: '"' !'"'* '"';
terminal WS returns ecore::EString: ' '+;
"#,
    );
    base.dependencies.push(inherited.ecore.clone());
    let mut child = DefinitionSource::new(
        "memory:document-provider-v3/child.xtext",
        SourceKind::Xtext,
        r#"grammar document.provider.Child with document.provider.Base hidden(WS)
import "urn:paint" as p
Paint returns p::Paint: 'paint' color=STRING?;
"#,
    );
    child
        .dependencies
        .extend([base.id.clone(), inherited.ecore.clone()]);
    inherited.xtext = Some(child.id.clone());
    inherited
        .bundle
        .sources
        .retain(|source| source.kind != SourceKind::Xtext);
    inherited.bundle.sources.extend([base, child]);
    let mut parameterized = inputs.clone();
    let mut parameter_source = DefinitionSource::new(
        "memory:document-provider-v4/parameter-default-companion.xtext",
        SourceKind::Xtext,
        r#"grammar document.provider.ParameterDefault hidden(WS)
import "urn:paint" as p
import "http://www.eclipse.org/emf/2002/Ecore" as ecore
Entry returns p::Paint: Paint<true>;
Paint<Flag> returns p::Paint: 'paint';
terminal WS returns ecore::EString: ' '+;
"#,
    );
    parameter_source
        .dependencies
        .push(parameterized.ecore.clone());
    parameterized.xtext = Some(parameter_source.id.clone());
    parameterized
        .bundle
        .sources
        .retain(|source| source.kind != SourceKind::Xtext);
    parameterized.bundle.sources.push(parameter_source);
    for (context, (inputs, reject_string_shape)) in
        [(inputs, true), (inherited, true), (parameterized, false)]
            .into_iter()
            .enumerate()
    {
        if reject_string_shape {
            let rejected_shape = call(
                json!({"method":"compile_bundle","compilation":inputs,"conversions":original["conversions"],"xtext_profile":"intersect.xtext.native.v65"}),
            );
            assert_eq!(rejected_shape["ok"], false);
            assert!(
                rejected_shape["diagnostics"]
                    .to_string()
                    .contains("Grammar assignment conflicts with the imported feature")
            );
        }
        // Separate default-only authored source; the rejected lexical graph above
        // stays checked and does not acquire admission through this companion.
        let mut positive = inputs;
        let old_entry = positive.xtext.as_ref().unwrap().clone();
        let old_source = positive.bundle.source(&old_entry).unwrap();
        let mut companion = DefinitionSource::new(
            format!("{}/default-action-companion", old_entry.uri),
            SourceKind::Xtext,
            old_source
                .text
                .replace("color=STRING?", "")
                .replace(": 'paint'", ": {p::Paint} 'paint'"),
        );
        companion.dependencies = old_source.dependencies.clone();
        positive.xtext = Some(companion.id.clone());
        positive
            .bundle
            .sources
            .retain(|source| source.id != old_entry);
        positive.bundle.sources.push(companion);
        let request = json!({"method":"compile_bundle","compilation":positive,"conversions":original["conversions"],"xtext_profile":"intersect.xtext.native.v65"});
        let built = call(request);
        assert_eq!(built["ok"], true, "{built}");
        for text in ["paint"] {
            let sources = json!([{"uri":"memory:document-provider/input","text":text}]);
            let request =
                json!({"method":"document/analyze","package":built["package"],"sources":sources});
            let analysis = intersect_web::dispatch(&request.to_string())
                .unwrap_or_else(|error| panic!("{error}; provider context {context}"));
            assert!(analysis["valid_model"].is_object(), "{analysis}");
            let color = &analysis["valid_model"]["model"]["elements"][0]["values"]["color"];
            assert_eq!(
                color["datatype"],
                json!(datatype.declaration()),
                "{analysis}"
            );
            assert_eq!(
                color["provider"],
                json!({"name":"example.rgb","version":"1"})
            );
            assert_eq!(
                color["value"],
                if text == "paint" {
                    json!({"red":18,"green":58,"blue":188})
                } else {
                    json!({"red":1,"green":2,"blue":3})
                }
            );
            for changed in ["unknown.provider", "stale-version"] {
                let mut forged = request.clone();
                if changed == "stale-version" {
                    forged["package"]["schema"]["conversion_contracts"][0]["provider"]["version"] =
                        json!("2");
                } else {
                    forged["package"]["schema"]["conversion_contracts"][0]["provider"]["name"] =
                        json!(changed);
                }
                assert!(intersect_web::dispatch(&forged.to_string()).is_err());
            }
            let mut missing = request;
            missing["package"]["schema"]["conversion_contracts"] = json!([]);
            assert!(intersect_web::dispatch(&missing.to_string()).is_err());
        }
    }
}
#[cfg(test)]
mod provider_control {
    #[test]
    fn current_nominal_provider_context() {
        super::document_analysis_explicit_bundle_uses_current_nominal_provider_context();
    }
}

pub fn source_rebuilt_node_links_check_current_context_and_never_import_observed_authority() {
    let built = call(
        json!({"method":"compile_bundle","compilation":compilation(),"xtext_profile":"intersect.xtext.native.v65"}),
    );
    assert_eq!(built["ok"], true, "{built}");
    let sources = json!([{"uri":"memory:source-rebuilt-node/input","text":"type:a;use:f=a;"}]);
    let analysis =
        call(json!({"method":"document/analyze","package":built["package"],"sources":sources}));
    assert!(analysis["valid_model"].is_object(), "{analysis}");
    let snapshot = &analysis["snapshots"][0];
    let request = json!({"method":"document/link","package":built["package"],"sources":sources,"source":snapshot["document"]["source"],"schema_revision":snapshot["schema_revision"],"resource_revision":snapshot["document"]["resource_revision"],"node":0});
    let linked = call(request.clone());
    assert_eq!(linked["object"]["resource"], sources[0]["uri"]);
    assert!(
        analysis["valid_model"]["model"]["elements"]
            .as_array()
            .unwrap()
            .contains(&linked["object"])
    );
    for field in ["schema_revision", "resource_revision", "source", "node"] {
        let mut stale = request.clone();
        match field {
            "source" => stale["source"]["sha256"] = json!("0".repeat(64)),
            "node" => stale["node"] = json!(u64::MAX),
            "resource_revision" => stale[field] = json!("999"),
            _ => stale[field] = json!("foreign-schema"),
        }
        assert!(
            intersect_web::dispatch(&stale.to_string()).is_err(),
            "{field}"
        );
    }
    let mut changed = request.clone();
    changed["sources"][0]["text"] = json!("type:b;use:f=b;");
    assert!(intersect_web::dispatch(&changed.to_string()).is_err());
    let mut invalid = request.clone();
    invalid["sources"][0]["text"] = json!("type:a;use:f=missing;");
    assert!(intersect_web::dispatch(&invalid.to_string()).is_err());
    let mut forged = request;
    forged["snapshot"] = snapshot.clone();
    forged["snapshot"]["semantic_objects"][0]["id"] = json!("forged-owner");
    forged["model"] = json!({"elements":[{"id":"forged-owner"}]});
    assert_eq!(call(forged), linked);
}

#[cfg(test)]
mod source_rebuilt_link_control {
    #[test]
    fn current_source_context() {
        super::source_rebuilt_node_links_check_current_context_and_never_import_observed_authority(
        );
    }
}
