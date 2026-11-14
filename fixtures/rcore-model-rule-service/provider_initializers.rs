//! Handwritten native-provider controls over direct Rcore and the shared atomic
//! initializer. This is an explicit constructor projection, not Pilot semantics.
use intersect_model::{
    api::ModelEdit,
    definitions::{
        conversion::ProviderId,
        generic_types::{RuntimeType, Type},
        initialization::ClassInitializationContract,
        resources::DeclarationId,
    },
    query::QueryBudget,
    rcore::{
        DefinitionBundle, DefinitionSource, RcoreModel, RcorePackage, SourceKind,
        compile_rcore_bundle, compile_rcore_bundle_selected_with_initializers,
        compile_rcore_bundle_with_initializers,
    },
    storage::{ClassInitializer, Construction, InitialValue, ModelRead},
};
use serde_json::{Value, json};
use std::cell::Cell;

const SOURCE: &str = r#"rcore 1; package directRelationship version "1";
metamodel "urn:direct-relationship" prefix d {
 class Type { attr name: String [1,1] default "unnamed"; }
 class Feature extends Type {}
 class Literal { attr name: String [1,1] default "not-a-type"; }
 class FeatureTyping {
   ref type: Type [1,1] readonly;
   ref typedFeature: Feature [1,1] readonly;
   attr origin: String [1,1] readonly default "schema-origin";
   attr label: String [1,1] default "schema-label";
 }
}
behavior {
 import model "urn:direct-relationship" as d;
 initialize d.FeatureTyping using "basic.relationship-constructor" version "1" {
   type; typedFeature; origin;
 }
}"#;

fn selected() -> ProviderId {
    ProviderId {
        name: "basic.relationship-constructor".into(),
        version: "1".into(),
    }
}
fn bundle(uri: &str, source: &str) -> (DefinitionBundle, intersect_model::rcore::SourceId) {
    let source = DefinitionSource::new(uri, SourceKind::Rcore, source);
    let entry = source.id.clone();
    (
        DefinitionBundle {
            version: 1,
            sources: vec![source],
            namespaces: vec![],
        },
        entry,
    )
}
fn package() -> Result<RcorePackage, String> {
    let (bundle, entry) = bundle("memory:relationship.rcore", SOURCE);
    compile_rcore_bundle_with_initializers(bundle, &entry, &[selected()])
        .map_err(|error| error.to_string())
}
fn ty(package: &RcorePackage, name: &str) -> Result<RuntimeType, String> {
    let (_, declaration) = package
        .schema()
        .declarations
        .iter()
        .find(|(key, declaration)| {
            declaration.name == name && package.schema().classes.contains_key(*key)
        })
        .ok_or_else(|| format!("Missing declared class {name}"))?;
    RuntimeType::new(Type::Classifier {
        declaration: DeclarationId {
            source: declaration.source.clone(),
            path: declaration.path.clone(),
        },
        arguments: vec![],
        raw: false,
    })
}
fn model(package: &RcorePackage) -> Result<RcoreModel, String> {
    let mut model = package.new_model().map_err(|error| error.to_string())?;
    model.create_typed("type", &ty(package, "Type")?, "memory:library")?;
    model.set("type", "name", json!("first"))?;
    model.create_typed("feature", &ty(package, "Feature")?, "memory:system")?;
    model.create("literal", "Literal", "memory:library")?;
    Ok(model)
}
fn saved(package: &RcorePackage, model: &RcoreModel) -> Result<Vec<u8>, String> {
    package.save(model).map_err(|error| error.to_string())
}

#[derive(Clone, Copy)]
enum Plan {
    Valid,
    WrongTarget,
    WrongFeature,
    Missing,
    Duplicate,
    CallbackError,
    ChangedContract,
}
struct Provider {
    contract: ClassInitializationContract,
    changed: ClassInitializationContract,
    plan: Plan,
    calls: Cell<usize>,
    changed_now: Cell<bool>,
}
impl Provider {
    fn new(package: &RcorePackage, plan: Plan) -> Result<Self, String> {
        let contract = package
            .initializer_contract(&ty(package, "FeatureTyping")?)
            .map_err(|error| error.to_string())?;
        let mut changed = contract.clone();
        changed.provider.version = "foreign".into();
        Ok(Self {
            contract,
            changed,
            plan,
            calls: Cell::new(0),
            changed_now: Cell::new(false),
        })
    }
}
impl ClassInitializer for Provider {
    fn contract(&self) -> &ClassInitializationContract {
        if self.changed_now.get() {
            &self.changed
        } else {
            &self.contract
        }
    }
    fn plan(
        &self,
        store: &dyn ModelRead,
        id: &str,
        budget: &mut QueryBudget,
    ) -> Result<Vec<InitialValue>, String> {
        self.calls.set(self.calls.get() + 1);
        budget.charge()?;
        // The canonical service has created the new identity inside its atomic
        // transaction. No plan or graph value is cached by the facade.
        if store.class(id)?.as_deref() != Some(self.contract.class.as_str()) {
            return Err("Provider did not observe the actual new receiver".into());
        }
        budget.charge()?;
        let name = store
            .value("type", "name")?
            .ok_or("Missing live type name")?;
        if matches!(self.plan, Plan::CallbackError) {
            return Err("authored provider failure".into());
        }
        let target = if matches!(self.plan, Plan::WrongTarget) {
            "literal"
        } else {
            "type"
        };
        let feature = if matches!(self.plan, Plan::WrongFeature) {
            "type"
        } else {
            "feature"
        };
        let values = [json!(target), json!(feature), name];
        if self.contract.members.len() != values.len() {
            return Err("Unexpected source member contract".into());
        }
        let mut result = Vec::new();
        for (feature, value) in self.contract.members.iter().zip(values) {
            budget.charge()?;
            result.push(InitialValue {
                feature: feature.clone(),
                value,
            });
        }
        if matches!(self.plan, Plan::Missing) {
            result.pop();
        }
        if matches!(self.plan, Plan::Duplicate) {
            let first = result.first().ok_or("Missing first member")?;
            result.push(InitialValue {
                feature: first.feature.clone(),
                value: first.value.clone(),
            });
        }
        self.changed_now
            .set(matches!(self.plan, Plan::ChangedContract));
        Ok(result)
    }
}
fn create(
    package: &RcorePackage,
    model: &mut RcoreModel,
    provider: &Provider,
    id: &str,
    authored: &[(String, Value)],
    budget: &mut QueryBudget,
) -> Result<(), String> {
    package
        .create_typed_initialized(
            model,
            &ty(package, "FeatureTyping")?,
            Construction {
                id,
                resource: "memory:system",
                authored,
            },
            provider,
            None,
            budget,
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn explicit_selection_preserves_disabled_routes() -> Result<(), String> {
    let (input, entry) = bundle("memory:relationship.rcore", SOURCE);
    assert!(compile_rcore_bundle(input.clone(), &entry).is_err());
    assert!(compile_rcore_bundle_with_initializers(input.clone(), &entry, &[]).is_err());
    let wrong = ProviderId {
        name: selected().name,
        version: "2".into(),
    };
    assert!(compile_rcore_bundle_with_initializers(input, &entry, &[wrong]).is_err());
    let package = package()?;
    let mut model = model(&package)?;
    let before = saved(&package, &model)?;
    let provider = Provider::new(&package, Plan::Valid)?;
    assert_eq!(provider.calls.get(), 0);
    assert!(
        model
            .create("raw", "FeatureTyping", "memory:system")
            .is_err()
    );
    assert!(
        model
            .create_typed("raw", &ty(&package, "FeatureTyping")?, "memory:system")
            .is_err()
    );
    assert!(
        package
            .create_initialized(
                &mut model,
                "FeatureTyping",
                Construction {
                    id: "missing-provider",
                    resource: "memory:system",
                    authored: &[],
                },
                &mut QueryBudget::new(100000)
            )
            .is_err()
    );
    assert_eq!(saved(&package, &model)?, before);
    assert_eq!(provider.calls.get(), 0);
    // Existing source-bound constant initialization remains the identical
    // default implementation; this shared fixture is separately selected.
    Ok(())
}

pub fn live_relationship_plan_defaults_and_checked_edits() -> Result<(), String> {
    let package = package()?;
    let mut model = model(&package)?;
    let provider = Provider::new(&package, Plan::Valid)?;
    let authored = [("label".into(), json!("authored"))];
    create(
        &package,
        &mut model,
        &provider,
        "typing-a",
        &authored,
        &mut QueryBudget::new(100000),
    )?;
    assert_eq!(model.value("typing-a", "type")?, Some(json!("type")));
    assert_eq!(
        model.value("typing-a", "typedFeature")?,
        Some(json!("feature"))
    );
    assert_eq!(model.value("typing-a", "origin")?, Some(json!("first")));
    assert_eq!(model.value("typing-a", "label")?, Some(json!("authored")));
    model.set("type", "name", json!("updated"))?;
    create(
        &package,
        &mut model,
        &provider,
        "typing-b",
        &[],
        &mut QueryBudget::new(100000),
    )?;
    assert_eq!(model.value("typing-b", "origin")?, Some(json!("updated")));
    assert_eq!(
        model.value("typing-b", "label")?,
        Some(json!("schema-label"))
    );
    assert_eq!(provider.calls.get(), 2);
    let before = saved(&package, &model)?;
    assert!(model.set("typing-a", "type", json!("literal")).is_err());
    assert!(model.set("typing-a", "origin", json!("rewritten")).is_err());
    assert_eq!(saved(&package, &model)?, before);
    Ok(())
}

pub fn invalid_plans_and_authored_values_roll_back_whole_graph() -> Result<(), String> {
    let package = package()?;
    let mut model = model(&package)?;
    let before = saved(&package, &model)?;
    for plan in [
        Plan::WrongTarget,
        Plan::WrongFeature,
        Plan::Missing,
        Plan::Duplicate,
        Plan::CallbackError,
        Plan::ChangedContract,
    ] {
        let provider = Provider::new(&package, plan)?;
        assert!(
            create(
                &package,
                &mut model,
                &provider,
                "rejected",
                &[],
                &mut QueryBudget::new(100000)
            )
            .is_err()
        );
        assert_eq!(provider.calls.get(), 1);
        assert_eq!(saved(&package, &model)?, before);
        assert_eq!(model.class("rejected")?, None);
    }
    let provider = Provider::new(&package, Plan::Valid)?;
    for authored in [
        vec![("origin".into(), json!("forbidden"))],
        vec![("label".into(), json!(42))],
        vec![
            ("label".into(), json!("one")),
            ("label".into(), json!("two")),
        ],
    ] {
        assert!(
            create(
                &package,
                &mut model,
                &provider,
                "rejected",
                &authored,
                &mut QueryBudget::new(100000)
            )
            .is_err()
        );
        assert_eq!(saved(&package, &model)?, before);
    }
    create(
        &package,
        &mut model,
        &provider,
        "fresh",
        &[],
        &mut QueryBudget::new(100000),
    )?;
    Ok(())
}

pub fn exact_short_zero_and_preconsumed_use_original_ledger() -> Result<(), String> {
    let package = package()?;
    let mut successful = model(&package)?;
    let provider = Provider::new(&package, Plan::Valid)?;
    let mut measured = QueryBudget::new(100000);
    create(
        &package,
        &mut successful,
        &provider,
        "typing",
        &[],
        &mut measured,
    )?;
    let exact = measured.reads;
    assert!(exact > 5);
    let expected = saved(&package, &successful)?;
    for amount in [0, exact - 1] {
        let mut denied = model(&package)?;
        let baseline = saved(&package, &denied)?;
        let provider = Provider::new(&package, Plan::Valid)?;
        let mut budget = QueryBudget::new(amount);
        assert!(create(&package, &mut denied, &provider, "typing", &[], &mut budget).is_err());
        assert_eq!(budget.remaining(), 0);
        assert_eq!(budget.reads, amount);
        assert_eq!(saved(&package, &denied)?, baseline);
    }
    for consumed in [0, 3] {
        let mut fresh = model(&package)?;
        let provider = Provider::new(&package, Plan::Valid)?;
        let mut budget = QueryBudget::new(exact + consumed);
        for _ in 0..consumed {
            budget.charge()?;
        }
        create(&package, &mut fresh, &provider, "typing", &[], &mut budget)?;
        assert_eq!(budget.reads, exact + consumed);
        assert_eq!(budget.remaining(), 0);
        assert_eq!(saved(&package, &fresh)?, expected);
    }
    println!(
        "DIRECT_RCORE_INITIALIZER_LEDGER {}",
        json!({"original_limit":100000,"measured_charges":exact,"preconsumed":3})
    );
    Ok(())
}

pub fn reload_requires_explicit_selection_and_revalidates_sources() -> Result<(), String> {
    let package = package()?;
    let mut model = model(&package)?;
    let provider = Provider::new(&package, Plan::Valid)?;
    create(
        &package,
        &mut model,
        &provider,
        "typing",
        &[],
        &mut QueryBudget::new(100000),
    )?;
    let bytes = saved(&package, &model)?;
    assert!(RcorePackage::reload(&bytes).is_err());
    assert!(RcorePackage::reload_with_initializers(&bytes, &[]).is_err());
    let (reloaded, mut restored) = RcorePackage::reload_with_initializers(&bytes, &[selected()])
        .map_err(|error| error.to_string())?;
    assert_eq!(saved(&reloaded, &restored)?, bytes);
    assert!(
        restored
            .create("raw-after-reload", "FeatureTyping", "memory:system")
            .is_err()
    );
    let supplied_again = Provider::new(&reloaded, Plan::Valid)?;
    create(
        &reloaded,
        &mut restored,
        &supplied_again,
        "new-after-reload",
        &[],
        &mut QueryBudget::new(100000),
    )?;
    assert_eq!(supplied_again.calls.get(), 1);
    let mut stale: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    stale["bundle"]["sources"][0]["text"] = json!(format!("{SOURCE}\n"));
    assert!(
        RcorePackage::reload_with_initializers(
            &serde_json::to_vec(&stale).map_err(|error| error.to_string())?,
            &[selected()]
        )
        .is_err()
    );
    let mut forged: Value = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let elements = forged["model"]["elements"]
        .as_array_mut()
        .ok_or("Missing saved elements")?;
    let typing = elements
        .iter_mut()
        .find(|element| element["id"] == "typing")
        .ok_or("Missing saved typing")?;
    typing["initialization"]["contract"]["provider"]["version"] = json!("forged");
    assert!(
        RcorePackage::reload_with_initializers(
            &serde_json::to_vec(&forged).map_err(|error| error.to_string())?,
            &[selected()]
        )
        .is_err()
    );
    Ok(())
}

pub fn exact_runtime_type_and_foreign_contract_boundaries() -> Result<(), String> {
    let package = package()?;
    let types = ["Type", "Feature", "Literal", "FeatureTyping"]
        .iter()
        .map(|name| ty(&package, name))
        .collect::<Result<Vec<_>, _>>()?;
    let (input, entry) = bundle("memory:relationship.rcore", SOURCE);
    let selected_package = compile_rcore_bundle_selected_with_initializers(
        input,
        &entry,
        "urn:direct-relationship",
        &types,
        &[selected()],
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(selected_package.schema(), package.schema());
    assert!(
        selected_package
            .initializer_contract(&ty(&package, "Type")?)
            .is_err()
    );
    let mut model = model(&selected_package)?;
    let before = saved(&selected_package, &model)?;
    let (foreign, entry) = bundle("memory:other-relationship.rcore", SOURCE);
    let foreign = compile_rcore_bundle_with_initializers(foreign, &entry, &[selected()])
        .map_err(|error| error.to_string())?;
    let foreign_provider = Provider::new(&foreign, Plan::Valid)?;
    assert!(
        create(
            &selected_package,
            &mut model,
            &foreign_provider,
            "foreign",
            &[],
            &mut QueryBudget::new(100000)
        )
        .is_err()
    );
    assert_eq!(foreign_provider.calls.get(), 0);
    assert_eq!(saved(&selected_package, &model)?, before);
    let provider = Provider::new(&selected_package, Plan::Valid)?;
    let foreign_ty = ty(&foreign, "FeatureTyping")?;
    assert!(
        selected_package
            .create_typed_initialized(
                &mut model,
                &foreign_ty,
                Construction {
                    id: "foreign-type",
                    resource: "memory:system",
                    authored: &[],
                },
                &provider,
                None,
                &mut QueryBudget::new(100000)
            )
            .is_err()
    );
    assert_eq!(provider.calls.get(), 0);
    assert_eq!(saved(&selected_package, &model)?, before);
    Ok(())
}
