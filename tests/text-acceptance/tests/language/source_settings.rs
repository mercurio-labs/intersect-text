//! Authored provider admission controls; no Xtext admission or KerML claim.
use intersect_model::core::{
    frozen::FrozenStore,
    query::{DerivedValues, QueryBudget},
    storage::{self, ComputedEdit, ModelRead, ModelStore, SettingEdit, SettingValues},
};
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    conversion::ProviderId,
    schema::Schema,
    setting::{ComputedSettingContract, SettingOperation},
};
use intersect_text::compiler;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
use intersect_text::setting::{SourceBoundSettings, SourceSettingValues};
use serde_json::{Value, json};
use std::cell::Cell;
const ECORE: &str = r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="bound" nsURI="urn:test:bound">
<eClassifiers x:type="e:EClass" name="Root">
<eStructuralFeatures x:type="e:EAttribute" name="computed" derived="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures x:type="e:EAttribute" name="backing" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
</eClassifiers></e:EPackage>"##;
fn fixture() -> (
    Schema,
    DefinitionBundle,
    ComputedSettingContract,
    FrozenStore,
) {
    fixture_for(ECORE, "Root")
}
fn fixture_for(
    text: &str,
    class: &str,
) -> (
    Schema,
    DefinitionBundle,
    ComputedSettingContract,
    FrozenStore,
) {
    let source = DefinitionSource::new("memory:ecore", SourceKind::Ecore, text);
    let (source, _) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![intersect_model::definitions::bundle::NamespaceBinding {
            namespace: "urn:test:bound".into(),
            source: source.id.clone(),
        }],
    };
    let schema = intersect_model::definitions::compiler::compile_structure_bundle(
        &bundle,
        &source.id,
        "urn:test:bound",
    )
    .unwrap();
    let contract = ComputedSettingContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class: class.into(),
        class_identity: schema.identity(class),
        feature: "computed".into(),
        member: schema
            .effective_feature("Root", "computed")
            .unwrap()
            .identity,
        provider: ProviderId {
            name: "test.bound".into(),
            version: "1".into(),
        },
        sources: vec![source.id.clone()],
        operations: vec![SettingOperation::Set],
    };
    let mut store = FrozenStore::for_schema(&schema).unwrap();
    storage::atomic(&mut store, |s| {
        s.create_element(&schema, "root", class, "model", 0, 0)
    })
    .unwrap();
    (schema, bundle, contract, store)
}
struct Provider {
    expected: ComputedSettingContract,
    changed: ComputedSettingContract,
    drift: &'static str,
    changed_now: Cell<bool>,
    calls: Cell<usize>,
}
impl Provider {
    fn new(contract: &ComputedSettingContract, drift: &'static str) -> Self {
        let mut changed = contract.clone();
        changed.provider.version = "forged".into();
        Self {
            expected: contract.clone(),
            changed,
            drift,
            changed_now: Cell::new(false),
            calls: Cell::new(0),
        }
    }
}
impl SourceSettingValues for Provider {
    fn contract(&self) -> &ComputedSettingContract {
        if self.changed_now.get() {
            &self.changed
        } else {
            &self.expected
        }
    }
}
impl DerivedValues for Provider {
    fn schema_revision(&self) -> Option<&str> {
        Some(&self.expected.schema_revision)
    }
    fn supports(&self, class: &str, feature: &str) -> bool {
        class == self.expected.class && feature == self.expected.feature
    }
    fn read(
        &self,
        store: &dyn ModelRead,
        id: &str,
        _: &str,
        budget: &mut QueryBudget,
    ) -> Result<Value, String> {
        budget.charge()?;
        if self.drift == "read" {
            self.changed_now.set(true);
        }
        store
            .value(id, "backing")?
            .ok_or("Missing backing value".into())
    }
}
impl SettingValues for Provider {
    fn plan(
        &self,
        _: &dyn ModelRead,
        id: &str,
        _: &str,
        edit: &ComputedEdit,
        budget: &mut QueryBudget,
    ) -> Result<Vec<SettingEdit>, String> {
        self.calls.set(self.calls.get() + 1);
        budget.charge()?;
        let ComputedEdit::Set(value) = edit else {
            return Err("Unsupported operation".into());
        };
        if self.drift == "plan" {
            self.changed_now.set(true);
        }
        Ok(vec![SettingEdit::Set {
            id: id.into(),
            feature: "backing".into(),
            value: value.clone(),
        }])
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn exact_contract_executes_and_every_budget_cutoff_rolls_back() {
    let (schema, bundle, contract, mut store) = fixture();
    let decoded: ComputedSettingContract =
        serde_json::from_value(serde_json::to_value(&contract).unwrap()).unwrap();
    assert_eq!(decoded, contract);
    let provider = Provider::new(&contract, "none");
    let service = SourceBoundSettings {
        schema: &schema,
        bundle: &bundle,
        expected: &decoded,
        provider: &provider,
        conversions: None,
    };
    let before = storage::export(&store).unwrap();
    let outer = store.begin().unwrap();
    let mut budget = QueryBudget::new(100);
    service
        .apply(
            &mut store,
            "root",
            ComputedEdit::Set(json!("hello")),
            &mut budget,
        )
        .unwrap();
    assert_eq!(
        store.value("root", "backing").unwrap(),
        Some(json!("hello"))
    );
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), before);
    for limit in 0..budget.reads {
        assert!(
            service
                .apply(
                    &mut store,
                    "root",
                    ComputedEdit::Set(json!("hello")),
                    &mut QueryBudget::new(limit)
                )
                .unwrap_err()
                .contains("budget")
        );
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    service
        .apply(
            &mut store,
            "root",
            ComputedEdit::Set(json!("hello")),
            &mut QueryBudget::new(budget.reads),
        )
        .unwrap();
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn stale_forged_and_incomplete_contracts_never_invoke_provider() {
    let (schema, bundle, contract, mut store) = fixture();
    let before = storage::export(&store).unwrap();
    for change in 0..13 {
        let mut expected = contract.clone();
        let mut sources = bundle.clone();
        match change {
            0 => expected.schema_revision.push('x'),
            1 => expected.class_identity.push('x'),
            2 => {
                expected.member = schema
                    .effective_feature("Root", "backing")
                    .unwrap()
                    .identity
            }
            3 => expected.provider.version = "2".into(),
            4 => expected.sources.clear(),
            5 => expected.sources.push(expected.sources[0].clone()),
            6 => sources.sources[0].text.push(' '),
            7 => sources.sources[0].id.uri = "other:ecore".into(),
            8 => expected.operations.clear(),
            9 => expected.operations.push(SettingOperation::Set),
            10 => expected.operations = vec![SettingOperation::Append],
            11 => sources.sources[0].kind = SourceKind::Xtext,
            12 => expected.feature = "backing".into(),
            _ => unreachable!(),
        }
        let provider = Provider::new(&contract, "none");
        assert!(
            SourceBoundSettings {
                schema: &schema,
                bundle: &sources,
                expected: &expected,
                provider: &provider,
                conversions: None
            }
            .apply(
                &mut store,
                "root",
                ComputedEdit::Set(json!("hello")),
                &mut QueryBudget::new(100)
            )
            .is_err(),
            "case {change}"
        );
        assert_eq!(provider.calls.get(), 0, "case {change}");
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    let provider = Provider::new(&contract, "none");
    let service = SourceBoundSettings {
        schema: &schema,
        bundle: &bundle,
        expected: &contract,
        provider: &provider,
        conversions: None,
    };
    assert!(
        service
            .apply(
                &mut store,
                "root",
                ComputedEdit::Unset,
                &mut QueryBudget::new(100)
            )
            .is_err()
    );
    assert!(
        service
            .apply(
                &mut store,
                "missing",
                ComputedEdit::Set(json!("hello")),
                &mut QueryBudget::new(100)
            )
            .is_err()
    );
    assert_eq!(provider.calls.get(), 0);
    let mut wire = serde_json::to_value(&contract).unwrap();
    wire["execute"] = json!(true);
    assert!(serde_json::from_value::<ComputedSettingContract>(wire).is_err());
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn changing_provider_contract_rolls_back_planning_and_readback() {
    let (schema, bundle, contract, mut store) = fixture();
    let before = storage::export(&store).unwrap();
    for drift in ["plan", "read"] {
        let provider = Provider::new(&contract, drift);
        let error = SourceBoundSettings {
            schema: &schema,
            bundle: &bundle,
            expected: &contract,
            provider: &provider,
            conversions: None,
        }
        .apply(
            &mut store,
            "root",
            ComputedEdit::Set(json!("hello")),
            &mut QueryBudget::new(100),
        )
        .unwrap_err();
        assert!(error.contains("contract mismatch"), "{error}");
        assert_eq!(provider.calls.get(), 1);
        assert_eq!(storage::export(&store).unwrap(), before);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn inherited_member_keeps_receiver_and_declaration_identities_distinct() {
    let text = ECORE.replace(
        "</e:EPackage>",
        r##"<eClassifiers x:type="e:EClass" name="Child" eSuperTypes="#//Root"/></e:EPackage>"##,
    );
    let (schema, bundle, contract, mut store) = fixture_for(&text, "Child");
    assert_eq!(
        contract.member,
        schema
            .effective_feature("Root", "computed")
            .unwrap()
            .identity
    );
    let provider = Provider::new(&contract, "none");
    SourceBoundSettings {
        schema: &schema,
        bundle: &bundle,
        expected: &contract,
        provider: &provider,
        conversions: None,
    }
    .apply(
        &mut store,
        "root",
        ComputedEdit::Set(json!("inherited")),
        &mut QueryBudget::new(100),
    )
    .unwrap();
    let before = storage::export(&store).unwrap();
    let mut parent = contract.clone();
    parent.class = "Root".into();
    parent.class_identity = schema.identity("Root");
    // A declaration identity shared through inheritance does not authorize another receiver contract.
    let parent_provider = Provider::new(&parent, "none");
    assert!(
        SourceBoundSettings {
            schema: &schema,
            bundle: &bundle,
            expected: &parent,
            provider: &parent_provider,
            conversions: None
        }
        .apply(
            &mut store,
            "root",
            ComputedEdit::Set(json!("wrong receiver")),
            &mut QueryBudget::new(100)
        )
        .is_err()
    );
    assert_eq!(parent_provider.calls.get(), 0);
    assert_eq!(storage::export(&store).unwrap(), before);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn grammar_admission_requires_exact_sources_and_keeps_execution_closed() {
    use intersect_text::grammar;
    use intersect_text::grammar::Profile;
    let (schema, mut bundle, mut contract, _) = fixture();
    let mut source = DefinitionSource::new(
        "memory:bound.xtext",
        SourceKind::Xtext,
        r#"grammar bound.Test hidden(WS) import "urn:test:bound" as b Root returns b::Root: 'root' computed=ID; terminal ID: ('a'..'z')+; terminal WS: ' '+;"#,
    );
    source.dependencies.push(bundle.sources[0].id.clone());
    bundle.sources.push(source.clone());
    contract.sources.push(source.id.clone());
    let admitted = grammar::compile_with_settings(
        &source,
        &schema,
        Profile::NativeV12,
        &bundle,
        &[contract.clone()],
    )
    .unwrap();
    assert_eq!(admitted.computed_settings, vec![contract.clone()]);
    assert!(
        admitted
            .declaration_sources
            .values()
            .all(|span| span.source == source.id)
    );
    assert!(grammar::compile_with_profile(&source.text, &schema, Profile::NativeV12).is_err());
    assert!(
        grammar::compile_with_settings(
            &source,
            &schema,
            Profile::NativeV11,
            &bundle,
            &[contract.clone()]
        )
        .is_err()
    );
    assert!(
        grammar::compile_with_settings(&source, &schema, Profile::NativeV12, &bundle, &[]).is_err()
    );
    assert!(
        grammar::compile_with_settings(
            &source,
            &schema,
            Profile::NativeV12,
            &bundle,
            &[contract.clone(), contract.clone()]
        )
        .is_err()
    );
    let mut omitted = contract.clone();
    omitted.sources.pop();
    assert!(
        grammar::compile_with_settings(&source, &schema, Profile::NativeV12, &bundle, &[omitted])
            .is_err()
    );
    let mut wrong_operation = contract.clone();
    wrong_operation.operations = vec![SettingOperation::Unset];
    assert!(
        grammar::compile_with_settings(
            &source,
            &schema,
            Profile::NativeV12,
            &bundle,
            &[wrong_operation]
        )
        .is_err()
    );
    let mut changed = source.clone();
    changed.text.push(' ');
    assert!(
        grammar::compile_with_settings(&changed, &schema, Profile::NativeV12, &bundle, &[contract])
            .is_err()
    );
    let mut package = intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(
        ECORE,
        r#"rcore 1; package settings version "1"; import model "urn:test:bound" as m; projection m.Root.computed from backing writable;"#,
    )
    .unwrap();
    package.schema = schema;
    package.sources.insert("xtext".into(), source.text.clone());
    package.grammar = Some(admitted);
    let evaluation = intersect_text::evaluate(
        &package,
        &[intersect_text::Source {
            uri: "model".into(),
            text: "root hello".into(),
        }],
    );
    assert!(
        evaluation
            .diagnostics
            .iter()
            .any(|d| d.message.contains("providers are not installed")),
        "{:?}",
        evaluation.diagnostics
    );
    // Retain the actual grammar request: replay still requires its explicit settings.
    let compilation = package.compilation.as_mut().unwrap();
    let mut behavior = compilation
        .bundle
        .source(&compilation.rcore_behavior)
        .unwrap()
        .clone();
    behavior.dependencies = vec![bundle.sources[0].id.clone()];
    compilation.ecore = bundle.sources[0].id.clone();
    compilation.bundle = bundle.clone();
    compilation.bundle.sources.push(behavior);
    compilation.xtext = Some(source.id.clone());
    let replay_errors = compiler::recompile(&package).unwrap_err();
    assert!(
        replay_errors
            .iter()
            .any(|d| d.message.contains("Cannot assign derived feature")),
        "{replay_errors:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn assigned_current_admission_uses_the_same_exact_member_contract() {
    use intersect_text::grammar;
    use intersect_text::grammar::Profile;
    let ecore = ECORE.replace(r#"x:type="e:EAttribute" name="computed" derived="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString""#,
        r##"x:type="e:EReference" name="computed" derived="true" upperBound="-1" containment="true" eType="#//Root""##);
    let (schema, mut bundle, mut contract, _) = fixture_for(&ecore, "Root");
    let source = DefinitionSource::new(
        "memory:capture.xtext",
        SourceKind::Xtext,
        r#"grammar bound.Test hidden(WS) import "urn:test:bound" as b Expr returns b::Root: Atom ({b::Root.computed+=current} '+' backing=ID)*; Atom returns b::Root: 'atom' backing=ID; terminal ID: ('a'..'z')+; terminal WS: ' '+;"#,
    );
    bundle.sources.push(source.clone());
    contract.sources.push(source.id.clone());
    contract.operations = vec![SettingOperation::Append];
    let admitted = grammar::compile_with_settings(
        &source,
        &schema,
        Profile::NativeV12,
        &bundle,
        &[contract.clone()],
    )
    .unwrap();
    assert!(admitted.rules["Expr"].has_current_actions());
    contract.operations = vec![SettingOperation::Set];
    assert!(
        grammar::compile_with_settings(&source, &schema, Profile::NativeV12, &bundle, &[contract])
            .is_err()
    );
    assert!(grammar::compile_with_profile(&source.text, &schema, Profile::NativeV11).is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn source_bound_linking_policy_preserves_strict_publication_and_rollback() {
    use intersect_model::core::storage::DraftReferencePolicy;
    let text=ECORE.replace("</eClassifiers>",r##"<eStructuralFeatures x:type="e:EReference" name="link" eType="#//Root"/></eClassifiers>"##);
    let (schema, bundle, contract, mut store) = fixture_for(&text, "Root");
    storage::atomic(&mut store, |s| {
        s.set_unresolved_reference("root", "link", "future", false)
    })
    .unwrap();
    let before = storage::export(&store).unwrap();
    let provider = Provider::new(&contract, "none");
    let service = SourceBoundSettings {
        schema: &schema,
        bundle: &bundle,
        expected: &contract,
        provider: &provider,
        conversions: None,
    };
    assert!(
        service
            .apply(
                &mut store,
                "root",
                ComputedEdit::Set(json!("value")),
                &mut QueryBudget::new(1000)
            )
            .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), before);
    let outer = store.begin().unwrap();
    service
        .apply_with_reference_policy(
            &mut store,
            "root",
            ComputedEdit::Set(json!("value")),
            &mut QueryBudget::new(1000),
            DraftReferencePolicy::PendingNonContainment,
        )
        .unwrap();
    assert_eq!(
        store.value("root", "backing").unwrap(),
        Some(json!("value"))
    );
    assert!(storage::validate_draft(&store, &schema).is_err());
    storage::set_with_opposite(&mut store, &schema, "root", "link", json!("root")).unwrap();
    storage::validate_draft(&store, &schema).unwrap();
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(&store).unwrap(), before);
    for drift in ["plan", "read"] {
        let provider = Provider::new(&contract, drift);
        let service = SourceBoundSettings {
            schema: &schema,
            bundle: &bundle,
            expected: &contract,
            provider: &provider,
            conversions: None,
        };
        assert!(
            service
                .apply_with_reference_policy(
                    &mut store,
                    "root",
                    ComputedEdit::Set(json!("value")),
                    &mut QueryBudget::new(1000),
                    DraftReferencePolicy::PendingNonContainment
                )
                .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), before);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn explicit_text_provider_executes_and_missing_or_drifting_selection_rejects() {
    use intersect_text::grammar;
    use intersect_text::grammar::Profile;
    use intersect_text::setting::TextSettings;
    use intersect_text::{EvaluationLimits, Source, evaluate_stored_with_settings};
    let (schema, mut bundle, mut contract, _) = fixture();
    let source = DefinitionSource::new(
        "memory:text-provider.xtext",
        SourceKind::Xtext,
        r#"grammar bound.Text hidden(WS) import "urn:test:bound" as b Root returns b::Root: 'root' computed=ID ';'; terminal ID: ('a'..'z')+; terminal WS: ' '+;"#,
    );
    bundle.sources.push(source.clone());
    contract.sources.push(source.id.clone());
    let admitted = grammar::compile_with_settings(
        &source,
        &schema,
        Profile::NativeV12,
        &bundle,
        &[contract.clone()],
    )
    .unwrap();
    let mut package = intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(
        ECORE,
        r#"rcore 1; package settings version "1"; import model "urn:test:bound" as m; projection m.Root.computed from backing writable;"#,
    )
    .unwrap();
    package.schema = schema;
    package.grammar = Some(admitted);
    let provider = Provider::new(&contract, "none");
    assert!(TextSettings::new(&package, &bundle, &[]).is_err());
    assert!(TextSettings::new(&package, &bundle, &[&provider, &provider]).is_err());
    let selected = TextSettings::new(&package, &bundle, &[&provider]).unwrap();
    let source = Source {
        uri: "text".into(),
        text: "root hello;".into(),
    };
    let result = evaluate_stored_with_settings(
        &package,
        std::slice::from_ref(&source),
        EvaluationLimits::native(),
        &selected,
    )
    .unwrap();
    assert!(
        result.evaluation.diagnostics.is_empty(),
        "{:?}",
        result.evaluation.diagnostics
    );
    assert_eq!(
        result.store.value("text#0", "backing").unwrap(),
        Some(json!("hello"))
    );
    assert_eq!(provider.calls.get(), 1);
    let rejected = evaluate_stored_with_settings(
        &package,
        &[Source {
            uri: "text".into(),
            text: "root hello".into(),
        }],
        EvaluationLimits::native(),
        &selected,
    )
    .unwrap();
    assert!(!rejected.evaluation.diagnostics.is_empty());
    assert!(
        storage::export(&rejected.store)
            .unwrap()
            .elements
            .is_empty()
    );
    let clone = package.clone();
    assert!(
        evaluate_stored_with_settings(
            &clone,
            std::slice::from_ref(&source),
            EvaluationLimits::native(),
            &selected
        )
        .is_err()
    );
    for drift in ["plan", "read"] {
        let provider = Provider::new(&contract, drift);
        let selected = TextSettings::new(&package, &bundle, &[&provider]).unwrap();
        assert!(
            evaluate_stored_with_settings(
                &package,
                std::slice::from_ref(&source),
                EvaluationLimits::native(),
                &selected
            )
            .is_err()
        );
    }
    let mut stale = contract.clone();
    stale.provider.version.push('x');
    let provider = Provider::new(&stale, "none");
    assert!(TextSettings::new(&package, &bundle, &[&provider]).is_err());
}

struct CaptureProvider {
    contract: ComputedSettingContract,
    calls: Cell<usize>,
}
impl SourceSettingValues for CaptureProvider {
    fn contract(&self) -> &ComputedSettingContract {
        &self.contract
    }
}
impl DerivedValues for CaptureProvider {
    fn schema_revision(&self) -> Option<&str> {
        Some(&self.contract.schema_revision)
    }
    fn supports(&self, class: &str, feature: &str) -> bool {
        class == "Root" && feature == "computed"
    }
    fn read(
        &self,
        store: &dyn ModelRead,
        id: &str,
        _: &str,
        budget: &mut QueryBudget,
    ) -> Result<Value, String> {
        budget.charge()?;
        Ok(store.value(id, "children")?.unwrap_or(json!([])))
    }
}
impl SettingValues for CaptureProvider {
    fn plan(
        &self,
        store: &dyn ModelRead,
        id: &str,
        _: &str,
        edit: &ComputedEdit,
        budget: &mut QueryBudget,
    ) -> Result<Vec<SettingEdit>, String> {
        budget.charge()?;
        self.calls.set(self.calls.get() + 1);
        let ComputedEdit::Append(value) = edit else {
            return Err("Append required".into());
        };
        let mut children = store
            .value(id, "children")?
            .unwrap_or(json!([]))
            .as_array()
            .ok_or("Expected children")?
            .clone();
        children.push(value.clone());
        Ok(vec![SettingEdit::Set {
            id: id.into(),
            feature: "children".into(),
            value: Value::Array(children),
        }])
    }
}
#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn computed_current_append_builds_nested_tree_and_rejects_without_leaks() {
    use intersect_text::grammar;
    use intersect_text::grammar::Profile;
    use intersect_text::setting::TextSettings;
    use intersect_text::{EvaluationLimits, Source, evaluate_stored_with_settings};
    let ecore=ECORE.replace(r#"x:type="e:EAttribute" name="computed" derived="true" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString""#,
        r##"x:type="e:EReference" name="computed" derived="true" upperBound="-1" eType="#//Root""##)
        .replace("</eClassifiers>",r##"<eStructuralFeatures x:type="e:EReference" name="children" upperBound="-1" containment="true" eType="#//Root"/></eClassifiers>"##);
    let (schema, mut bundle, mut contract, _) = fixture_for(&ecore, "Root");
    let source = DefinitionSource::new(
        "memory:current-provider.xtext",
        SourceKind::Xtext,
        r#"grammar bound.Current hidden(WS) import "urn:test:bound" as b Expr returns b::Root: Atom ({b::Root.computed+=current} '+' backing=ID)*; Atom returns b::Root: 'atom' backing=ID; terminal ID: ('a'..'z')+; terminal WS: ' '+;"#,
    );
    bundle.sources.push(source.clone());
    contract.sources.push(source.id.clone());
    contract.operations = vec![SettingOperation::Append];
    assert!(
        grammar::compile_with_settings(
            &source,
            &schema,
            Profile::NativeV21,
            &bundle,
            &[contract.clone()]
        )
        .is_err()
    );
    assert!(grammar::compile_with_profile(&source.text, &schema, Profile::NativeV22).is_err());
    let grammar = grammar::compile_with_settings(
        &source,
        &schema,
        Profile::NativeV22,
        &bundle,
        &[contract.clone()],
    )
    .unwrap();
    let mut package=intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(&ecore,r#"{"version":1,"scope":"unique_names","derived":[{"class":"Root","feature":"computed","expression":{"op":"stored","feature":"children","writable":true}}],"validations":[]}"#).unwrap();
    package.schema = schema;
    package.grammar = Some(grammar);
    let provider = CaptureProvider {
        contract,
        calls: Cell::new(0),
    };
    let selected = TextSettings::new(&package, &bundle, &[&provider]).unwrap();
    let result = evaluate_stored_with_settings(
        &package,
        &[Source {
            uri: "tree".into(),
            text: "atom a + b + c".into(),
        }],
        EvaluationLimits::native(),
        &selected,
    )
    .unwrap();
    assert!(
        result.evaluation.diagnostics.is_empty(),
        "{:?}",
        result.evaluation.diagnostics
    );
    assert_eq!(storage::export(&result.store).unwrap().elements.len(), 3);
    assert_eq!(
        result.store.value("tree#0", "backing").unwrap(),
        Some(json!("a"))
    );
    assert_eq!(
        result.store.value("tree#1", "children").unwrap(),
        Some(json!(["tree#0"]))
    );
    assert_eq!(
        result.store.value("tree#2", "children").unwrap(),
        Some(json!(["tree#1"]))
    );
    assert_eq!(
        result.store.value("tree#2", "backing").unwrap(),
        Some(json!("c"))
    );
    assert_eq!(provider.calls.get(), 2);
    let before = provider.calls.get();
    let rejected = evaluate_stored_with_settings(
        &package,
        &[Source {
            uri: "tree".into(),
            text: "atom a +".into(),
        }],
        EvaluationLimits::native(),
        &selected,
    )
    .unwrap();
    assert!(!rejected.evaluation.diagnostics.is_empty());
    assert!(
        storage::export(&rejected.store)
            .unwrap()
            .elements
            .is_empty()
    );
    assert!(provider.calls.get() > before);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn composed_settings_bind_inherited_sources_and_execute_shared_provider() {
    use intersect_text::grammar;
    use intersect_text::grammar::Profile;
    use intersect_text::grammar::SourceLimits;
    use intersect_text::setting::TextSettings;
    use intersect_text::{EvaluationLimits, Source, evaluate_stored_with_settings};
    let (schema, mut bundle, mut contract, _) = fixture();
    let base = DefinitionSource::new(
        "memory:base.xtext",
        SourceKind::Xtext,
        r#"grammar bound.Base hidden(WS) import "urn:test:bound" as b Root returns b::Root: 'root' computed=ID ';'; terminal ID: ('a'..'z')+; terminal WS: ' '+;"#,
    );
    let mut child = DefinitionSource::new(
        "memory:child.xtext",
        SourceKind::Xtext,
        r#"grammar bound.Child with bound.Base import "urn:test:bound" as b Entry returns b::Root: Root;"#,
    );
    child.dependencies.push(base.id.clone());
    bundle.sources.extend([base.clone(), child.clone()]);
    contract.sources.extend([base.id.clone(), child.id.clone()]);
    let compile = |contracts: &[ComputedSettingContract], limits: SourceLimits| {
        let mut current = schema.clone();
        let before = serde_json::to_value(&current).unwrap();
        let result = grammar::compile_sources_with_settings_and_limits(
            &bundle,
            &child.id,
            &mut current,
            Profile::NativeV22,
            limits,
            contracts,
        );
        if result.is_err() {
            assert_eq!(serde_json::to_value(current).unwrap(), before);
        }
        result
    };
    let limits = SourceLimits::default();
    assert!(compile(&[], limits).is_err());
    for omitted in [&base.id, &child.id] {
        let mut incomplete = contract.clone();
        incomplete.sources.retain(|id| id != omitted);
        assert!(compile(&[incomplete], limits).is_err());
    }
    assert!(compile(&[contract.clone(), contract.clone()], limits).is_err());
    let mut stale = contract.clone();
    stale.schema_revision.push('x');
    assert!(compile(&[stale], limits).is_err());
    assert!(
        compile(
            &[contract.clone()],
            SourceLimits {
                declarations: 3,
                ..limits
            }
        )
        .is_err()
    );
    let admitted = compile(&[contract.clone()], limits).unwrap();
    assert_eq!(admitted.declaration_sources["Root"].source, base.id);
    assert_eq!(admitted.declaration_sources["Entry"].source, child.id);
    assert_eq!(admitted.computed_settings, vec![contract.clone()]);
    let replay = compile(&[contract.clone()], limits).unwrap();
    assert_eq!(
        serde_json::to_value(&admitted).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    let mut package = intersect_text_acceptance::migration_fixture::compile_migrated_structure_fixture(ECORE,
        r#"rcore 1; package settings version "1"; import model "urn:test:bound" as m; projection m.Root.computed from backing writable;"#).unwrap();
    package.schema = schema;
    package.grammar = Some(admitted);
    let provider = Provider::new(&contract, "none");
    let selected = TextSettings::new(&package, &bundle, &[&provider]).unwrap();
    let result = evaluate_stored_with_settings(
        &package,
        &[Source {
            uri: "composed".into(),
            text: "root hello;".into(),
        }],
        EvaluationLimits::native(),
        &selected,
    )
    .unwrap();
    assert!(
        result.evaluation.diagnostics.is_empty(),
        "{:?}",
        result.evaluation.diagnostics
    );
    assert_eq!(
        result.store.value("composed#0", "backing").unwrap(),
        Some(json!("hello"))
    );
    assert_eq!(provider.calls.get(), 1);
    let rejected = evaluate_stored_with_settings(
        &package,
        &[Source {
            uri: "composed".into(),
            text: "root hello".into(),
        }],
        EvaluationLimits::native(),
        &selected,
    )
    .unwrap();
    assert!(!rejected.evaluation.diagnostics.is_empty());
    assert!(
        storage::export(&rejected.store)
            .unwrap()
            .elements
            .is_empty()
    );
    assert!(provider.calls.get() > 1);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn package_settings_recompile_requires_explicit_matching_contracts() {
    use intersect_text::grammar::Profile;
    use intersect_text::setting::TextSettings;
    use intersect_text::{EvaluationLimits, Source, evaluate_stored_with_settings};
    for composed in [false, true] {
        let (_, mut bundle, mut contract, _) = fixture();
        let ecore = bundle.sources[0].id.clone();
        let mut base = DefinitionSource::new(
            "memory:package-base.xtext",
            SourceKind::Xtext,
            r#"grammar bound.Base hidden(WS) import "urn:test:bound" as b Root returns b::Root: 'root' computed=ID ';'; terminal ID: ('a'..'z')+; terminal WS: ' '+;"#,
        );
        base.dependencies.push(ecore.clone());
        let mut child = DefinitionSource::new(
            "memory:package-child.xtext",
            SourceKind::Xtext,
            r#"grammar bound.Child with bound.Base import "urn:test:bound" as b Entry returns b::Root: Root;"#,
        );
        child.dependencies.extend([base.id.clone(), ecore.clone()]);
        let mut semantics = DefinitionSource::new(
            "memory:package-behavior.rcore",
            SourceKind::RcoreBehavior,
            r#"rcore 1; package settings version "1"; import model "urn:test:bound" as m; projection m.Root.computed from backing writable;"#,
        );
        semantics.dependencies.push(ecore.clone());
        contract
            .sources
            .extend([base.id.clone(), semantics.id.clone()]);
        bundle.sources.extend([base.clone(), semantics.clone()]);
        let xtext = if composed {
            contract.sources.push(child.id.clone());
            bundle.sources.push(child.clone());
            child.id
        } else {
            base.id
        };
        let input = BundleCompilation {
            materialized_types: vec![],
            bundle,
            ecore,
            namespace: "urn:test:bound".into(),
            xtext: Some(xtext),
            rcore_behavior: semantics.id,
        };
        let limits = CompilationSourcePolicy::default();
        assert!(compiler::compile_bundle_with_profile(input.clone(), Profile::NativeV22).is_err());
        let package = compiler::compile_bundle_with_settings_and_source_policy(
            input.clone(),
            Profile::NativeV22,
            &[contract.clone()],
            limits,
            limits,
        )
        .unwrap();
        assert_eq!(package.version, 2);
        assert!(compiler::recompile(&package).is_err());
        let replay = compiler::recompile_with_settings_and_source_policy(
            &package,
            &[contract.clone()],
            limits,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&package).unwrap(),
            serde_json::to_value(&replay).unwrap()
        );
        let mut changed = contract.clone();
        changed.provider.version.push('x');
        assert!(
            compiler::recompile_with_settings_and_source_policy(
                &package,
                &[changed.clone()],
                limits
            )
            .is_err()
        );
        let mut forged = package.clone();
        forged.grammar.as_mut().unwrap().computed_settings = vec![changed];
        assert!(
            compiler::recompile_with_settings_and_source_policy(
                &forged,
                &[contract.clone()],
                limits
            )
            .is_err()
        );
        let tiny = CompilationSourcePolicy {
            xtext_bytes: 1,
            ..limits
        };
        assert!(
            compiler::compile_bundle_with_settings_and_source_policy(
                input.clone(),
                Profile::NativeV22,
                &[contract.clone()],
                limits,
                tiny
            )
            .is_err()
        );
        assert!(
            compiler::recompile_with_settings_and_source_policy(
                &package,
                &[contract.clone()],
                tiny
            )
            .is_err()
        );
        let provider = Provider::new(&contract, "none");
        let bundle = &replay.compilation.as_ref().unwrap().bundle;
        let selected = TextSettings::new(&replay, bundle, &[&provider]).unwrap();
        let result = evaluate_stored_with_settings(
            &replay,
            &[Source {
                uri: "package".into(),
                text: "root hello;".into(),
            }],
            EvaluationLimits::native(),
            &selected,
        )
        .unwrap();
        assert!(
            result.evaluation.diagnostics.is_empty(),
            "{:?}",
            result.evaluation.diagnostics
        );
        assert_eq!(
            result.store.value("package#0", "backing").unwrap(),
            Some(json!("hello"))
        );
        assert_eq!(provider.calls.get(), 1);
        use intersect_text::distribution::ordinary::{self as distribution, PackageMetadata};
        let metadata = PackageMetadata {
            package_id: "test.bound".into(),
            release: "1".into(),
            display_name: "Bound setters".into(),
            description: "Authored archive control".into(),
        };
        let archive = distribution::build_bundle_with_settings_and_source_policy(
            input.clone(),
            metadata.clone(),
            Profile::NativeV22,
            &[contract.clone()],
            limits,
            limits,
        )
        .unwrap();
        let mut files = archive.files.clone();
        files.insert(
            "manifest.json".into(),
            serde_json::to_string(&archive.manifest).unwrap(),
        );
        assert!(distribution::import_files(files.clone()).is_err());
        let restored = distribution::import_files_with_settings_and_source_policy(
            files.clone(),
            &[contract.clone()],
            limits,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&restored.package).unwrap(),
            serde_json::to_value(&package).unwrap()
        );
        assert_eq!(restored.manifest.revision, archive.manifest.revision);
        let selected = TextSettings::new(
            &restored.package,
            &restored.package.compilation.as_ref().unwrap().bundle,
            &[&provider],
        )
        .unwrap();
        let result = evaluate_stored_with_settings(
            &restored.package,
            &[Source {
                uri: "archive".into(),
                text: "root saved;".into(),
            }],
            EvaluationLimits::native(),
            &selected,
        )
        .unwrap();
        assert!(result.evaluation.diagnostics.is_empty());
        assert_eq!(
            result.store.value("archive#0", "backing").unwrap(),
            Some(json!("saved"))
        );
        let mut other = contract.clone();
        other.provider.version.push('x');
        let alternate = distribution::build_bundle_with_settings_and_source_policy(
            input,
            metadata,
            Profile::NativeV22,
            &[other.clone()],
            limits,
            limits,
        )
        .unwrap();
        assert_ne!(alternate.manifest.revision, archive.manifest.revision);
        assert!(
            distribution::import_files_with_settings_and_source_policy(
                files.clone(),
                &[other.clone()],
                limits
            )
            .is_err()
        );
        assert!(
            distribution::import_files_with_settings_and_source_policy(
                files.clone(),
                &[contract.clone()],
                tiny
            )
            .is_err()
        );
        let mut changed_manifest = archive.manifest.clone();
        changed_manifest.runtime.computed_settings = vec![other.clone()];
        let mut metadata_forged = files.clone();
        metadata_forged.insert(
            "manifest.json".into(),
            serde_json::to_string(&changed_manifest).unwrap(),
        );
        assert!(
            distribution::import_files_with_settings_and_source_policy(
                metadata_forged,
                &[other],
                limits
            )
            .unwrap_err()
            .contains("identity mismatch")
        );
        let mut saved: Value = serde_json::from_str(&files["package.json"]).unwrap();
        saved["grammar"]["computed_settings"][0]["provider"]["version"] = json!("forged");
        let payload = serde_json::to_string(&saved).unwrap();
        let digest = DefinitionSource::new("digest", SourceKind::Xtext, &payload)
            .id
            .sha256;
        let mut changed_manifest = archive.manifest.clone();
        let entry = changed_manifest.files.get_mut("package.json").unwrap();
        entry.sha256 = digest;
        entry.bytes = payload.len();
        files.insert("package.json".into(), payload);
        files.insert(
            "manifest.json".into(),
            serde_json::to_string(&changed_manifest).unwrap(),
        );
        assert!(
            distribution::import_files_with_settings_and_source_policy(files, &[contract], limits)
                .unwrap_err()
                .contains("freshly compiled")
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
pub fn computed_contracts_bind_after_explicit_ecore_context_preparation() {
    use intersect_text::grammar;
    use intersect_text::grammar::Profile;
    use intersect_text::grammar::SourceLimits;
    let (mut schema, mut bundle, mut contract, _) = fixture();
    let source = DefinitionSource::new(
        "memory:ecore-context.xtext",
        SourceKind::Xtext,
        r#"grammar bound.Context hidden() import "urn:test:bound" as b import "http://www.eclipse.org/emf/2002/Ecore" as e Root returns b::Root: computed=ID; terminal ID: ('a'..'z')+;"#,
    );
    bundle.sources.push(source.clone());
    contract.sources.push(source.id.clone());
    let before = serde_json::to_value(&schema).unwrap();
    assert!(
        grammar::compile_sources_with_settings_and_limits(
            &bundle,
            &source.id,
            &mut schema,
            Profile::NativeV22,
            SourceLimits::default(),
            &[contract.clone()]
        )
        .is_err()
    );
    assert_eq!(serde_json::to_value(&schema).unwrap(), before);
    grammar::prepare_ecore_reference_context(&mut schema).unwrap();
    assert_ne!(schema.revision, contract.schema_revision);
    let prepared = serde_json::to_value(&schema).unwrap();
    grammar::prepare_ecore_reference_context(&mut schema).unwrap();
    assert_eq!(serde_json::to_value(&schema).unwrap(), prepared);
    assert!(contract.validate(&schema, &bundle).is_err());
    contract.schema_revision = schema.revision.clone();
    let grammar = grammar::compile_sources_with_settings_and_limits(
        &bundle,
        &source.id,
        &mut schema,
        Profile::NativeV22,
        SourceLimits::default(),
        &[contract.clone()],
    )
    .unwrap();
    assert_eq!(grammar.computed_settings, vec![contract]);
    assert_eq!(serde_json::to_value(&schema).unwrap(), prepared);
}
