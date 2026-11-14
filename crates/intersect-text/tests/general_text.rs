//! Independent languages using grammar-derived defaults, not Library adapters.
use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
use intersect_text::package::BundleCompilation;
use intersect_text::setting::TextSettings;
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
    grammar::Profile,
    ordinary,
    serialization::{CanonicalSerializer, SerializationLimits},
    source_edit::SourceEditSession,
};
use serde_json::json;

fn compilation(structure: &str, grammar: &str) -> BundleCompilation {
    let model = DefinitionSource::new("memory:general.rcore", SourceKind::Rcore, structure);
    let mut text = DefinitionSource::new("memory:general.xtext", SourceKind::Xtext, grammar);
    let mut common = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        include_str!(
            "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
        ),
    );
    common.dependencies.push(model.id.clone());
    text.dependencies = vec![model.id.clone(), common.id.clone()];
    BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:general-text".into(),
        xtext: Some(text.id.clone()),
        rcore_behavior: model.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, text, common],
            namespaces: vec![],
        },
    }
}
fn language(structure: &str, grammar: &str) -> DocumentLanguage {
    language_with_profile(structure, grammar, Profile::NativeV80)
}
fn language_with_profile(structure: &str, grammar: &str, profile: Profile) -> DocumentLanguage {
    let package = ordinary::compile(compilation(structure, grammar), profile).unwrap();
    DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench()).unwrap()
}
const MACHINE: &str = include_str!("../../../fixtures/xtext/model-to-text/machine.rcore");
const MACHINE_TEXT: &str = include_str!("../../../fixtures/xtext/model-to-text/machine.xtext");
fn machine() -> SourceEditSession {
    SourceEditSession::new(language(MACHINE,MACHINE_TEXT), &[Source{uri:"machine.test".into(), text:"// header 😀\r\nmachine M {\r\n  state start label 'Start'; // retained\r\n  edge next start -> end;\r\n  state end final;\r\n}\r\n".into()}], EvaluationLimits::workbench()).unwrap()
}
fn named(s: &SourceEditSession, name: &str) -> String {
    s.model()
        .elements
        .iter()
        .find(|e| e.values.get("name") == Some(&json!(name)))
        .unwrap()
        .id
        .clone()
}

use intersect_model::core::{
    query::QueryBudget,
    storage::{ModelRead, Query},
};
use intersect_text::linking::{ReferenceCandidates, ReferenceRequest, UniqueNames};
struct Aliases;
impl ReferenceCandidates for Aliases {
    fn validate_definition(
        &self,
        package: intersect_text::package::TextPackageView<'_>,
    ) -> Result<(), String> {
        UniqueNames.validate_definition(package)
    }
    fn candidates(
        &self,
        store: &dyn ModelRead,
        reference: &ReferenceRequest<'_>,
        work: &mut QueryBudget,
    ) -> Result<Vec<String>, String> {
        work.charge()?;
        store.query(Query::Value(
            "name",
            &json!(
                reference
                    .spelling
                    .strip_prefix("alias_")
                    .unwrap_or(reference.spelling)
            ),
        ))
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn independent_actions_references_and_alternatives_generate_without_callbacks() {
    let session = machine();
    let original = session.model().clone();
    let output = CanonicalSerializer::new(
        language(MACHINE, MACHINE_TEXT),
        EvaluationLimits::workbench(),
        Default::default(),
    )
    .serialize(&original)
    .unwrap();
    assert_eq!(output.identities.len(), original.elements.len());
    assert!(output.sources[0].text.contains("state start"));
    assert!(output.sources[0].text.contains("edge next"));
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn generic_scalar_and_rename_keep_authored_order_quotes_comments_and_crlf() {
    let mut session = machine();
    let before = session.sources()[0].text.clone();
    let start = named(&session, "start");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[(start.clone(), "name".into(), json!("begin"))],
        )
        .unwrap();
    let prepared = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(prepared.sources()[0].text, before.replace("start", "begin"));
    session.adopt(prepared).unwrap();
    assert_eq!(named(&session, "begin"), start);
    let p = session.plan(session.model(), Default::default()).unwrap();
    assert!(p.patches().is_empty());
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn generic_subtree_creation_deletion_reordering_and_optional_fields_keep_comments() {
    let mut session = machine();
    let root = named(&session, "M");
    let start = named(&session, "start");
    let end = named(&session, "end");
    let schema = &session.package().schema;
    let mut candidate = session.model().clone();
    candidate
        .create(schema, "region", "Region", "machine.test", 0, 0)
        .unwrap();
    candidate
        .create(schema, "nested", "State", "machine.test", 0, 0)
        .unwrap();
    candidate
        .edit(
            schema,
            &[
                ("region".into(), "name".into(), json!("R")),
                ("nested".into(), "name".into(), json!("nested")),
                ("region".into(), "states".into(), json!(["nested"])),
                (root.clone(), "regions".into(), json!(["region"])),
                (root, "states".into(), json!([end, start.clone()])),
                (start.clone(), "label".into(), json!(null)),
            ],
        )
        .unwrap();
    let prepared = session.plan(&candidate, Default::default()).unwrap();
    assert!(prepared.sources()[0].text.contains("// retained"));
    assert!(prepared.sources()[0].text.contains("// header 😀"));
    session.adopt(prepared).unwrap();
    assert_eq!(named(&session, "start"), start);
    let old = session.sources()[0].text.clone();
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[("nested".into(), "label".into(), json!("Nested"))],
        )
        .unwrap();
    let prepared = session.plan(&candidate, Default::default()).unwrap();
    session.adopt(prepared).unwrap();
    assert!(session.sources()[0].text.contains("Nested"));
    assert!(old.contains("nested"));
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn expression_capture_actions_and_recursive_delegation_round_trip() {
    let structure = include_str!("../../../fixtures/xtext/model-to-text/expressions.rcore");
    let grammar = include_str!("../../../fixtures/xtext/model-to-text/expressions.xtext");
    let mut session = SourceEditSession::new(
        language(structure, grammar),
        &[Source {
            uri: "calc.test".into(),
            text: "calc 01 /* left */ + (2 + 3) // end\n".into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let model = session.model().clone();
    let output = CanonicalSerializer::new(
        language(structure, grammar),
        EvaluationLimits::workbench(),
        SerializationLimits {
            steps: 1_000_000,
            output_bytes: 1_000_000,
            nesting: 128,
        },
    )
    .serialize(&model)
    .unwrap();
    assert_eq!(output.identities.len(), model.elements.len());
    let mut candidate = model.clone();
    let number = candidate
        .elements
        .iter()
        .find(|e| e.values.get("value") == Some(&json!(2)))
        .unwrap()
        .id
        .clone();
    candidate
        .edit(
            &session.package().schema,
            &[(number, "value".into(), json!(20))],
        )
        .unwrap();
    let plan = session
        .plan(
            &candidate,
            SerializationLimits {
                steps: 1_000_000,
                output_bytes: 1_000_000,
                nesting: 128,
            },
        )
        .unwrap();
    assert_eq!(
        plan.sources()[0].text,
        "calc 01 /* left */ + (20 + 3) // end\n"
    );
    session.adopt(plan).unwrap();
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn datatype_enum_and_locally_significant_fragment_round_trip() {
    let structure = include_str!("../../../fixtures/xtext/model-to-text/records.rcore");
    let grammar = include_str!("../../../fixtures/xtext/model-to-text/records.xtext");
    let session = SourceEditSession::new(
        language(structure, grammar),
        &[Source {
            uri: "records.test".into(),
            text: "doc [Sample:x]!!! path Alpha::Beta slow 2147483647".into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let serializer = CanonicalSerializer::new(
        language(structure, grammar),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let output = serializer.serialize(session.model()).unwrap();
    assert!(output.sources[0].text.contains("[Sample:"));
    assert!(output.sources[0].text.contains("2147483647"));
    let mut candidate = session.model().clone();
    let root = candidate.elements[0].id.clone();
    candidate
        .edit(
            &session.package().schema,
            &[(root, "path".into(), json!("Gamma::Delta"))],
        )
        .unwrap();
    let prepared = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(
        prepared.sources()[0].text,
        "doc [Sample:x]!!! path Gamma::Delta slow 2147483647"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn general_plans_are_stale_checked_and_search_is_finite() {
    let mut session = machine();
    let id = named(&session, "start");
    let original = session.sources()[0].text.clone();
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[(id.clone(), "name".into(), json!("begin"))],
        )
        .unwrap();
    assert!(
        session
            .plan(
                &candidate,
                SerializationLimits {
                    steps: 0,
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(session.sources()[0].text, original);
    let stale = session.plan(&candidate, Default::default()).unwrap();
    candidate
        .edit(
            &session.package().schema,
            &[(id, "label".into(), json!("Begin"))],
        )
        .unwrap();
    let current = session.plan(&candidate, Default::default()).unwrap();
    session.adopt(current).unwrap();
    assert!(session.adopt(stale).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn duplicate_collection_values_keep_distinct_spellings_and_changed_strings_keep_quotes() {
    let mut session = SourceEditSession::new(
        language(MACHINE, MACHINE_TEXT),
        &[Source {
            uri: "values.test".into(),
            text: "machine M { state start label 'Start' tags ['one',\"one\",'two']; }\n".into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let id = named(&session, "start");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[
                (id.clone(), "label".into(), json!("It's ready")),
                (id, "final".into(), json!(true)),
            ],
        )
        .unwrap();
    let prepared = session.plan(&candidate, Default::default()).unwrap();
    assert!(prepared.sources()[0].text.contains("label 'It\\'s ready'"));
    assert!(
        prepared.sources()[0]
            .text
            .contains("tags ['one',\"one\",'two']")
    );
    session.adopt(prepared).unwrap();
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn generic_cross_resource_rename_preserves_unrelated_bytes_and_ids() {
    let sources = vec![
        Source {
            uri: "a.test".into(),
            text: "machine A { state alpha; } // A\r\n".into(),
        },
        Source {
            uri: "b.test".into(),
            text: "machine B { state beta; edge follow alpha -> beta; } // B\r\n".into(),
        },
    ];
    let mut session = SourceEditSession::new(
        language(MACHINE, MACHINE_TEXT),
        &sources,
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let alpha = named(&session, "alpha");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[(alpha.clone(), "name".into(), json!("renamed"))],
        )
        .unwrap();
    let prepared = session.plan(&candidate, Default::default()).unwrap();
    for (before, after) in sources.iter().zip(prepared.sources()) {
        assert_eq!(after.text, before.text.replace("alpha", "renamed"));
    }
    session.adopt(prepared).unwrap();
    assert_eq!(named(&session, "renamed"), alpha);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn grouped_containment_and_keyword_alternatives_share_grammar_inversion() {
    let structure = MACHINE.replace(
        "class Region {",
        "class Region { attr access: String [0,1];",
    );
    let grammar = format!(
        "{}\nRegionAlias returns m::Region: 'area' name=ID ('access' access=('public'|'private'))? '{{' states+=State* '}}';",
        MACHINE_TEXT.replace("regions+=Region", "regions+=(Region | RegionAlias)")
    );
    let mut session = SourceEditSession::new(
        language(&structure, &grammar),
        &[Source {
            uri: "groups.test".into(),
            text: "machine M { area R access private { state a; } }".into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let serializer = CanonicalSerializer::new(
        language(&structure, &grammar),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let output = serializer.serialize(session.model()).unwrap();
    assert!(output.sources[0].text.contains("area R access private"));
    let id = named(&session, "a");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[(id, "label".into(), json!("A"))],
        )
        .unwrap();
    let prepared = session.plan(&candidate, Default::default()).unwrap();
    session.adopt(prepared).unwrap();
    assert!(session.sources()[0].text.contains("area R access private"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_forward_and_inverse_scope_services_share_verification() {
    use intersect_text::{
        document::DocumentServices, linking::LinkingContext, serialization::default_spelling,
    };
    let prepared = language(MACHINE, MACHINE_TEXT);
    let sources = vec![Source {
        uri: "aliases.test".into(),
        text: "machine M { state alpha; state beta; edge next alias_alpha -> alias_beta; }".into(),
    }];
    assert!(
        !prepared
            .analyze_sources(&sources, None, EvaluationLimits::workbench())
            .unwrap()
            .diagnostics
            .is_empty()
    );
    let mut work = QueryBudget::new(10_000);
    let policy = Aliases;
    let mut linking = LinkingContext {
        resolver: &policy,
        work: &mut work,
    };
    let mut services = DocumentServices {
        linking: Some(&mut linking),
        ..Default::default()
    };
    let result = prepared
        .analyze_sources_with_services(&sources, EvaluationLimits::workbench(), &mut services)
        .unwrap();
    let model = &result.valid_model.as_ref().unwrap().model;
    let serializer = CanonicalSerializer::new(
        language(MACHINE, MACHINE_TEXT),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let output = serializer
        .serialize_with_services(model, &mut services, |value| {
            if let Some(target) = value.target {
                Ok(format!("alias_{}", target.values["name"].as_str().unwrap()))
            } else {
                default_spelling(serializer.package(), value)
            }
        })
        .unwrap();
    assert!(output.sources[0].text.contains("alias_alpha -> alias_beta"));
    assert_eq!(
        output
            .analysis
            .valid_model
            .as_ref()
            .unwrap()
            .references
            .len(),
        2
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn selected_serialization_entry_is_verified_with_the_same_parser_context() {
    use intersect_text::document::DocumentServices;
    let language = language(MACHINE, MACHINE_TEXT);
    let sources = vec![Source {
        uri: "state.test".into(),
        text: "state standalone label 'State';".into(),
    }];
    let mut services = DocumentServices {
        entry: Some("State"),
        ..Default::default()
    };
    let result = language
        .analyze_sources_with_services(&sources, EvaluationLimits::workbench(), &mut services)
        .unwrap();
    let model = &result.valid_model.as_ref().unwrap().model;
    let serializer =
        CanonicalSerializer::new(language, EvaluationLimits::workbench(), Default::default());
    assert!(serializer.serialize(model).is_err());
    let output = serializer
        .serialize_with_services(model, &mut services, |value| {
            intersect_text::serialization::default_spelling(serializer.package(), value)
        })
        .unwrap();
    assert!(output.sources[0].text.starts_with("state standalone"));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn preserving_sessions_require_their_original_scope_context_through_history() {
    use intersect_text::{
        document::DocumentServices, linking::LinkingContext, serialization::default_spelling,
        source_edit::SourceEditContext,
    };
    use std::{cell::Cell, sync::Arc};
    let language = Arc::new(language(MACHINE, MACHINE_TEXT));
    let policy = Aliases;
    let mut work = QueryBudget::new(100_000);
    let mut linking = LinkingContext {
        resolver: &policy,
        work: &mut work,
    };
    let mut context = SourceEditContext::new(DocumentServices {
        linking: Some(&mut linking),
        ..Default::default()
    });
    let sources=vec![Source{uri:"scope.test".into(),text:"// retain\r\nmachine M { state alpha; edge step alias_alpha -> alias_beta; state beta; }\r\n".into()}];
    let mut session = SourceEditSession::new_with_context(
        Arc::clone(&language),
        &sources,
        EvaluationLimits::workbench(),
        &mut context,
    )
    .unwrap();
    let saved = session.checkpoint();
    let alpha = named(&session, "alpha");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[(alpha.clone(), "name".into(), json!("renamed"))],
        )
        .unwrap();
    assert!(session.plan(&candidate, Default::default()).is_err());
    let mut foreign = SourceEditContext::new(DocumentServices::default());
    let called = Cell::new(false);
    assert!(
        session
            .plan_with_context(&candidate, Default::default(), &mut foreign, |_| {
                called.set(true);
                Err("Unexpected spelling".into())
            })
            .is_err()
    );
    assert!(!called.get());
    let plan = session
        .plan_with_context(&candidate, Default::default(), &mut context, |value| {
            if let Some(target) = value.target {
                Ok(format!("alias_{}", target.values["name"].as_str().unwrap()))
            } else {
                default_spelling(session.package(), value)
            }
        })
        .unwrap();
    assert_eq!(
        plan.sources()[0].text,
        sources[0].text.replace("alpha", "renamed")
    );
    session.adopt(plan).unwrap();
    assert_eq!(named(&session, "renamed"), alpha);
    assert!(session.plan_restore(&saved).is_err());
    context.reset_linking_budget(100_000).unwrap();
    let restore = session
        .plan_restore_with_context(&saved, &mut context)
        .unwrap();
    session.adopt(restore).unwrap();
    assert_eq!(session.sources()[0].text, sources[0].text);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn custom_nominal_conversion_is_shared_by_canonical_and_preserving_contexts() {
    use intersect_model::{
        conversion::RgbConverter,
        definitions::{
            compiler::import_datatype_bundle,
            conversion::{ConversionContract, ConversionRegistry, ProviderId},
            resources::ResourceSet,
        },
    };
    use intersect_text::{document::DocumentServices, source_edit::SourceEditContext};
    use std::{rc::Rc, sync::Arc};
    let model = DefinitionSource::new(
        "memory:paint.rcore",
        SourceKind::Rcore,
        r#"rcore 1; package paint version "1"; metamodel "urn:paint:text" prefix p { datatype RGB instanceClass "example.RGB" {} class Paint { attr name: String [1,1]; attr color: RGB [1,1]; } }"#,
    );
    let mut text = DefinitionSource::new(
        "memory:paint.xtext",
        SourceKind::Xtext,
        r#"grammar independent.Paint with org.eclipse.xtext.common.Terminals
    import "urn:paint:text" as p
    Paint returns p::Paint: 'paint' name=ID color=COLOR ';';
    terminal COLOR returns p::RGB: '#' ('0'..'9'|'a'..'f'|'A'..'F')+;
    "#,
    );
    let mut common = DefinitionSource::new(
        "classpath:/org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        include_str!(
            "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
        ),
    );
    common.dependencies.push(model.id.clone());
    text.dependencies = vec![model.id.clone(), common.id.clone()];
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![model.clone(), text.clone(), common],
        namespaces: vec![],
    };
    let declaration = ResourceSet::new(&bundle)
        .unwrap()
        .resolve(&model.id, "#//RGB")
        .unwrap();
    let datatype = import_datatype_bundle(&bundle, &declaration).unwrap();
    let mut registry = ConversionRegistry::default();
    registry
        .register_shared(
            &datatype,
            Rc::new(RgbConverter::new(ConversionContract {
                declarations: vec![],
                members: vec![],
                datatype: declaration.clone(),
                provider: ProviderId {
                    name: "example.rgb".into(),
                    version: "1".into(),
                },
                serializable: true,
            })),
        )
        .unwrap();
    let inputs = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:paint:text".into(),
        xtext: Some(text.id),
        rcore_behavior: model.id,
        bundle,
    };
    let package = ordinary::compile_with_context(
        inputs,
        Profile::NativeV80,
        &[],
        Some(&registry),
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let language = Arc::new(
        DocumentLanguage::prepare_with_context(
            &package,
            &[],
            Some(&registry),
            Default::default(),
            AnalysisLimits::workbench(),
        )
        .unwrap(),
    );
    let sources = vec![Source {
        uri: "paint.test".into(),
        text: "paint P #123abc; // retain spelling\r\n".into(),
    }];
    let mut context = SourceEditContext::new(DocumentServices {
        conversions: Some(&registry),
        ..Default::default()
    });
    let mut session = SourceEditSession::new_with_context(
        Arc::clone(&language),
        &sources,
        EvaluationLimits::workbench(),
        &mut context,
    )
    .unwrap();
    let serializer = CanonicalSerializer::new_shared(
        language,
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let generated = serializer
        .serialize_using_services(
            session.model(),
            &mut DocumentServices {
                conversions: Some(&registry),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(generated.sources[0].text.contains("#123ABC"));
    assert!(serializer.serialize(session.model()).is_err());
    let original = session.checkpoint();
    let mut candidate = session.model().clone();
    let paint = candidate.elements[0].id.clone();
    let value = serde_json::to_value(registry.parse(&declaration, "#abcdef").unwrap()).unwrap();
    candidate
        .edit_with_conversions(
            &session.package().schema,
            &[(paint, "color".into(), value)],
            &registry,
        )
        .unwrap();
    let plan = session
        .plan_using_context(&candidate, Default::default(), &mut context)
        .unwrap();
    assert_eq!(
        plan.sources()[0].text,
        sources[0].text.replace("#123abc", "#ABCDEF")
    );
    session.adopt(plan).unwrap();
    let restored = session
        .plan_restore_with_context(&original, &mut context)
        .unwrap();
    session.adopt(restored).unwrap();
    assert_eq!(session.sources()[0].text, sources[0].text);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn owning_native_services_retain_scope_through_edits_and_history() {
    use intersect_text::{
        serialization::{TerminalSpelling, default_spelling},
        text_services::{TextServiceProvider, TextServices},
    };
    use std::{rc::Rc, sync::Arc};
    struct Provider(Aliases);
    impl TextServiceProvider for Provider {
        fn scope(&self) -> Option<&dyn ReferenceCandidates> {
            Some(&self.0)
        }
        fn spelling(
            &self,
            package: &intersect_text::package::Package,
            _model: &intersect_model::core::model::Model,
            value: TerminalSpelling<'_>,
        ) -> Result<String, String> {
            if let Some(target) = value.target {
                Ok(format!(
                    "alias_{}",
                    target.values["name"].as_str().ok_or("Missing name")?
                ))
            } else {
                default_spelling(package, value)
            }
        }
    }
    let language = Arc::new(language(MACHINE, MACHINE_TEXT));
    let services = TextServices::new(Rc::new(Provider(Aliases)));
    let authored =
        "machine M { state alpha; edge step alpha -> alias_beta; state beta; } // retained\r\n";
    let preserved = SourceEditSession::new_with_services(
        Arc::clone(&language),
        &[Source {
            uri: "mixed.test".into(),
            text: authored.into(),
        }],
        EvaluationLimits::workbench(),
        services.clone(),
    )
    .unwrap();
    let mut candidate = preserved.model().clone();
    let beta = named(&preserved, "beta");
    candidate
        .edit(
            &preserved.package().schema,
            &[(beta, "final".into(), json!(true))],
        )
        .unwrap();
    let plan = preserved.plan(&candidate, Default::default()).unwrap();
    assert!(
        plan.sources()[0]
            .text
            .contains("edge step alpha -> alias_beta")
    );
    let original =
        "machine M { state alpha; edge step alias_alpha -> alias_beta; state beta; } // keep\r\n";
    let mut session = SourceEditSession::new_with_services(
        Arc::clone(&language),
        &[Source {
            uri: "owning.test".into(),
            text: original.into(),
        }],
        EvaluationLimits::workbench(),
        services.clone(),
    )
    .unwrap();
    let checkpoint = session.checkpoint();
    let mut candidate = session.model().clone();
    let id = named(&session, "alpha");
    candidate
        .edit(
            &session.package().schema,
            &[(id, "name".into(), json!("renamed"))],
        )
        .unwrap();
    let edit = session.plan(&candidate, Default::default()).unwrap();
    session.adopt(edit).unwrap();
    assert_eq!(
        session.sources()[0].text,
        original.replace("alpha", "renamed")
    );
    let serializer = CanonicalSerializer::new_shared(
        language,
        EvaluationLimits::workbench(),
        Default::default(),
    );
    assert!(
        serializer
            .serialize_with_native_services(session.model(), &services)
            .unwrap()
            .sources[0]
            .text
            .contains("alias_renamed")
    );
    let restored = session.plan_restore(&checkpoint).unwrap();
    session.adopt(restored).unwrap();
    assert_eq!(session.sources()[0].text, original);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn cross_resource_subtree_moves_preserve_comments_and_stable_handles() {
    let sources = vec![
        Source {
            uri: "a.test".into(),
            text: "machine A {\r\n /* moved */ region R { state nested label 'N'; } // region inline\r\n } // A\r\n".into(),
        },
        Source {
            uri: "b.test".into(),
            text: "machine B { state beta; } // B\r\n".into(),
        },
    ];
    let mut session = SourceEditSession::new(
        language(MACHINE, MACHINE_TEXT),
        &sources,
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let saved = session.checkpoint();
    let region = named(&session, "R");
    let child = named(&session, "nested");
    let a = named(&session, "A");
    let b = named(&session, "B");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[
                (a, "regions".into(), json!([])),
                (b, "regions".into(), json!([region])),
            ],
        )
        .unwrap();
    let mut store = intersect_model::core::frozen::FrozenStore::from_materialized(
        &session.package().schema,
        &candidate,
    )
    .unwrap();
    let mut relocation_work = QueryBudget::new(1_000_000);
    intersect_model::core::storage::StructuralContext {
        schema: &session.package().schema,
        conversions: None,
    }
    .relocate_subtree(&mut store, &region, "b.test", &mut relocation_work)
    .unwrap();
    let mut limited = intersect_model::core::frozen::FrozenStore::from_materialized(
        &session.package().schema,
        &candidate,
    )
    .unwrap();
    assert!(
        intersect_model::core::storage::StructuralContext {
            schema: &session.package().schema,
            conversions: None
        }
        .relocate_subtree(
            &mut limited,
            &region,
            "b.test",
            &mut QueryBudget::new(relocation_work.reads - 1)
        )
        .is_err()
    );
    assert_eq!(
        intersect_model::core::storage::export(&limited).unwrap(),
        candidate
    );
    let candidate = intersect_model::core::storage::export(&store).unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    let a = &proposal
        .sources()
        .iter()
        .find(|s| s.uri == "a.test")
        .unwrap()
        .text;
    let b = &proposal
        .sources()
        .iter()
        .find(|s| s.uri == "b.test")
        .unwrap()
        .text;
    assert!(!a.contains("/* moved */"));
    assert!(b.contains("/* moved */ region R"));
    assert!(b.contains("label 'N'"));
    assert!(b.contains("} // region inline\r\n"));
    assert!(!a.contains("region inline"));
    assert!(a.ends_with("// A\r\n"));
    assert!(b.ends_with("// B\r\n"));
    session.adopt(proposal).unwrap();
    assert_eq!(named(&session, "nested"), child);
    assert_eq!(
        session
            .model()
            .elements
            .iter()
            .find(|e| e.id == child)
            .unwrap()
            .resource,
        "b.test"
    );
    let restored = session.plan_restore(&saved).unwrap();
    session.adopt(restored).unwrap();
    assert_eq!(session.sources()[0].text, sources[0].text);
    assert_eq!(session.sources()[1].text, sources[1].text);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn resource_creation_deletion_and_root_replacement_restore_exact_sources() {
    use intersect_model::core::storage::{
        self, DeleteRequest, DeleteSelection, IncomingReferences, ModelStore, StructuralContext,
    };
    let mut session = machine();
    let original = session.sources()[0].text.clone();
    let saved = session.checkpoint();
    let mut candidate = session.model().clone();
    let schema = &session.package().schema;
    candidate
        .create(schema, "new-root", "Machine", "new.test", 0, 0)
        .unwrap();
    candidate
        .edit(schema, &[("new-root".into(), "name".into(), json!("New"))])
        .unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(proposal.sources().len(), 2);
    assert_eq!(proposal.sources()[0].text, original);
    session.adopt(proposal).unwrap();
    assert_eq!(named(&session, "New"), "new-root");
    let restored = session.plan_restore(&saved).unwrap();
    session.adopt(restored).unwrap();
    assert_eq!(session.sources().len(), 1);
    assert_eq!(session.sources()[0].text, original);
    let mut store = intersect_model::core::frozen::FrozenStore::from_materialized(
        &session.package().schema,
        session.model(),
    )
    .unwrap();
    let root = named(&session, "M");
    StructuralContext {
        schema: &session.package().schema,
        conversions: None,
    }
    .delete(
        &mut store,
        DeleteRequest {
            roots: &[&root],
            selection: DeleteSelection::Subtrees,
            incoming: IncomingReferences::Reject,
        },
        &mut QueryBudget::new(10000),
    )
    .unwrap();
    storage::atomic(&mut store, |store| {
        store.remove_resource_record("machine.test")
    })
    .unwrap();
    let mut replacement = storage::export(&store).unwrap();
    replacement
        .create(
            &session.package().schema,
            "replacement",
            "Machine",
            "other.test",
            0,
            0,
        )
        .unwrap();
    replacement
        .edit(
            &session.package().schema,
            &[("replacement".into(), "name".into(), json!("Other"))],
        )
        .unwrap();
    let proposal = session.plan(&replacement, Default::default()).unwrap();
    session.adopt(proposal).unwrap();
    assert_eq!(session.sources().len(), 1);
    assert_eq!(session.sources()[0].uri, "other.test");
    let restored = session.plan_restore(&saved).unwrap();
    session.adopt(restored).unwrap();
    assert_eq!(named(&session, "M"), root);
    assert_eq!(session.sources()[0].text, original);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn custom_formatting_is_verified_and_preservation_keeps_existing_layout() {
    use intersect_text::{document::DocumentServices, serialization::TextFormatter};
    struct Layout(bool);
    impl TextFormatter for Layout {
        fn format(
            &self,
            _package: &intersect_text::package::Package,
            _model: &intersect_model::core::model::Model,
            source: &Source,
        ) -> Result<String, String> {
            Ok(if self.0 {
                source.text.replace("start", "wrong")
            } else {
                format!("// formatted\r\n{}", source.text.replace('\n', "\r\n"))
            })
        }
    }
    let session = machine();
    let serializer = CanonicalSerializer::new(
        language(MACHINE, MACHINE_TEXT),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let output = serializer
        .serialize_using_services(
            session.model(),
            &mut DocumentServices {
                formatter: Some(&Layout(false)),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(output.sources[0].text.starts_with("// formatted\r\n"));
    let rejecting = CanonicalSerializer::new(
        language(MACHINE, MACHINE_TEXT),
        EvaluationLimits::workbench(),
        SerializationLimits {
            steps: 20_000,
            output_bytes: 8_000,
            ..Default::default()
        },
    );
    assert!(
        rejecting
            .serialize_using_services(
                session.model(),
                &mut DocumentServices {
                    formatter: Some(&Layout(true)),
                    ..Default::default()
                }
            )
            .is_err()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn grammar_assigned_computed_values_use_the_installed_inverse_getter() {
    computed_projection_round_trip(false, false);
    computed_projection_round_trip(true, false);
    computed_projection_round_trip(false, true);
    computed_projection_round_trip(true, true);
}
fn computed_projection_round_trip(containment: bool, transient_backing: bool) {
    use intersect_model::{
        core::storage::{ComputedEdit, ModelRead, SettingEdit, SettingValues},
        definitions::{
            conversion::ProviderId,
            setting::{ComputedSettingContract, SettingOperation},
        },
        query::DerivedValues,
        setting::SourceSettingValues,
    };
    use intersect_text::{document::DocumentServices, source_edit::SourceEditContext};
    use std::sync::Arc;
    let mut model = DefinitionSource::new(
        "memory:computed.rcore",
        SourceKind::Rcore,
        if containment {
            r#"rcore 1; package computed version "1"; metamodel "urn:computed:text" prefix c { class Root { ref backing: Leaf [1,1] containment; ref computed: Leaf [1,1] containment derived; } class Leaf { attr name: String [1,1]; } }"#
        } else if transient_backing {
            r#"rcore 1; package computed version "1"; metamodel "urn:computed:text" prefix c { class Root { attr backing: String [1,1] transient; attr computed: String [1,1] derived; } } behavior { import model "urn:computed:text" as c; projection c.Root.computed from backing writable; }"#
        } else {
            r#"rcore 1; package computed version "1"; metamodel "urn:computed:text" prefix c { class Root { attr backing: String [1,1]; attr computed: String [1,1] derived; } } behavior { import model "urn:computed:text" as c; projection c.Root.computed from backing writable; }"#
        },
    );
    if containment && transient_backing {
        model = DefinitionSource::new(
            model.id.uri,
            SourceKind::Rcore,
            model.text.replace(
                "ref backing: Leaf [1,1] containment;",
                "ref backing: Leaf [1,1] containment transient;",
            ),
        );
    }
    let mut text = DefinitionSource::new(
        "memory:computed.xtext",
        SourceKind::Xtext,
        r#"grammar independent.Computed hidden(WS)
    import "http://www.eclipse.org/emf/2002/Ecore" as ecore
    import "urn:computed:text" as c
    Root returns c::Root: 'root' computed=ID ';';
    terminal ID: ('a'..'z'|'A'..'Z')+;
    terminal WS: (' '|'\t'|'\r'|'\n')+;"#,
    );
    if containment {
        text = DefinitionSource::new(
            "memory:computed.xtext",
            SourceKind::Xtext,
            r#"grammar independent.Computed hidden(WS)
            import "http://www.eclipse.org/emf/2002/Ecore" as ecore
            import "urn:computed:text" as c
            Root returns c::Root: 'root' computed=Leaf ';';
            Leaf returns c::Leaf: name=ID;
            terminal ID: ('a'..'z'|'A'..'Z')+;
            terminal WS: (' '|'\t'|'\r'|'\n')+;"#,
        );
    }
    text.dependencies = vec![model.id.clone()];
    let inputs = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:computed:text".into(),
        xtext: None,
        rcore_behavior: model.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), text.clone()],
            namespaces: vec![],
        },
    };
    let mut structure = intersect_model::definitions::compiler::compile_structure_bundle(
        &inputs.bundle,
        &inputs.ecore,
        &inputs.namespace,
    )
    .unwrap();
    intersect_text::grammar::prepare_ecore_reference_context(&mut structure).unwrap();
    let schema = &structure;
    let contract = ComputedSettingContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class: "Root".into(),
        class_identity: schema.identity("Root"),
        feature: "computed".into(),
        member: schema
            .effective_feature("Root", "computed")
            .unwrap()
            .identity,
        provider: ProviderId {
            name: "test.inverse".into(),
            version: "1".into(),
        },
        sources: vec![model.id.clone(), text.id.clone()],
        operations: vec![SettingOperation::Set],
    };
    struct Projection(ComputedSettingContract);
    impl DerivedValues for Projection {
        fn schema_revision(&self) -> Option<&str> {
            Some(&self.0.schema_revision)
        }
        fn supports(&self, class: &str, feature: &str) -> bool {
            class == "Root" && feature == "computed"
        }
        fn read(
            &self,
            store: &dyn ModelRead,
            id: &str,
            _feature: &str,
            work: &mut QueryBudget,
        ) -> Result<serde_json::Value, String> {
            work.charge()?;
            store.value(id, "backing")?.ok_or("Missing backing".into())
        }
    }
    impl SettingValues for Projection {
        fn plan(
            &self,
            _store: &dyn ModelRead,
            id: &str,
            _feature: &str,
            edit: &ComputedEdit,
            work: &mut QueryBudget,
        ) -> Result<Vec<SettingEdit>, String> {
            work.charge()?;
            match edit {
                ComputedEdit::Set(value) => Ok(vec![SettingEdit::Set {
                    id: id.into(),
                    feature: "backing".into(),
                    value: value.clone(),
                }]),
                _ => Err("Unsupported operation".into()),
            }
        }
    }
    impl SourceSettingValues for Projection {
        fn contract(&self) -> &ComputedSettingContract {
            &self.0
        }
    }
    let mut selected = inputs.clone();
    selected.xtext = Some(text.id);
    if containment {
        assert!(ordinary::compile(selected.clone(), Profile::NativeV80).is_err());
    }
    let package = ordinary::compile_with_context(
        selected,
        Profile::NativeV80,
        std::slice::from_ref(&contract),
        None,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    let language = Arc::new(
        DocumentLanguage::prepare_with_context(
            &package,
            std::slice::from_ref(&contract),
            None,
            Default::default(),
            AnalysisLimits::workbench(),
        )
        .unwrap(),
    );
    let provider = Projection(contract);
    let settings = TextSettings::new(language.package(), &inputs.bundle, &[&provider]).unwrap();
    let mut context = SourceEditContext::new(DocumentServices {
        settings: Some(&settings),
        derived: Some(&provider),
        ..Default::default()
    });
    let source = Source {
        uri: "computed.test".into(),
        text: "root hello ;\r\n".into(),
    };
    let mut session = SourceEditSession::new_with_context(
        Arc::clone(&language),
        std::slice::from_ref(&source),
        EvaluationLimits::workbench(),
        &mut context,
    )
    .unwrap();
    let mut candidate = session.model().clone();
    let root = candidate
        .elements
        .iter()
        .find(|e| e.class == "Root")
        .unwrap();
    let (id, field) = if containment {
        (root.values["backing"].as_str().unwrap().to_owned(), "name")
    } else {
        (root.id.clone(), "backing")
    };
    candidate
        .edit(
            &session.package().schema,
            &[(id, field.into(), json!("changed"))],
        )
        .unwrap();
    let plan = session
        .plan_using_context(&candidate, Default::default(), &mut context)
        .unwrap();
    session.adopt(plan).unwrap();
    assert_eq!(
        session.sources()[0].text,
        source.text.replace("hello", "changed")
    );
    let serializer = CanonicalSerializer::new_shared(
        Arc::clone(&language),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    assert!(
        serializer
            .serialize_using_services(
                session.model(),
                &mut DocumentServices {
                    settings: Some(&settings),
                    derived: Some(&provider),
                    ..Default::default()
                }
            )
            .unwrap()
            .sources[0]
            .text
            .contains("changed")
    );
    if transient_backing {
        struct Corrupt;
        impl intersect_text::serialization::TextFormatter for Corrupt {
            fn format(
                &self,
                _package: &intersect_text::package::Package,
                _model: &intersect_model::core::model::Model,
                source: &Source,
            ) -> Result<String, String> {
                Ok(source.text.replace("changed", "wrong"))
            }
        }
        let error = serializer
            .serialize_using_services(
                session.model(),
                &mut DocumentServices {
                    settings: Some(&settings),
                    derived: Some(&provider),
                    formatter: Some(&Corrupt),
                    ..Default::default()
                },
            )
            .unwrap_err();
        assert!(error.contains("computed"), "{error}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn unassigned_text_uses_model_values_and_retained_source_without_bypassing_verification() {
    use intersect_text::{
        document::DocumentServices,
        serialization::{UnassignedSpelling, UnassignedText},
        text_services::{TextServiceProvider, TextServices},
    };
    use std::{rc::Rc, sync::Arc};
    struct Plural;
    impl UnassignedText for Plural {
        fn spelling(
            &self,
            _package: &intersect_text::package::Package,
            _model: &intersect_model::core::model::Model,
            value: UnassignedSpelling<'_>,
        ) -> Result<Option<String>, String> {
            if value.rule.rsplit("::").next() != Some("Plural") {
                return Ok(None);
            }
            if let Some(original) = value.original
                && original != "item"
                && original != "items"
            {
                return Err(format!("Wrong retained unassigned node: {original:?}"));
            }
            Ok(Some(
                if value.owner.ok_or("Missing Count owner")?.values["count"] == json!(1) {
                    "item"
                } else {
                    "items"
                }
                .into(),
            ))
        }
    }
    impl TextServiceProvider for Plural {
        fn unassigned(&self) -> Option<&dyn UnassignedText> {
            Some(self)
        }
    }
    let structure = r#"rcore 1; package phrases version "1"; metamodel "urn:general-text" prefix p { class Count { attr count: Int32 [1,1]; } }"#;
    let grammar = r#"grammar independent.Phrases with org.eclipse.xtext.common.Terminals
    import "urn:general-text" as p
    import "http://www.eclipse.org/emf/2002/Ecore" as ecore
    Count returns p::Count: 'count' count=INT Plural ';';
    Plural returns ecore::EString: 'item' | 'items';"#;
    let language = Arc::new(language(structure, grammar));
    let services = TextServices::new(Rc::new(Plural));
    let source = Source {
        uri: "phrases.test".into(),
        text: "count 2 items; // preserve\r\n".into(),
    };
    let mut session = SourceEditSession::new_with_services(
        Arc::clone(&language),
        std::slice::from_ref(&source),
        EvaluationLimits::workbench(),
        services.clone(),
    )
    .unwrap();
    let serializer = CanonicalSerializer::new_shared(
        Arc::clone(&language),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let canonical = serializer
        .serialize_with_native_services(session.model(), &services)
        .unwrap();
    assert!(canonical.sources[0].text.contains("2 items"));
    let mut candidate = session.model().clone();
    let id = candidate.elements[0].id.clone();
    candidate
        .edit(&session.package().schema, &[(id, "count".into(), json!(1))])
        .unwrap();
    let plan = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(plan.sources()[0].text, "count 1 item; // preserve\r\n");
    session.adopt(plan).unwrap();
    let exact = session.plan(session.model(), Default::default()).unwrap();
    assert!(exact.patches().is_empty());
    struct Bad;
    impl UnassignedText for Bad {
        fn spelling(
            &self,
            _package: &intersect_text::package::Package,
            _model: &intersect_model::core::model::Model,
            _value: UnassignedSpelling<'_>,
        ) -> Result<Option<String>, String> {
            Ok(Some("items injected".into()))
        }
    }
    let rejecting = CanonicalSerializer::new_shared(
        language,
        EvaluationLimits::workbench(),
        SerializationLimits {
            steps: 20_000,
            output_bytes: 8_000,
            ..Default::default()
        },
    );
    assert!(
        rejecting
            .serialize_using_services(
                session.model(),
                &mut DocumentServices {
                    unassigned: Some(&Bad),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(session.sources()[0].text, "count 1 item; // preserve\r\n");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn empty_gap_preservation_uses_the_parser_to_insert_required_separators() {
    let structure = r#"rcore 1; package gaps version "1"; metamodel "urn:general-text" prefix g { class Root { attr name: String [1,1]; attr colon: Bool [0,1]; } }"#;
    let grammar = r#"grammar independent.Gaps with org.eclipse.xtext.common.Terminals
    import "urn:general-text" as g
    Root returns g::Root: 'root' colon?=':'? name=ID ';';"#;
    let source = Source {
        uri: "gaps.test".into(),
        text: "root:abc; // preserve\r\n".into(),
    };
    let mut session = SourceEditSession::new(
        language(structure, grammar),
        std::slice::from_ref(&source),
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let mut candidate = session.model().clone();
    let id = candidate.elements[0].id.clone();
    candidate
        .edit(
            &session.package().schema,
            &[(id, "colon".into(), json!(false))],
        )
        .unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(proposal.sources()[0].text, "root abc; // preserve\r\n");
    session.adopt(proposal).unwrap();
    assert_eq!(named(&session, "abc"), session.model().elements[0].id);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn transient_attributes_and_subtrees_preserve_runtime_state_through_text_and_history() {
    let structure = r#"rcore 1; package caches version "1"; metamodel "urn:general-text" prefix c {
      class Root { attr name: String [1,1]; attr cache: Int32 [0,1] transient unsettable; ref cached: Cache [0,*] containment transient; }
      class Cache { attr name: String [1,1]; }
    }"#;
    let grammar = r#"grammar independent.Caches with org.eclipse.xtext.common.Terminals
      import "urn:general-text" as c
      Root returns c::Root: 'root' name=ID ';';"#;
    let prepared = language(structure, grammar);
    let mut session = SourceEditSession::new(
        prepared,
        &[Source {
            uri: "caches.test".into(),
            text: "root Main; // source stays authored\r\n".into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let root = named(&session, "Main");
    let saved = session.checkpoint();
    let mut candidate = session.model().clone();
    let schema = &session.package().schema;
    candidate
        .create(schema, "runtime-cache", "Cache", "caches.test", 0, 0)
        .unwrap();
    candidate
        .edit(
            schema,
            &[
                (root.clone(), "name".into(), json!("Renamed")),
                (root.clone(), "cache".into(), json!(7)),
                ("runtime-cache".into(), "name".into(), json!("runtime")),
                (root.clone(), "cached".into(), json!(["runtime-cache"])),
            ],
        )
        .unwrap();
    let serializer = CanonicalSerializer::new(
        language(structure, grammar),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let canonical = serializer.serialize(&candidate).unwrap();
    assert_eq!(canonical.identities.len(), 1);
    assert_eq!(
        canonical
            .analysis
            .valid_model
            .as_ref()
            .unwrap()
            .model
            .elements
            .len(),
        1
    );
    assert!(!canonical.sources[0].text.contains("runtime"));
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(
        proposal.sources()[0].text,
        "root Renamed; // source stays authored\r\n"
    );
    session.adopt(proposal).unwrap();
    assert_eq!(named(&session, "runtime"), "runtime-cache");
    assert!(
        session
            .model()
            .is_set(&session.package().schema, &root, "cache")
            .unwrap()
    );
    let before = session.sources()[0].text.clone();
    let revision = session.model().resources["caches.test"].content_revision;
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[
                (root.clone(), "cache".into(), json!(8)),
                ("runtime-cache".into(), "name".into(), json!("cached again")),
            ],
        )
        .unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert!(proposal.patches().is_empty());
    session.adopt(proposal).unwrap();
    assert_eq!(session.sources()[0].text, before);
    assert_eq!(
        session
            .model()
            .elements
            .iter()
            .find(|element| element.id == root)
            .unwrap()
            .values["cache"],
        json!(8)
    );
    assert!(session.model().resources["caches.test"].content_revision > revision);
    let restored = session.plan_restore(&saved).unwrap();
    session.adopt(restored).unwrap();
    assert_eq!(session.model().elements.len(), 1);
    assert_eq!(named(&session, "Main"), root);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn custom_transient_members_share_persistent_graph_verification_and_stable_handles() {
    use intersect_text::{
        serialization::{TransientValue, TransientValues, default_transient},
        text_services::{TextServiceProvider, TextServices},
    };
    use std::{rc::Rc, sync::Arc};
    struct Policy;
    impl TransientValues for Policy {
        fn is_transient(
            &self,
            package: &intersect_text::package::Package,
            model: &intersect_model::core::model::Model,
            value: TransientValue<'_>,
        ) -> Result<bool, String> {
            if value.feature == "children" && value.index.is_some() {
                let id = value.value.as_str().ok_or("Expected child ID")?;
                return Ok(model
                    .elements
                    .iter()
                    .any(|element| element.id == id && element.class == "Cache"));
            }
            default_transient(package, value)
        }
    }
    impl TextServiceProvider for Policy {
        fn transient(&self) -> Option<&dyn TransientValues> {
            Some(self)
        }
    }
    let structure = r#"rcore 1; package mixed version "1"; metamodel "urn:general-text" prefix m {
      abstract class Entry { attr name: String [1,1]; }
      class Root { attr name: String [1,1]; ref children: Entry [0,*] containment; }
      class Leaf extends Entry {} class Cache extends Entry {}
    }"#;
    let grammar = r#"grammar independent.Mixed with org.eclipse.xtext.common.Terminals
      import "urn:general-text" as m
      Root returns m::Root: 'root' name=ID '{' children+=Leaf* '}';
      Leaf returns m::Leaf: 'leaf' name=ID ';';"#;
    let prepared = Arc::new(language(structure, grammar));
    let services = TextServices::new(Rc::new(Policy));
    let source = Source {
        uri: "mixed.test".into(),
        text: "root R { leaf first; /* keep */ leaf last; }\r\n".into(),
    };
    let mut session = SourceEditSession::new_with_services(
        Arc::clone(&prepared),
        std::slice::from_ref(&source),
        EvaluationLimits::workbench(),
        services.clone(),
    )
    .unwrap();
    let saved = session.checkpoint();
    let root = named(&session, "R");
    let first = named(&session, "first");
    let last = named(&session, "last");
    let mut candidate = session.model().clone();
    candidate
        .create(
            &session.package().schema,
            "cache",
            "Cache",
            "mixed.test",
            0,
            0,
        )
        .unwrap();
    candidate
        .edit(
            &session.package().schema,
            &[
                ("cache".into(), "name".into(), json!("hidden")),
                (
                    root.clone(),
                    "children".into(),
                    json!([first, "cache", last]),
                ),
            ],
        )
        .unwrap();
    let serializer = CanonicalSerializer::new_shared(
        prepared,
        EvaluationLimits::workbench(),
        Default::default(),
    );
    assert!(serializer.serialize(&candidate).is_err());
    let generated = serializer
        .serialize_with_native_services(&candidate, &services)
        .unwrap();
    assert_eq!(generated.identities.len(), 3);
    assert!(!generated.sources[0].text.contains("hidden"));
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(proposal.sources()[0].text, source.text);
    session.adopt(proposal).unwrap();
    assert_eq!(named(&session, "first"), first);
    assert_eq!(named(&session, "last"), last);
    assert_eq!(named(&session, "hidden"), "cache");
    assert_eq!(
        session
            .model()
            .elements
            .iter()
            .find(|element| element.id == root)
            .unwrap()
            .values["children"],
        json!([first, "cache", last])
    );
    let restored = session.plan_restore(&saved).unwrap();
    session.adopt(restored).unwrap();
    assert_eq!(session.model().elements.len(), 3);
    assert_eq!(session.sources()[0].text, source.text);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn empty_object_entries_generate_and_retain_resources_through_root_lifecycle() {
    use intersect_model::core::storage::{
        self, DeleteRequest, DeleteSelection, IncomingReferences, StructuralContext,
    };
    let structure = r#"rcore 1; package empty version "1"; metamodel "urn:general-text" prefix e { class Root { attr name: String [1,1]; } }"#;
    let grammar = r#"grammar independent.Empty with org.eclipse.xtext.common.Terminals
      import "urn:general-text" as e
      import "http://www.eclipse.org/emf/2002/Ecore" as ecore
      Root returns e::Root: 'header' Stamp (name=ID ';')?;
      Stamp returns ecore::EString: 'v1' | 'v2';"#;
    let source = Source {
        uri: "empty.test".into(),
        text: "// header comment\r\nheader v2 /* empty */\r\n".into(),
    };
    let mut session = SourceEditSession::new(
        language(structure, grammar),
        std::slice::from_ref(&source),
        EvaluationLimits::workbench(),
    )
    .unwrap();
    assert!(session.model().elements.is_empty());
    assert!(session.model().resources["empty.test"].roots.is_empty());
    let serializer = CanonicalSerializer::new(
        language(structure, grammar),
        EvaluationLimits::workbench(),
        Default::default(),
    );
    let canonical = serializer.serialize(session.model()).unwrap();
    assert!(canonical.identities.is_empty());
    assert!(canonical.sources[0].text.contains("header"));
    struct Stamp;
    impl intersect_text::serialization::UnassignedText for Stamp {
        fn spelling(
            &self,
            _package: &intersect_text::package::Package,
            _model: &intersect_model::core::model::Model,
            value: intersect_text::serialization::UnassignedSpelling<'_>,
        ) -> Result<Option<String>, String> {
            if value.owner.is_some() || value.resource != "empty.test" {
                return Err("Wrong rootless spelling context".into());
            }
            Ok(Some("v2".into()))
        }
    }
    let selected = serializer
        .serialize_using_services(
            session.model(),
            &mut intersect_text::document::DocumentServices {
                unassigned: Some(&Stamp),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(selected.sources[0].text.contains("v2"));
    assert_eq!(
        session
            .plan(session.model(), Default::default())
            .unwrap()
            .sources()[0]
            .text,
        source.text
    );
    let original = session.checkpoint();
    let mut candidate = session.model().clone();
    candidate
        .create(&session.package().schema, "new", "Root", "empty.test", 0, 0)
        .unwrap();
    candidate
        .edit(
            &session.package().schema,
            &[("new".into(), "name".into(), json!("created"))],
        )
        .unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert!(proposal.sources()[0].text.contains("v2"));
    assert!(
        proposal.sources()[0]
            .text
            .starts_with("// header comment\r\nheader v2")
    );
    session.adopt(proposal).unwrap();
    assert_eq!(named(&session, "created"), "new");
    let mut store = intersect_model::core::frozen::FrozenStore::from_materialized(
        &session.package().schema,
        session.model(),
    )
    .unwrap();
    StructuralContext {
        schema: &session.package().schema,
        conversions: None,
    }
    .delete(
        &mut store,
        DeleteRequest {
            roots: &["new"],
            selection: DeleteSelection::Subtrees,
            incoming: IncomingReferences::Reject,
        },
        &mut QueryBudget::new(1_000_000),
    )
    .unwrap();
    let candidate = storage::export(&store).unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert!(proposal.sources()[0].text.contains("v2"));
    assert!(
        proposal.sources()[0]
            .text
            .starts_with("// header comment\r\nheader v2")
    );
    session.adopt(proposal).unwrap();
    assert!(session.model().elements.is_empty());
    assert_eq!(session.model().resources.len(), 1);
    session
        .adopt(session.plan_restore(&original).unwrap())
        .unwrap();
    assert_eq!(session.sources()[0].text, source.text);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn negated_terminal_witnesses_include_unicode_complements() {
    let structure = r#"rcore 1; package unicode version "1"; metamodel "urn:general-text" prefix u { class Root { attr name: String [1,1]; } }"#;
    let grammar = r#"grammar independent.Unicode with org.eclipse.xtext.common.Terminals
      import "urn:general-text" as u
      import "http://www.eclipse.org/emf/2002/Ecore" as ecore
      Root returns u::Root: 'root' name=ID MARK ';';
      terminal MARK: !('\u0000'..'\u007F');"#;
    // MARK consumes one UTF-16 unit; the independent input uses a BMP scalar.
    let source = Source {
        uri: "unicode.test".into(),
        text: "root original Ω;".into(),
    };
    let session = SourceEditSession::new(
        language(structure, grammar),
        &[source],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let output = CanonicalSerializer::new(
        language(structure, grammar),
        EvaluationLimits::workbench(),
        Default::default(),
    )
    .serialize(session.model())
    .unwrap();
    assert!(!output.sources[0].text.is_ascii());
    assert_eq!(output.identities.len(), 1);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn moving_syntax_into_a_different_hidden_context_retains_comments_where_legal() {
    let structure = r#"rcore 1; package hiddenMove version "1"; metamodel "urn:general-text" prefix h {
        class Root { ref loose: Item [0,*] containment; ref tight: Item [0,*] containment; }
        class Item { attr name: String [1,1]; }
    }"#;
    let grammar = r#"grammar independent.HiddenMove with org.eclipse.xtext.common.Terminals
        import "urn:general-text" as h
        Root returns h::Root: {h::Root} 'loose' loose+=Loose* 'tight' tight+=Tight*;
        Loose returns h::Item: Item;
        Tight returns h::Item hidden(): Item;
        Item returns h::Item: '[' name=ID ']' ';';"#;
    let source = Source {
        uri: "hidden.test".into(),
        text: "loose\r\n [ /* keep comment */ one ] ; // inline\r\n tight".into(),
    };
    let mut session = SourceEditSession::new(
        language(structure, grammar),
        std::slice::from_ref(&source),
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let original = session.checkpoint();
    let root = session
        .model()
        .elements
        .iter()
        .find(|object| object.class == "Root")
        .unwrap()
        .id
        .clone();
    let item = named(&session, "one");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[
                (root.clone(), "loose".into(), json!([])),
                (root, "tight".into(), json!([item])),
            ],
        )
        .unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(
        proposal.sources()[0]
            .text
            .matches("/* keep comment */")
            .count(),
        1
    );
    assert_eq!(proposal.sources()[0].text.matches("// inline").count(), 1);
    assert!(proposal.sources()[0].text.contains("[one];"));
    session.adopt(proposal).unwrap();
    assert_eq!(named(&session, "one"), item);
    session
        .adopt(session.plan_restore(&original).unwrap())
        .unwrap();
    assert_eq!(session.sources()[0].text, source.text);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn grammar_defined_hidden_separators_generate_and_preserve_without_whitespace_assumptions() {
    let structure = r#"rcore 1; package pad version "1"; metamodel "urn:general-text" prefix p { class Root { attr name: String [1,1]; } }"#;
    for (hidden, input, expected) in [
        ("terminal PAD: '#'+;", "root###Main;", "#"),
        (
            "terminal PAD: '/*' -> '*/';",
            "root/* authored */Main;",
            "/**/",
        ),
    ] {
        let grammar = format!(
            r#"grammar independent.Pad hidden(PAD)
            import "urn:general-text" as p
            import "http://www.eclipse.org/emf/2002/Ecore" as ecore
            Root returns p::Root: 'root' name=ID ';';
            terminal ID: ('a'..'z'|'A'..'Z')+;
            {hidden}"#
        );
        let source = Source {
            uri: "pad.test".into(),
            text: input.into(),
        };
        let mut session = SourceEditSession::new(
            language(structure, &grammar),
            std::slice::from_ref(&source),
            EvaluationLimits::workbench(),
        )
        .unwrap();
        let output = CanonicalSerializer::new(
            language(structure, &grammar),
            EvaluationLimits::workbench(),
            Default::default(),
        )
        .serialize(session.model())
        .unwrap();
        assert!(output.sources[0].text.contains(expected));
        let root = named(&session, "Main");
        let mut candidate = session.model().clone();
        candidate
            .edit(
                &session.package().schema,
                &[(root, "name".into(), json!("Changed"))],
            )
            .unwrap();
        let proposal = session.plan(&candidate, Default::default()).unwrap();
        assert_eq!(proposal.sources()[0].text, input.replace("Main", "Changed"));
        session.adopt(proposal).unwrap();
        assert_eq!(
            session
                .plan(session.model(), Default::default())
                .unwrap()
                .sources()[0]
                .text,
            session.sources()[0].text
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn native_v81_builtin_terminal_and_parser_returns_share_all_scalar_codecs() {
    use intersect_model::definitions::scalars::Builtin;
    let examples = [
        ("EString", "hello Ω"),
        ("EBoolean", "true"),
        ("EByte", "-128"),
        ("EShort", "32767"),
        ("EInt", "2147483647"),
        ("ELong", "9223372036854775807"),
        ("EBooleanObject", "false"),
        ("EByteObject", "127"),
        ("EShortObject", "-32768"),
        ("EIntegerObject", "-2147483648"),
        ("ELongObject", "-9223372036854775808"),
        ("EFloat", "-0.0"),
        ("EDouble", "Infinity"),
        ("EFloatObject", "NaN"),
        ("EDoubleObject", "1.23456789"),
        ("EBigInteger", "123456789012345678901234567890"),
        ("EBigDecimal", "123.4500"),
        ("EDate", "1970-01-01T00:00:00.000+0000"),
        ("EByteArray", "00ffA1"),
        ("EChar", "Ω"),
        ("ECharacterObject", "Z"),
    ];
    for (name, lexical) in examples {
        let codec = Builtin::named(name).unwrap();
        let expected = codec
            .parse(lexical)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let structure = format!(
            r#"rcore 1; package scalars version "1"; metamodel "urn:general-text" prefix s {{
            class Root {{ attr value: {name} [1,1]; }}
        }}"#
        );
        for parser_rule in [false, true] {
            let rules = if !parser_rule {
                format!(
                    "Root returns s::Root: value=VALUE; terminal VALUE returns ecore::{name}: .+;"
                )
            } else if name == "EBoolean" {
                "Root returns s::Root: value=Value; Value returns ecore::EBoolean: 'true' | 'false';".into()
            } else {
                format!(
                    "Root returns s::Root: value=Value; Value returns ecore::{name}: RAW; terminal RAW: .+;"
                )
            };
            let grammar = format!(
                r#"grammar independent.Scalars hidden()
                import "urn:general-text" as s
                import "http://www.eclipse.org/emf/2002/Ecore" as ecore
                {rules}"#
            );
            let package = ordinary::compile(compilation(&structure, &grammar), Profile::NativeV81)
                .unwrap_or_else(|error| panic!("{name} parser={parser_rule}: {error:?}"));
            let prepared = DocumentLanguage::prepare(
                &package,
                Default::default(),
                AnalysisLimits::workbench(),
            )
            .unwrap();
            let source = Source {
                uri: "scalar.test".into(),
                text: lexical.into(),
            };
            let mut session = SourceEditSession::new(
                prepared,
                std::slice::from_ref(&source),
                EvaluationLimits::workbench(),
            )
            .unwrap_or_else(|error| panic!("{name} parser={parser_rule}: {error}"));
            assert_eq!(
                session.model().elements[0].values["value"],
                expected,
                "{name}, parser={parser_rule}"
            );
            let canonical = CanonicalSerializer::new(
                language_with_profile(&structure, &grammar, Profile::NativeV81),
                EvaluationLimits::workbench(),
                Default::default(),
            )
            .serialize(session.model())
            .unwrap_or_else(|error| panic!("{name} parser={parser_rule}: {error}"));
            assert_eq!(
                canonical.analysis.valid_model.unwrap().model.elements[0].values["value"],
                expected,
                "{name}"
            );
            assert_eq!(
                session
                    .plan(session.model(), Default::default())
                    .unwrap()
                    .sources()[0]
                    .text,
                lexical
            );
            let replacement = match name {
                "EString" => "changed Ω",
                "EBoolean" => "false",
                "EBooleanObject" => "true",
                "EDate" => "2001-02-03T04:05:06.007+0000",
                "EByteArray" => "ABCD",
                "EChar" | "ECharacterObject" => "Q",
                "EBigDecimal" => "7.00",
                _ => "1",
            };
            let value = codec.parse(replacement).unwrap();
            let mut candidate = session.model().clone();
            let id = candidate.elements[0].id.clone();
            candidate
                .edit(
                    &session.package().schema,
                    &[(id.clone(), "value".into(), value.clone())],
                )
                .unwrap();
            let proposal = session
                .plan(&candidate, Default::default())
                .unwrap_or_else(|error| panic!("{name} edit parser={parser_rule}: {error}"));
            assert_eq!(
                proposal.sources()[0].text,
                codec.format(&value).unwrap().unwrap(),
                "{name} preserving edit"
            );
            session.adopt(proposal).unwrap();
            assert_eq!(session.model().elements[0].id, id);
            assert_eq!(session.model().elements[0].values["value"], value);
            if name == "ELong" {
                assert!(
                    ordinary::compile(compilation(&structure, &grammar), Profile::NativeV80)
                        .is_err()
                );
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn native_v82_unordered_modifiers_enforce_membership_and_preserve_authored_order() {
    let structure = r#"rcore 1; package modifiers version "1"; metamodel "urn:general-text" prefix m {
      class Modifier { attr static: Bool [0,1]; attr final: Bool [0,1]; attr visibility: String [1,1]; }
    }"#;
    let grammar = r#"grammar independent.Modifiers with org.eclipse.xtext.common.Terminals
      import "urn:general-text" as m
      Modifier returns m::Modifier: static?='static'? & final?='final'? & visibility=('public'|'private'|'protected');"#;
    for (text, has_static, has_final, visibility) in [
        ("public static final", true, true, "public"),
        ("static protected", true, false, "protected"),
        ("final private static", true, true, "private"),
        ("public", false, false, "public"),
    ] {
        let session = SourceEditSession::new(
            language_with_profile(structure, grammar, Profile::NativeV82),
            &[Source {
                uri: "modifiers.test".into(),
                text: text.into(),
            }],
            EvaluationLimits::workbench(),
        )
        .unwrap();
        let values = &session.model().elements[0].values;
        assert_eq!(values["static"], json!(has_static));
        assert_eq!(values["final"], json!(has_final));
        assert_eq!(values["visibility"], json!(visibility));
        CanonicalSerializer::new(
            language_with_profile(structure, grammar, Profile::NativeV82),
            EvaluationLimits::workbench(),
            Default::default(),
        )
        .serialize(session.model())
        .unwrap();
    }
    for text in [
        "static final static",
        "public static final private",
        "final",
    ] {
        assert!(
            SourceEditSession::new(
                language_with_profile(structure, grammar, Profile::NativeV82),
                &[Source {
                    uri: "invalid.test".into(),
                    text: text.into()
                }],
                EvaluationLimits::workbench()
            )
            .is_err(),
            "{text}"
        );
    }
    let original = "private /* retain */ static final\r\n";
    let mut session = SourceEditSession::new(
        language_with_profile(structure, grammar, Profile::NativeV82),
        &[Source {
            uri: "modifiers.test".into(),
            text: original.into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let root = session.model().elements[0].id.clone();
    let checkpoint = session.checkpoint();
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[(root.clone(), "visibility".into(), json!("public"))],
        )
        .unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(
        proposal.sources()[0].text,
        original.replace("private", "public")
    );
    session.adopt(proposal).unwrap();
    candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[(root.clone(), "static".into(), json!(false))],
        )
        .unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(proposal.sources()[0].text.matches("retain").count(), 1);
    session.adopt(proposal).unwrap();
    assert_eq!(session.model().elements[0].id, root);
    session
        .adopt(session.plan_restore(&checkpoint).unwrap())
        .unwrap();
    assert_eq!(session.sources()[0].text, original);
    assert!(ordinary::compile(compilation(structure, grammar), Profile::NativeV81).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn native_v82_unordered_repetitions_are_contiguous_and_empty_entries_survive() {
    let structure = r#"rcore 1; package repetitions version "1"; metamodel "urn:general-text" prefix r {
      class Root { attr values: Int32 [0,*] nonunique; attr name: String [1,1]; }
    }"#;
    for cardinality in ['*', '+'] {
        let grammar = format!(
            r#"grammar independent.Repeated with org.eclipse.xtext.common.Terminals
            import "urn:general-text" as r
            Root returns r::Root: values+=INT{cardinality} & name=ID;"#
        );
        for text in ["0 8 15 x", "x 0 8 15"] {
            let session = SourceEditSession::new(
                language_with_profile(structure, &grammar, Profile::NativeV82),
                &[Source {
                    uri: "repeat.test".into(),
                    text: text.into(),
                }],
                EvaluationLimits::workbench(),
            )
            .unwrap();
            assert_eq!(
                session.model().elements[0].values["values"],
                json!([0, 8, 15])
            );
            assert_eq!(session.model().elements[0].values["name"], json!("x"));
            CanonicalSerializer::new(
                language_with_profile(structure, &grammar, Profile::NativeV82),
                EvaluationLimits::workbench(),
                Default::default(),
            )
            .serialize(session.model())
            .unwrap();
        }
        for text in ["0 x 8 15", "x y", "0 8 15"] {
            assert!(
                SourceEditSession::new(
                    language_with_profile(structure, &grammar, Profile::NativeV82),
                    &[Source {
                        uri: "repeat.test".into(),
                        text: text.into()
                    }],
                    EvaluationLimits::workbench()
                )
                .is_err(),
                "{text}"
            );
        }
        let empty = SourceEditSession::new(
            language_with_profile(structure, &grammar, Profile::NativeV82),
            &[Source {
                uri: "repeat.test".into(),
                text: "x".into(),
            }],
            EvaluationLimits::workbench(),
        );
        assert_eq!(empty.is_ok(), cardinality == '*');
    }
    let structure = r#"rcore 1; package flags version "1"; metamodel "urn:general-text" prefix f { class Flags { attr a: Bool [0,1]; attr b: Bool [0,1]; } }"#;
    let grammar = r#"grammar independent.Flags with org.eclipse.xtext.common.Terminals
        import "urn:general-text" as f
        Flags returns f::Flags: a?='a'? & b?='b'?;"#;
    let session = SourceEditSession::new(
        language_with_profile(structure, grammar, Profile::NativeV82),
        &[Source {
            uri: "empty.test".into(),
            text: "/* empty flags */\r\n".into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    assert!(session.model().elements.is_empty());
    let generated = CanonicalSerializer::new(
        language_with_profile(structure, grammar, Profile::NativeV82),
        EvaluationLimits::workbench(),
        Default::default(),
    )
    .serialize(session.model())
    .unwrap();
    assert!(generated.identities.is_empty());
    assert_eq!(generated.sources.len(), 1);
    assert_eq!(
        session
            .plan(session.model(), Default::default())
            .unwrap()
            .sources()[0]
            .text,
        "/* empty flags */\r\n"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn native_v82_nested_unordered_groups_preserve_inner_members_as_a_unit() {
    let structure = r#"rcore 1; package nested version "1"; metamodel "urn:general-text" prefix n {
        class Root { attr x: Bool [0,1]; attr y: Bool [0,1]; attr name: String [1,1]; }
    }"#;
    let grammar = r#"grammar independent.Nested with org.eclipse.xtext.common.Terminals
        import "urn:general-text" as n
        Root returns n::Root: ('begin' (x?='x'? & y?='y'?) 'end') & name=ID;"#;
    for text in [
        "begin end N",
        "N begin x y end",
        "begin y x end N",
        "N begin y end",
    ] {
        let session = SourceEditSession::new(
            language_with_profile(structure, grammar, Profile::NativeV82),
            &[Source {
                uri: "nested.test".into(),
                text: text.into(),
            }],
            EvaluationLimits::workbench(),
        )
        .unwrap();
        assert_eq!(session.model().elements[0].values["name"], json!("N"));
        CanonicalSerializer::new(
            language_with_profile(structure, grammar, Profile::NativeV82),
            EvaluationLimits::workbench(),
            Default::default(),
        )
        .serialize(session.model())
        .unwrap();
    }
    assert!(
        SourceEditSession::new(
            language_with_profile(structure, grammar, Profile::NativeV82),
            &[Source {
                uri: "nested.test".into(),
                text: "begin x N y end".into()
            }],
            EvaluationLimits::workbench()
        )
        .is_err()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn native_v82_nullable_unordered_members_skip_empty_paths_without_reordering_groups() {
    let structure = r#"rcore 1; package nullable version "1"; metamodel "urn:general-text" prefix n {
      class Root { attr x: Bool [0,1]; attr y: Bool [0,1]; attr name: String [1,1]; }
    }"#;
    for member in ["(x?='x'? & y?='y'?)", "(x?='x'? y?='y'?)"] {
        let grammar = format!(
            r#"grammar independent.Nullable with org.eclipse.xtext.common.Terminals
            import "urn:general-text" as n
            Root returns n::Root: {member} & name=ID;"#
        );
        for text in ["N", "N x y", "x y N", "N y"] {
            let session = SourceEditSession::new(
                language_with_profile(structure, &grammar, Profile::NativeV82),
                &[Source {
                    uri: "nullable.test".into(),
                    text: text.into(),
                }],
                EvaluationLimits::workbench(),
            )
            .unwrap();
            CanonicalSerializer::new(
                language_with_profile(structure, &grammar, Profile::NativeV82),
                EvaluationLimits::workbench(),
                Default::default(),
            )
            .serialize(session.model())
            .unwrap();
        }
        assert!(
            SourceEditSession::new(
                language_with_profile(structure, &grammar, Profile::NativeV82),
                &[Source {
                    uri: "nullable.test".into(),
                    text: "x N y".into()
                }],
                EvaluationLimits::workbench()
            )
            .is_err()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn native_v82_unordered_containment_references_and_limits_use_shared_services() {
    let structure = r#"rcore 1; package nodes version "1"; metamodel "urn:general-text" prefix n {
      class Root { ref nodes: Node [0,*] containment; ref selected: Node [1,1]; }
      class Node { attr name: String [1,1]; }
    }"#;
    let grammar = r#"grammar independent.Nodes with org.eclipse.xtext.common.Terminals
      import "urn:general-text" as n
      Root returns n::Root: ('nodes' nodes+=Node*) & ('select' selected=[n::Node]);
      Node returns n::Node: 'node' name=ID ';';"#;
    let original = "select A /* reference */ nodes node A; node B;\r\n";
    let mut session = SourceEditSession::new(
        language_with_profile(structure, grammar, Profile::NativeV82),
        &[Source {
            uri: "nodes.test".into(),
            text: original.into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    let checkpoint = session.checkpoint();
    let node = named(&session, "A");
    let mut candidate = session.model().clone();
    candidate
        .edit(
            &session.package().schema,
            &[(node.clone(), "name".into(), json!("Renamed"))],
        )
        .unwrap();
    let proposal = session.plan(&candidate, Default::default()).unwrap();
    assert_eq!(proposal.sources()[0].text, original.replace('A', "Renamed"));
    session.adopt(proposal).unwrap();
    assert_eq!(named(&session, "Renamed"), node);
    CanonicalSerializer::new(
        language_with_profile(structure, grammar, Profile::NativeV82),
        EvaluationLimits::workbench(),
        Default::default(),
    )
    .serialize(session.model())
    .unwrap();
    candidate
        .edit(
            &session.package().schema,
            &[(node.clone(), "name".into(), json!("Other"))],
        )
        .unwrap();
    assert!(
        session
            .plan(
                &candidate,
                SerializationLimits {
                    steps: 1,
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(session.sources()[0].text, original.replace('A', "Renamed"));
    session
        .adopt(session.plan_restore(&checkpoint).unwrap())
        .unwrap();
    assert_eq!(session.sources()[0].text, original);
    let large = r#"grammar independent.Large with org.eclipse.xtext.common.Terminals
      import "urn:general-text" as n
      Root returns n::Root: 'a' & 'b' & 'c' & 'd' & 'e' & 'f' & 'g' & 'h';"#;
    let error = ordinary::compile(compilation(structure, large), Profile::NativeV82).unwrap_err();
    assert!(
        error
            .iter()
            .any(|d| d.message.contains("Unordered generated declaration budget")),
        "{error:?}"
    );
    for body in ["{n::Root} & 'x'", "Node & 'x'", "=>(('a' & 'b'))"] {
        let invalid = format!(
            r#"grammar independent.Invalid with org.eclipse.xtext.common.Terminals
            import "urn:general-text" as n
            Root returns n::Root: {body}; Node returns n::Node: name=ID;"#
        );
        assert!(
            ordinary::compile(compilation(structure, &invalid), Profile::NativeV82).is_err(),
            "{body}"
        );
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn native_v82_unordered_inferred_metamodel_replays_original_source() {
    let structure = r#"rcore 1; package anchor version "1"; metamodel "urn:general-text" prefix a { class Anchor {} }"#;
    let grammar = r#"grammar independent.Inferred with org.eclipse.xtext.common.Terminals
        generate inferred "urn:unordered:inferred"
        Root: enabled?='enabled'? & name=ID;"#;
    let inputs = compilation(structure, grammar);
    let inferred = intersect_text::inference::InferenceRequest {
        version: 1,
        grammar: inputs.xtext.clone().unwrap(),
        bundle: inputs.bundle,
    }
    .compile_with_selected_profile(
        Default::default(),
        Some(Profile::NativeV82),
        &[],
        None,
        None,
    )
    .unwrap();
    let prepared = DocumentLanguage::prepare(
        inferred.package(),
        Default::default(),
        AnalysisLimits::workbench(),
    )
    .unwrap();
    let session = SourceEditSession::new(
        prepared,
        &[Source {
            uri: "inferred.test".into(),
            text: "Name enabled".into(),
        }],
        EvaluationLimits::workbench(),
    )
    .unwrap();
    assert_eq!(session.model().elements[0].values["name"], json!("Name"));
    CanonicalSerializer::new(
        DocumentLanguage::prepare(
            inferred.package(),
            Default::default(),
            AnalysisLimits::workbench(),
        )
        .unwrap(),
        EvaluationLimits::workbench(),
        Default::default(),
    )
    .serialize(session.model())
    .unwrap();
}
