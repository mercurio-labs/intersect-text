use super::*;
use crate::package::BundleCompilation;
use crate::{
    EvaluationLimits, Source, compiler, evaluate_stored, evaluate_stored_with_linking_context,
    linking::LinkingContext,
};
use intersect_model::{
    core::storage::{ModelRead, ValueView},
    definitions::{conversion::InvocationContext, schema::MemberIdentity},
    query::PackageDerived,
};
use serde_json::json;

struct ReadBoundary<'a> {
    store: &'a dyn ModelRead,
    forged: Option<&'a str>,
}
impl ModelRead for ReadBoundary<'_> {
    fn revision(&self) -> Result<String, String> {
        self.store.revision()
    }
    fn get(&self, id: &str) -> Result<Option<intersect_model::model::Element>, String> {
        if self.forged != Some(id) {
            return Err("Whole-object materialization is disabled".into());
        }
        let mut value = self.store.get(id)?.ok_or("Missing forged test object")?;
        value.type_identity = "foreign:type-identity".into();
        Ok(Some(value))
    }
    fn class(&self, id: &str) -> Result<Option<String>, String> {
        self.store.class(id)
    }
    fn page(&self, _: Option<u64>, _: usize) -> Result<Vec<intersect_model::storage::Row>, String> {
        Err("Whole-model export is disabled".into())
    }
    fn id_page_observed(
        &self,
        after: Option<u64>,
        limit: usize,
        observation: &InvocationContext<'_>,
    ) -> Result<Vec<(u64, String)>, String> {
        self.store.id_page_observed(after, limit, observation)
    }
    fn query(&self, query: intersect_model::storage::Query<'_>) -> Result<Vec<String>, String> {
        self.store.query(query)
    }
    fn matches(
        &self,
        id: &str,
        query: intersect_model::storage::Query<'_>,
    ) -> Result<bool, String> {
        self.store.matches(id, query)
    }
    fn inspect(
        &self,
        id: &str,
        consume: &mut dyn FnMut(intersect_model::storage::ObjectView<'_>) -> Result<(), String>,
    ) -> Result<bool, String> {
        if self.forged == Some(id) {
            let value = self.get(id)?.ok_or("Missing forged test object")?;
            consume(intersect_model::storage::ObjectView::from_element(&value))?;
            Ok(true)
        } else {
            self.store.inspect(id, consume)
        }
    }
    fn inspect_stored_value_observed(
        &self,
        id: &str,
        field: &str,
        many: bool,
        observation: &InvocationContext<'_>,
        consume: &mut dyn FnMut(ValueView<'_>) -> Result<(), String>,
    ) -> Result<bool, String> {
        self.store
            .inspect_stored_value_observed(id, field, many, observation, consume)
    }
}

const RCORE: &str = r#"rcore 1;
package example.scoped version "1";
metamodel "urn:example:scoped" prefix m {
  class Namespace {
    attr name: String [1,1];
    ref types: Type [0,*] containment;
    ref properties: Property [0,*] containment opposite Property.namespace;
  }
  class Type { attr name: String [1,1]; }
  class Property {
    attr name: String [1,1];
    ref namespace: Namespace [1,1] opposite Namespace.properties;
    ref targetType: Type [1,1];
  }
}
behavior {
  import model "urn:example:scoped" as m;
  bind scope m.Property.targetType = visible_types;
  fn visible_types(owner:m.Property, spelling:String)->Sequence<m.Type> =
    match container::<m.Namespace>(owner) {
      Some(ns) => Sequence(filter(ns.types, |target| target.name == spelling)),
      None => []
    };
}
"#;
const XTEXT: &str = r#"grammar example.Scoped with org.eclipse.xtext.common.Terminals
import "urn:example:scoped" as m
Namespace returns m::Namespace:
  'namespace' name=ID '{' (types+=Type | properties+=Property)* '}';
Type returns m::Type: 'type' name=ID ';';
Property returns m::Property: 'property' name=ID ':' targetType=[m::Type|ID] ';';
"#;

fn package(source: &str) -> Package {
    compiler::compile_rcore_project(source, XTEXT).expect("source-defined scoped language")
}
fn sources() -> Vec<Source> {
    vec![
        Source {
            uri: "memory:electrical".into(),
            text: "namespace Electrical { property supply : Voltage; type Voltage; }".into(),
        },
        Source {
            uri: "memory:hydraulic".into(),
            text: "namespace Hydraulic { type Voltage; property supply : Voltage; }".into(),
        },
    ]
}
fn run(
    package: &Package,
    sources: &[Source],
    budget: &mut QueryBudget,
) -> Result<crate::EvaluatedStore, String> {
    let scopes =
        RcoreScopes::from_package(package, &Limits::default(), ExecutionLimits::default())?;
    evaluate_stored_with_linking_context(
        package,
        sources,
        EvaluationLimits::native(),
        None,
        None,
        &PackageDerived(package),
        &mut LinkingContext {
            resolver: &scopes,
            work: budget,
        },
    )
}

#[test]
fn ordinary_exact_member_lookup_uses_the_original_observer() {
    let package = package(RCORE);
    let member = package
        .schema
        .effective_feature("Property", "targetType")
        .unwrap();
    let MemberIdentity::Source { declaration } = member.identity else {
        panic!("source-defined test member");
    };
    let mut denied = QueryBudget::new(0);
    let work = BorrowedBindingWork::bind(&mut denied);
    assert!(observed_member(&package.schema, "Property", &declaration, &work.observer()).is_err());
    let mut allowed = QueryBudget::new(10_000);
    let work = BorrowedBindingWork::bind(&mut allowed);
    assert_eq!(
        observed_member(&package.schema, "Property", &declaration, &work.observer())
            .unwrap()
            .name,
        "targetType"
    );
    assert!(work.used() > 1);
}

#[test]
fn source_scope_rejects_same_identity_scalar_or_containment_contract_mutation() {
    for reference in [false, true] {
        let mut package = package(RCORE);
        let owner = package
            .schema
            .effective_feature("Property", "targetType")
            .unwrap()
            .owner
            .to_owned();
        let field = package
            .schema
            .classes
            .get_mut(&owner)
            .unwrap()
            .features
            .get_mut("targetType")
            .unwrap();
        field.reference = reference;
        field.containment = reference;
        assert!(
            RcoreScopes::from_package(&package, &Limits::default(), ExecutionLimits::default())
                .is_err()
        );
    }
}

#[test]
fn source_scope_resolves_forward_references_inside_each_namespace() {
    let package = package(RCORE);
    let sources = sources();
    let mut budget = QueryBudget::new(100_000);
    let result = run(&package, &sources, &mut budget).expect("source scope runs");
    assert!(
        result.evaluation.diagnostics.is_empty(),
        "{:?}",
        result.evaluation.diagnostics
    );
    assert_eq!(result.evaluation.references.len(), 2);
    for reference in &result.evaluation.references {
        let target = result.store.get(&reference.target).unwrap().unwrap();
        assert_eq!(target.class, "Type");
        assert_eq!(target.resource, reference.uri);
        assert_eq!(target.values.get("name"), Some(&json!("Voltage")));
        assert_eq!(
            &sources
                .iter()
                .find(|source| source.uri == reference.uri)
                .unwrap()
                .text[reference.start..reference.end],
            "Voltage"
        );
    }
    assert!(budget.reads > 0);
}

#[test]
fn declared_scope_cannot_silently_use_the_legacy_global_policy() {
    let package = package(RCORE);
    let error = evaluate_stored(&package, &sources(), EvaluationLimits::native())
        .err()
        .expect("missing scope adapter must fail");
    assert!(error.contains("installed Rcore scope"), "{error}");
    let mut budget = QueryBudget::new(100_000);
    let error = evaluate_stored_with_linking_context(
        &package,
        &sources(),
        EvaluationLimits::native(),
        None,
        None,
        &PackageDerived(&package),
        &mut LinkingContext {
            resolver: &crate::linking::UniqueNames,
            work: &mut budget,
        },
    )
    .err()
    .expect("legacy context cannot replace declared source scopes");
    assert!(error.contains("compiled scope context"), "{error}");
    assert_eq!(budget.reads, 0);
}

#[test]
fn empty_scope_has_no_global_fallback_and_ambiguity_retains_source_location() {
    let empty = RCORE.replace("target.name == spelling", "false");
    let empty = package(&empty);
    let result = run(&empty, &sources(), &mut QueryBudget::new(100_000)).unwrap();
    assert_eq!(result.evaluation.diagnostics.len(), 2);
    assert!(
        result
            .evaluation
            .diagnostics
            .iter()
            .all(|d| d.message.contains("Unresolved"))
    );
    assert!(result.evaluation.references.is_empty());

    let package = package(RCORE);
    let sources = [Source {
        uri: "memory:ambiguous".into(),
        text: "namespace N { type Voltage; type Voltage; property supply : Voltage; }".into(),
    }];
    let result = run(&package, &sources, &mut QueryBudget::new(100_000)).unwrap();
    assert_eq!(result.evaluation.diagnostics.len(), 1);
    let diagnostic = &result.evaluation.diagnostics[0];
    assert!(
        diagnostic.message.contains("Ambiguous"),
        "{}",
        diagnostic.message
    );
    assert_eq!(
        &sources[0].text[diagnostic.start..diagnostic.end],
        "Voltage"
    );
}

#[test]
fn exact_and_exhausted_scope_budgets_do_not_change_model_results() {
    let package = package(RCORE);
    let sources = sources();
    let mut baseline_budget = QueryBudget::new(100_000);
    let baseline = run(&package, &sources, &mut baseline_budget).unwrap();
    let used = baseline_budget.reads;
    let mut exact = QueryBudget::new(used);
    let repeated = run(&package, &sources, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    assert_eq!(
        serde_json::to_value(intersect_model::storage::export(&baseline.store).unwrap()).unwrap(),
        serde_json::to_value(intersect_model::storage::export(&repeated.store).unwrap()).unwrap()
    );
    assert!(run(&package, &sources, &mut QueryBudget::new(used - 1)).is_err());
    assert!(run(&package, &sources, &mut QueryBudget::new(0)).is_err());
}

#[test]
fn scope_services_reject_another_package_even_with_an_equal_revision() {
    let package = package(RCORE);
    let copied: Package = serde_json::from_value(serde_json::to_value(&package).unwrap()).unwrap();
    let scopes =
        RcoreScopes::from_package(&package, &Limits::default(), ExecutionLimits::default())
            .unwrap();
    assert!(scopes.validate_package(&copied).is_err());
    assert!(scopes.validate_package(&package).is_ok());
    let mut forged = copied;
    forged.semantics.scope_bindings[0].source.sha256 = "0".repeat(64);
    assert!(
        RcoreScopes::from_package(&forged, &Limits::default(), ExecutionLimits::default()).is_err()
    );
}

#[test]
fn source_scope_uses_borrowed_fields_and_rejects_foreign_object_type_identity() {
    let package = package(RCORE);
    let result = run(&package, &sources(), &mut QueryBudget::new(100_000)).unwrap();
    let owner = result
        .store
        .query(intersect_model::storage::Query::Class("Property"))
        .unwrap()
        .remove(0);
    let request = ReferenceRequest {
        owner: &owner,
        feature: "targetType",
        spelling: "Voltage",
        target_class: "Type",
        uri: "memory:test",
        start: 0,
        end: 7,
    };
    let scopes =
        RcoreScopes::from_package(&package, &Limits::default(), ExecutionLimits::default())
            .unwrap();
    let borrowed = ReadBoundary {
        store: &result.store,
        forged: None,
    };
    assert_eq!(
        scopes
            .candidates(&borrowed, &request, &mut QueryBudget::new(100_000))
            .unwrap()
            .len(),
        1
    );
    let forged = ReadBoundary {
        store: &result.store,
        forged: Some(&owner),
    };
    let error = scopes
        .candidates(&forged, &request, &mut QueryBudget::new(100_000))
        .unwrap_err();
    assert!(error.contains("foreign source/type identity"), "{error}");
}

fn distribution(source: &str) -> crate::distribution::Distribution {
    let package = package(source);
    crate::distribution::build_bundle_with_profile(
        package.compilation.expect("retained sources"),
        crate::distribution::PackageMetadata {
            package_id: "example.scoped".into(),
            release: "1".into(),
            display_name: "Source scoped language".into(),
            description: String::new(),
        },
        crate::grammar::Profile::default(),
    )
    .expect("source archive builds")
}

#[test]
fn source_scope_archive_rebuilds_bindings_and_rejects_forged_compiled_selectors() {
    let mut exported = distribution(RCORE);
    let revision = exported.manifest.revision.clone();
    exported.files.insert(
        "manifest.json".into(),
        serde_json::to_string(&exported.manifest).unwrap(),
    );
    let loaded =
        crate::distribution::import_files(exported.files.clone()).expect("source archive reloads");
    assert_eq!(loaded.manifest.revision, revision);
    assert_eq!(
        loaded.package.semantics.scope_bindings,
        exported.package.semantics.scope_bindings
    );
    let result = run(&loaded.package, &sources(), &mut QueryBudget::new(100_000)).unwrap();
    assert!(
        result.evaluation.diagnostics.is_empty(),
        "{:?}",
        result.evaluation.diagnostics
    );
    assert_ne!(
        distribution(&format!("{RCORE}\n// changed source revision\n"))
            .manifest
            .revision,
        revision
    );

    // Repair the inventory digest deliberately: source reconstruction must still
    // reject a forged binding selector instead of trusting cached package IR.
    let path = exported.manifest.compiled_package.clone();
    let mut forged: serde_json::Value = serde_json::from_str(&exported.files[&path]).unwrap();
    forged["semantics"]["scope_bindings"][0]["declaration"] = json!(9999);
    let forged = serde_json::to_string(&forged).unwrap();
    let digest = exported.manifest.files.get_mut(&path).unwrap();
    digest.bytes = forged.len();
    digest.sha256 =
        intersect_model::definitions::bundle::SourceId::for_text("memory:forged", &forged).sha256;
    exported.files.insert(path, forged);
    exported.files.insert(
        "manifest.json".into(),
        serde_json::to_string(&exported.manifest).unwrap(),
    );
    assert!(crate::distribution::import_files(exported.files).is_err());
}

#[test]
fn retained_scope_activation_rejects_valid_alternative_source_selector() {
    use intersect_model::definitions::{
        bundle::{DefinitionSource, SourceKind},
        rcore_scope::ScopeBindingDescriptor,
    };
    let mut package = package(RCORE);
    let recipe = package.compilation.as_mut().unwrap();
    let mut alternative = DefinitionSource::new(
        "memory:alternative-scope.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1;
package example.alternative version "1";
import model "urn:example:scoped" as m;
bind scope m.Property.targetType = hidden_types;
fn hidden_types(owner:m.Property, spelling:String)->Sequence<m.Type> = [];
"#,
    );
    alternative.dependencies.push(recipe.ecore.clone());
    recipe.bundle.sources.push(alternative.clone());
    // This is a well-typed scope for the same exact reference, but it is not a
    // declaration in the selected entry that originally produced the package.
    intersect_model::definitions::rcore_scope::compile_scope(
        &recipe.bundle,
        &alternative.id,
        1,
        &Limits::default(),
    )
    .expect("the alternate selector is independently valid");
    package.semantics.scope_bindings = vec![ScopeBindingDescriptor {
        source: alternative.id,
        declaration: 1,
    }];
    assert!(validate_retained_scope_bindings_for_evaluation(&package).is_err());
    let error = RcoreScopes::from_package(&package, &Limits::default(), ExecutionLimits::default())
        .err()
        .expect("cached selector cannot redirect authored activation");
    assert!(error.contains("selected authored entry"), "{error}");
}

#[test]
fn removing_cached_scope_selectors_cannot_enable_default_global_linking() {
    for source in [
        RCORE.to_owned(),
        RCORE.replace("bind scope", "bind /* keyword boundary */\n scope"),
    ] {
        let mut package = package(&source);
        package.semantics.scope_bindings.clear();
        assert!(validate_retained_scope_bindings_for_evaluation(&package).is_err());
        let error = evaluate_stored(&package, &sources(), EvaluationLimits::native())
            .err()
            .expect("deleted cached selectors must be reconstructed before default evaluation");
        assert!(error.contains("selected authored entry"), "{error}");

        // Removing the keyword from stale text must not take the empty-selector
        // fast path while retaining the original authored content digest.
        let recipe = package.compilation.as_mut().unwrap();
        let entry = recipe.rcore_behavior.clone();
        let retained = recipe
            .bundle
            .sources
            .iter_mut()
            .find(|source| source.id == entry)
            .unwrap();
        retained.text = retained.text.replace("bind", "removed");
        assert!(validate_retained_scope_bindings_for_evaluation(&package).is_err());
    }
}

#[test]
fn descriptor_only_evaluation_replay_preserves_already_retained_large_source() {
    use intersect_model::definitions::{
        bundle::{DefinitionSource, SourceKind},
        rcore_scope::ScopeBindingDescriptor,
    };
    let mut package = package(RCORE);
    let recipe = package.compilation.as_mut().unwrap();
    let mut text = String::from(
        "rcore 1;\npackage example.large version \"1\";\nimport model \"urn:example:scoped\" as m;\nbind scope m.Property.targetType = hidden_types;\nfn hidden_types(owner:m.Property, spelling:String)->Sequence<m.Type> = [];\n//",
    );
    text.push_str(&"x".repeat(ParseLimits::default().source_bytes));
    let mut large = DefinitionSource::new(
        "memory:retained-large-scope.rcore",
        SourceKind::RcoreBehavior,
        text,
    );
    large.dependencies.push(recipe.ecore.clone());
    recipe.rcore_behavior = large.id.clone();
    recipe.bundle.sources.push(large.clone());
    package.semantics.scope_bindings = vec![ScopeBindingDescriptor {
        source: large.id,
        declaration: 1,
    }];
    assert!(validate_retained_scope_bindings(&package, ParseLimits::default()).is_err());
    validate_retained_scope_bindings_for_evaluation(&package)
        .expect("descriptor replay does not lower prior source admission");
    assert!(
        RcoreScopes::from_package(&package, &Limits::default(), ExecutionLimits::default())
            .is_err(),
        "descriptor replay must not enlarge executable source admission"
    );
}

#[test]
fn runtime_owner_views_link_in_the_callers_store_and_reject_foreign_services() -> Result<(), String>
{
    use crate::grammar::Profile;
    use crate::grammar::RuntimeGuardedPolicy;
    use crate::{bounded_runtime, ordinary, setting::TextSettings};
    use intersect_model::frozen::FrozenStore;
    let model = DefinitionSource::new("memory:runtime-scopes.rcore", SourceKind::Rcore, RCORE);
    let syntax_text = XTEXT
        .replace(
            " with org.eclipse.xtext.common.Terminals",
            " hidden(WS)\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore",
        )
        .replace("(types+=Type | properties+=Property)*", "(=> Item)*")
        + r#"
fragment Item returns m::Namespace: types+=Type | properties+=Property;
terminal ID returns ecore::EString: ('a'..'z' | 'A'..'Z')+;
terminal WS returns ecore::EString: (' ' | '\t' | '\r' | '\n')+;
"#;
    let mut syntax = DefinitionSource::new(
        "memory:runtime-scopes.xtext",
        SourceKind::Xtext,
        syntax_text,
    );
    syntax.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        rcore_behavior: model.id.clone(),
        namespace: "urn:example:scoped".into(),
        xtext: Some(syntax.id.clone()),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, syntax],
            namespaces: vec![],
        },
    };
    let compiled = ordinary::compile_with_runtime_guarded_policy(
        input,
        Profile::NativeV80,
        &[],
        None,
        Default::default(),
        Default::default(),
        RuntimeGuardedPolicy::StrictCompletionScopes,
    )
    .map_err(|errors| format!("{errors:?}"))?;
    let (definition, admission) = compiled.parts();
    let grammar = definition.grammar.ok_or("No runtime grammar")?;
    let bundle = definition
        .compilation
        .ok_or("No exact retained inputs")?
        .bundle;
    let scopes =
        RcoreScopes::from_package(definition, &Limits::default(), ExecutionLimits::default())?;
    let settings = TextSettings::new(definition, bundle, &[])?;
    let mut store =
        FrozenStore::for_schema(definition.schema).map_err(|error| error.to_string())?;
    let mut parse_work = QueryBudget::new(1_000_000);
    let mut link_work = QueryBudget::new(1_000_000);
    let before = serde_json::to_value(definition).map_err(|error| error.to_string())?;
    let (evaluation, consumed) = bounded_runtime::evaluate_into(
        definition,
        grammar,
        admission,
        &sources(),
        EvaluationLimits::native(),
        &mut store,
        Some(&settings),
        &mut LinkingContext {
            resolver: &scopes,
            work: &mut link_work,
        },
        &mut parse_work,
    )?;
    assert!(
        evaluation.diagnostics.is_empty(),
        "{:?}",
        evaluation.diagnostics
    );
    assert!(consumed > 0 && parse_work.reads > 0 && link_work.reads > 0);
    let model = store.into_model()?;
    assert_eq!(model.elements.len(), 6);
    for property in model
        .elements
        .iter()
        .filter(|element| element.class == "Property")
    {
        let namespace = property.values["namespace"]
            .as_str()
            .ok_or("No property namespace")?;
        let target = property.values["targetType"]
            .as_str()
            .ok_or("No resolved target")?;
        let owner = model
            .elements
            .iter()
            .find(|element| element.id == namespace)
            .ok_or("No actual namespace")?;
        assert_eq!(owner.values["types"], json!([target]));
    }
    assert_eq!(
        serde_json::to_value(definition).map_err(|error| error.to_string())?,
        before
    );
    // Equal exported DTO data does not substitute for the actual bound component world.
    let copied = definition.to_owned();
    let foreign_scopes =
        RcoreScopes::from_package(&copied, &Limits::default(), ExecutionLimits::default())?;
    assert!(scopes.validate_package(&copied).is_err());
    assert!(!settings.belongs_to(&copied));
    let mut store =
        FrozenStore::for_schema(definition.schema).map_err(|error| error.to_string())?;
    let empty = intersect_model::storage::export(&store)?;
    let refused = bounded_runtime::evaluate_into(
        definition,
        grammar,
        admission,
        &sources(),
        EvaluationLimits::native(),
        &mut store,
        Some(&settings),
        &mut LinkingContext {
            resolver: &foreign_scopes,
            work: &mut link_work,
        },
        &mut parse_work,
    );
    assert!(
        refused
            .err()
            .ok_or("Foreign scope services were accepted")?
            .contains("different compiled package")
    );
    assert_eq!(intersect_model::storage::export(&store)?, empty);
    Ok(())
}
