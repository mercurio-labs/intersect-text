//! Imported-feature compatibility is separate from inferred-feature creation defaults.
use intersect_model::definitions::bundle::SourceId;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
use serde_json::{Value, json};
fn call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap()
}
// The frozen JSON/WASM carrier wraps dispatch failures; native Result alone does not.
fn abi_call(request: Value) -> Value {
    intersect_web::dispatch(&request.to_string()).unwrap_or_else(|message| json!({"error":message}))
}
fn before(profile: u8) -> Value {
    serde_json::from_str(if profile == 64 {
        include_str!("../../../fixtures/xtext/imported-assignment-v66/native-before-v64.json")
    } else {
        include_str!("../../../fixtures/xtext/imported-assignment-v66/native-before-v65.json")
    })
    .unwrap()
}
const MIGRATED_RCORE: &str = "<migrated Rcore text and its identity>";
/// 203902dd and eeb500c4 changed the Ecore-to-Rcore writer (local classifiers
/// by name, names quoted only where needed), so an `ecore/migrate` response no
/// longer reproduces the sealed transcripts' migrated text. For those
/// responses only, the fields derived from that text are first checked for
/// internal consistency and then replaced by a marker on both sides:
/// `report.draft`, `report.converted.text`, `report.converted.id.sha256`, and
/// each correspondence's `converted.source.sha256` and `converted_span`
/// (digest, start, end). Every other field, and every later call (which
/// carries the sealed migrated text), is compared exactly.
fn normalize_migration(request: &Value, mut result: Value) -> Value {
    if request["method"] != "ecore/migrate" {
        return result;
    }
    let report = &mut result["report"];
    let text = report["converted"]["text"].as_str().unwrap().to_owned();
    assert_eq!(report["draft"], json!(text));
    let uri = report["converted"]["id"]["uri"].as_str().unwrap();
    let digest = SourceId::for_text(uri, &text).sha256;
    assert_eq!(report["converted"]["id"]["sha256"], json!(digest));
    report["draft"] = json!(MIGRATED_RCORE);
    report["converted"]["text"] = json!(MIGRATED_RCORE);
    report["converted"]["id"]["sha256"] = json!(MIGRATED_RCORE);
    for entry in report["correspondence"].as_array_mut().unwrap() {
        assert_eq!(entry["converted"]["source"]["sha256"], json!(digest));
        entry["converted"]["source"]["sha256"] = json!(MIGRATED_RCORE);
        let span = &mut entry["converted_span"];
        assert_eq!(span["source"]["sha256"], json!(digest));
        let (start, end) = (
            span["start"].as_u64().unwrap() as usize,
            span["end"].as_u64().unwrap() as usize,
        );
        assert!(start <= end && text.get(start..end).is_some(), "{span}");
        span["source"]["sha256"] = json!(MIGRATED_RCORE);
        span["start"] = json!(MIGRATED_RCORE);
        span["end"] = json!(MIGRATED_RCORE);
    }
    result
}
fn replay_exact_except_migrated_rcore(profile: u8) {
    let original = before(profile);
    assert_eq!(original["transcript"].as_array().unwrap().len(), 22);
    for row in original["transcript"].as_array().unwrap() {
        let request = &row["request"];
        assert_eq!(
            normalize_migration(request, abi_call(request.clone())),
            normalize_migration(request, row["result"].clone())
        );
    }
}
#[test]
fn v64_complete_original_source_migration_archive_and_model_responses_stay_exact() {
    replay_exact_except_migrated_rcore(64);
}
#[test]
fn v65_complete_original_source_migration_archive_and_model_responses_stay_exact() {
    replay_exact_except_migrated_rcore(65);
}
#[test]
fn v66_preserves_imported_unique_flags_and_exact_source_archive_runtime_authority() {
    let original = before(64);
    for (i, c) in original["cases"].as_array().unwrap().iter().enumerate() {
        let compilation = c["selection"]["compilation"].clone();
        let compiled = call(
            json!({"method":"compile_bundle","compilation":compilation,"xtext_profile":"intersect.xtext.native.v66"}),
        );
        assert_eq!(compiled["ok"], true, "{compiled}");
        assert_eq!(
            compiled["package"]["schema"]["classes"]["Model"]["features"]["refs"]["unique"],
            i != 2
        );
        assert_eq!(
            compiled["package"]["schema"]["classes"]["Model"]["features"]["nodes"]["unique"],
            true
        );
        assert_eq!(
            compiled["package"]["compilation"]["bundle"],
            compilation["bundle"]
        );
        let exported = call(
            json!({"method":"package/export","compilation":compilation,"xtext_profile":"intersect.xtext.native.v66","metadata":{"package_id":format!("v66.{}",c["id"].as_str().unwrap()),"release":"1","display_name":"Imported unique compatibility","description":"Source-preserving new-profile control"}}),
        );
        assert!(exported["manifest"].is_object(), "{exported}");
        let mut files = exported["files"].as_object().unwrap().clone();
        files.insert(
            "manifest.json".into(),
            Value::String(exported["manifest"].to_string()),
        );
        let restored = call(json!({"method":"package/import","files":files}));
        assert!(restored["package"].is_object(), "{restored}");
        assert_eq!(restored["package"], compiled["package"]);
        for (j, input) in c["inputs"].as_array().unwrap().iter().enumerate() {
            let sources = json!([{"uri":format!("memory:{}/model.unique",c["id"].as_str().unwrap()),"text":input["text"]}]);
            let direct =
                call(json!({"method":"evaluate","package":compiled["package"],"sources":sources}));
            let replay =
                call(json!({"method":"evaluate","package":restored["package"],"sources":sources}));
            assert_eq!(direct, replay);
            if j == 0 || j == 3 {
                assert_eq!(direct["diagnostics"], json!([]), "{direct}");
            }
            if j >= 4 {
                assert!(
                    !direct["diagnostics"].as_array().unwrap().is_empty(),
                    "{direct}"
                );
            }
            if j < 4 {
                let prefix = format!("memory:{}/model.unique#", c["id"].as_str().unwrap());
                let expected = match j {
                    0 => vec![format!("{prefix}1"), format!("{prefix}2")],
                    1 => vec![format!("{prefix}1"), format!("{prefix}1")],
                    2 => vec![
                        format!("{prefix}2"),
                        format!("{prefix}1"),
                        format!("{prefix}2"),
                    ],
                    _ => vec![format!("{prefix}1")],
                };
                // Resolution preserves the authored reference sequence, including
                // duplicates. This is not a general list-mutation equivalence claim.
                assert_eq!(
                    direct["model"]["elements"][0]["values"]["refs"],
                    json!(expected)
                );
            }
            println!(
                "V66_OBSERVATION {}",
                json!({"id":c["id"],"text":input["text"],"result":direct})
            );
        }
    }
}

#[test]
fn v66_independent_imported_shape_and_mutability_negatives_reject_without_source_repair() {
    let original: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/imported-assignment-v66/native-negative-before-v64.json"
    ))
    .unwrap();
    assert_eq!(original["cases"].as_array().unwrap().len(), 6);
    for c in original["cases"].as_array().unwrap() {
        if c["selection"].is_null() {
            assert_eq!(c["migration"]["ok"], false);
            assert!(c["migration"]["report"]["converted"].is_null());
            continue;
        }
        let result = call(
            json!({"method":"compile_bundle","compilation":c["selection"]["compilation"],"xtext_profile":"intersect.xtext.native.v66"}),
        );
        assert_eq!(result["ok"], false, "{result}");
        assert!(result["package"].is_null());
        assert_eq!(
            result["diagnostics"][0]["source"],
            c["selection"]["compilation"]["xtext"]["uri"]
        );
        assert_eq!(
            result["diagnostics"][0]["message"],
            "Grammar assignment conflicts with the imported feature"
        );
    }
}

#[test]
fn v66_preserves_source_selected_computed_projection_and_parameterized_fresh_validation() {
    use intersect_model::definitions::bundle::DefinitionBundle;
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::conversion::ProviderId;
    use intersect_model::definitions::setting::ComputedSettingContract;
    use intersect_model::definitions::setting::SettingOperation;
    use intersect_text::grammar::Profile;
    let plan: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext/inferred-computed-collection-contexts/plan.json"
    ))
    .unwrap();
    for (parameterized, row) in plan["cases"].as_array().unwrap().iter().enumerate() {
        let mut bundle: DefinitionBundle =
            serde_json::from_value(row["inference"]["bundle"].clone()).unwrap();
        let structure = bundle
            .sources
            .iter()
            .find(|s| s.kind == SourceKind::Rcore)
            .unwrap()
            .id
            .clone();
        let behavior = bundle
            .sources
            .iter()
            .find(|s| s.kind == SourceKind::RcoreBehavior)
            .unwrap()
            .id
            .clone();
        bundle.sources.retain(|s| s.kind != SourceKind::Xtext);
        let body = if parameterized == 0 {
            "Model returns p::Item: computed+=ID (',' computed+=ID)*;\n"
        } else {
            "Model returns p::Item: Item<true>;\nItem<Flag> returns p::Item: <Flag> computed+=ID (',' computed+=ID)* | <!Flag> backing+=ID (',' backing+=ID)*;\n"
        };
        let mut grammar = DefinitionSource::new(
            format!("memory:v66-provider/{parameterized}.xtext"),
            SourceKind::Xtext,
            format!(
                "grammar contexts.Ordinary{parameterized} hidden(WS)\nimport \"urn:inferred:collections\" as p\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\n{body}terminal ID returns ecore::EString: ('a'..'z')+;\nterminal WS returns ecore::EString: (' '|'\\t'|'\\r'|'\\n')+;\n"
            ),
        );
        grammar.dependencies = vec![structure.clone(), behavior.clone()];
        let entry = grammar.id.clone();
        bundle.sources.push(grammar);
        let mut compilation = BundleCompilation {
            bundle,
            ecore: structure,
            namespace: "urn:inferred:collections".into(),
            xtext: None,
            rcore_behavior: behavior,
            materialized_types: vec![],
        };
        let mut structural =
            intersect_text::ordinary::compile(compilation.clone(), Profile::NativeV66).unwrap();
        let before_reference_revision = structural.schema.revision.clone();
        intersect_text::grammar::prepare_ecore_reference_context(&mut structural.schema).unwrap();
        assert_ne!(before_reference_revision, structural.schema.revision);
        let class = structural
            .schema
            .declarations
            .iter()
            .find(|(_, d)| d.namespace == "urn:inferred:collections" && d.name == "Item")
            .unwrap()
            .0
            .clone();
        let contract = ComputedSettingContract {
            version: 1,
            schema_revision: structural.schema.revision.clone(),
            class_identity: structural.schema.identity(&class),
            member: structural
                .schema
                .effective_feature(&class, "computed")
                .unwrap()
                .identity,
            class,
            feature: "computed".into(),
            provider: ProviderId {
                name: "rcore.projection".into(),
                version: "1".into(),
            },
            sources: compilation
                .bundle
                .sources
                .iter()
                .map(|s| s.id.clone())
                .collect(),
            operations: vec![
                SettingOperation::Set,
                SettingOperation::Append,
                SettingOperation::Unset,
            ],
        };
        compilation.xtext = Some(entry);
        let mut stale = contract.clone();
        stale.schema_revision = before_reference_revision;
        for profile in [Profile::NativeV65, Profile::NativeV66] {
            let error = intersect_text::ordinary::compile_with_context(
                compilation.clone(),
                profile,
                std::slice::from_ref(&stale),
                None,
                Default::default(),
                Default::default(),
            )
            .unwrap_err();
            assert_eq!(
                error[0].message,
                "Invalid or stale computed setting contract"
            );
        }
        assert!(
            intersect_text::ordinary::compile(compilation.clone(), Profile::NativeV66).is_err()
        );
        let old_package = intersect_text::ordinary::compile_with_context(
            compilation.clone(),
            Profile::NativeV65,
            std::slice::from_ref(&contract),
            None,
            Default::default(),
            Default::default(),
        )
        .unwrap();
        let package = intersect_text::ordinary::compile_with_context(
            compilation.clone(),
            Profile::NativeV66,
            std::slice::from_ref(&contract),
            None,
            CompilationSourcePolicy::default(),
            CompilationSourcePolicy::default(),
        )
        .unwrap();
        assert_eq!(old_package.schema, package.schema);
        assert_eq!(
            old_package.grammar.as_ref().unwrap().computed_settings,
            package.grammar.as_ref().unwrap().computed_settings
        );
        assert_eq!(
            package.grammar.as_ref().unwrap().computed_settings,
            vec![contract.clone()]
        );
        assert_eq!(
            package.compilation.as_ref().unwrap().bundle,
            compilation.bundle
        );
        let sources = json!([{"uri":"memory:v66-computed.model","text":"alpha,beta"}]);
        let old_json =
            abi_call(json!({"method":"evaluate","package":old_package,"sources":sources}));
        let new_json = abi_call(json!({"method":"evaluate","package":package,"sources":sources}));
        assert_eq!(old_json, new_json);
        assert!(
            new_json["error"]
                .as_str()
                .unwrap()
                .contains("Cannot assign derived feature")
        );
        // Ordinary JSON's context-free reconstruction remains a separate host
        // boundary. The explicit native context reconstructs and installs the
        // real source-backed provider through the existing shared APIs.
        let package = intersect_text::recompile_with_context(
            &package,
            std::slice::from_ref(&contract),
            None,
            Default::default(),
        )
        .unwrap();
        use intersect_text::setting::{RcoreProjection, SourceSettingValues, TextSettings};
        let bundle = &package.compilation.as_ref().unwrap().bundle;
        let provider = RcoreProjection::new(&package, bundle, &contract).unwrap();
        let providers: [&dyn SourceSettingValues; 1] = [&provider];
        let settings = TextSettings::new(&package, bundle, &providers).unwrap();
        let model_sources = serde_json::from_value::<Vec<intersect_text::Source>>(sources).unwrap();
        let result = serde_json::to_value(intersect_text::evaluate_with_context(
            &package,
            &model_sources,
            intersect_text::EvaluationLimits::workbench(),
            None,
            Some(&settings),
            &intersect_model::query::PackageDerived(&package),
        ))
        .unwrap();
        assert_eq!(result["diagnostics"], json!([]), "{result}");
        let objects = result["model"]["elements"].as_array().unwrap();
        assert_eq!(objects.len(), 1, "{result}");
        assert_eq!(objects[0]["values"], json!({"backing":["alpha","beta"]}));
        assert_eq!(
            result["derived"][objects[0]["id"].as_str().unwrap()]["computed"],
            json!(["alpha", "beta"])
        );
        let mut missing = contract;
        missing
            .operations
            .retain(|op| *op != SettingOperation::Append);
        assert!(
            intersect_text::ordinary::compile_with_context(
                compilation,
                Profile::NativeV66,
                &[missing],
                None,
                Default::default(),
                Default::default()
            )
            .is_err()
        );
    }
}

#[test]
fn v66_inherits_every_v65_boolean_capability_without_widening_earlier_profiles() {
    use intersect_text::grammar::Profile;
    let a = Profile::NativeV65;
    let b = Profile::NativeV66;
    assert!(!a.has_imported_assignment_compatibility());
    assert!(b.has_imported_assignment_compatibility());
    assert_eq!(
        a.has_durable_documents(),
        b.has_durable_documents(),
        "has_durable_documents"
    );
    assert_eq!(
        a.has_scoped_hidden_tokens(),
        b.has_scoped_hidden_tokens(),
        "has_scoped_hidden_tokens"
    );
    assert_eq!(
        a.has_terminal_cardinality_decisions(),
        b.has_terminal_cardinality_decisions(),
        "has_terminal_cardinality_decisions"
    );
    assert_eq!(
        a.has_contextual_first_rule(),
        b.has_contextual_first_rule(),
        "has_contextual_first_rule"
    );
    assert_eq!(
        a.has_projected_terminal_negation(),
        b.has_projected_terminal_negation(),
        "has_projected_terminal_negation"
    );
    assert_eq!(
        a.has_until_operands(),
        b.has_until_operands(),
        "has_until_operands"
    );
    assert_eq!(
        a.has_multi_unit_literal_negation(),
        b.has_multi_unit_literal_negation(),
        "has_multi_unit_literal_negation"
    );
    assert_eq!(
        a.has_terminal_operand_negation(),
        b.has_terminal_operand_negation(),
        "has_terminal_operand_negation"
    );
    assert_eq!(
        a.has_constant_guards(),
        b.has_constant_guards(),
        "has_constant_guards"
    );
    assert_eq!(
        a.has_lexical_prediction(),
        b.has_lexical_prediction(),
        "has_lexical_prediction"
    );
    assert_eq!(
        a.has_parser_overrides(),
        b.has_parser_overrides(),
        "has_parser_overrides"
    );
    assert_eq!(
        a.has_return_inference(),
        b.has_return_inference(),
        "has_return_inference"
    );
    assert_eq!(
        a.has_current_actions(),
        b.has_current_actions(),
        "has_current_actions"
    );
    assert_eq!(
        a.has_parser_prediction(),
        b.has_parser_prediction(),
        "has_parser_prediction"
    );
    assert_eq!(a.has_enum_rules(), b.has_enum_rules(), "has_enum_rules");
    assert_eq!(
        a.has_rule_conversions(),
        b.has_rule_conversions(),
        "has_rule_conversions"
    );
    assert_eq!(
        a.has_recursive_predicates(),
        b.has_recursive_predicates(),
        "has_recursive_predicates"
    );
    assert_eq!(
        a.has_double_rules(),
        b.has_double_rules(),
        "has_double_rules"
    );
    assert_eq!(
        a.has_recursive_fragments(),
        b.has_recursive_fragments(),
        "has_recursive_fragments"
    );
    assert_eq!(
        a.has_computed_settings(),
        b.has_computed_settings(),
        "has_computed_settings"
    );
    assert_eq!(
        a.has_terminal_decisions(),
        b.has_terminal_decisions(),
        "has_terminal_decisions"
    );
    assert_eq!(
        a.has_validated_syntax(),
        b.has_validated_syntax(),
        "has_validated_syntax"
    );
    assert_eq!(
        a.has_terminal_overrides(),
        b.has_terminal_overrides(),
        "has_terminal_overrides"
    );
    assert_eq!(
        a.has_explicit_terminal_calls(),
        b.has_explicit_terminal_calls(),
        "has_explicit_terminal_calls"
    );
    assert_eq!(
        a.has_explicit_parser_calls(),
        b.has_explicit_parser_calls(),
        "has_explicit_parser_calls"
    );
    assert_eq!(
        a.has_typed_overrides(),
        b.has_typed_overrides(),
        "has_typed_overrides"
    );
    assert_eq!(
        serde_json::to_value(b).unwrap(),
        "intersect.xtext.native.v66"
    );
    assert_eq!(b.as_str(), "intersect.xtext.native.v66");
}
