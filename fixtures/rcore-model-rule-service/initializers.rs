//! Same direct source-bound atomic constructor controls in native and WASM.
use intersect_model::{
    api::ModelEdit,
    compile_rcore_at,
    query::QueryBudget,
    rcore::RcorePackage,
    storage::{Construction, ModelRead},
};
use serde_json::json;

fn require(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.into()) }
}
const SOURCE: &str = r#"rcore 1; package directInit version "1";
metamodel "urn:direct-init" prefix d {
 class Item {
   attr label: String [1,1] default "schema";
   attr origin: String [1,1] readonly default "schema-origin";
 }
}
behavior {
 import model "urn:direct-init" as d;
 initialize d.Item using "intersect.constant-attributes" version "1" {
   label = "constructor"; origin = "constructed-origin";
 }
}"#;

pub fn verify() -> Result<(), String> {
    let package =
        compile_rcore_at("memory:direct-init.rcore", SOURCE).map_err(|error| error.to_string())?;
    let mut model = package.new_model().map_err(|error| error.to_string())?;
    let empty = package.save(&model).map_err(|error| error.to_string())?;
    require(
        model.create("plain", "Item", "memory:initialized").is_err(),
        "Ordinary creation silently bypassed a required initializer",
    )?;
    require(
        package.save(&model).map_err(|error| error.to_string())? == empty,
        "Rejected ordinary creation mutated model",
    )?;
    let authored = [("label".to_owned(), json!("authored"))];
    let mut successful_budget = QueryBudget::new(1000);
    let receipt = package
        .create_initialized(
            &mut model,
            "Item",
            Construction {
                id: "a",
                resource: "memory:initialized",
                authored: &authored,
            },
            &mut successful_budget,
        )
        .map_err(|error| error.to_string())?;
    require(
        model.value("a", "label")? == Some(json!("authored"))
            && model.value("a", "origin")? == Some(json!("constructed-origin")),
        "Constructor/authored value order changed",
    )?;
    require(
        receipt.object == "a"
            && receipt.contract.sources
                == package
                    .sources()
                    .sources
                    .iter()
                    .map(|source| source.id.clone())
                    .collect::<Vec<_>>()
            && receipt.contract.provider.name == "intersect.constant-attributes"
            && receipt.contract.provider.version == "1"
            && receipt.contract.members.iter().all(|member| match member {
                intersect_model::definitions::schema::MemberIdentity::Source { declaration } => {
                    declaration.source == *package.entry()
                }
                _ => false,
            })
            && receipt.authored_members
                == vec![
                    package
                        .schema()
                        .effective_feature(&receipt.contract.class, "label")?
                        .identity,
                ],
        "Initialization receipt lost source/member identity or authored order",
    )?;
    let baseline = package.save(&model).map_err(|error| error.to_string())?;
    let readonly = [("origin".to_owned(), json!("forbidden"))];
    let wrong_type = [("label".to_owned(), json!(42))];
    for (id, authored, budget) in [
        ("readonly", readonly.as_slice(), 1000),
        ("wrong-type", wrong_type.as_slice(), 1000),
        ("budget", authored.as_slice(), 0),
        (
            "late-budget",
            authored.as_slice(),
            successful_budget.reads.saturating_sub(1),
        ),
        ("a", authored.as_slice(), 1000),
    ] {
        require(
            package
                .create_initialized(
                    &mut model,
                    "Item",
                    Construction {
                        id,
                        resource: "memory:initialized",
                        authored,
                    },
                    &mut QueryBudget::new(budget),
                )
                .is_err(),
            "Invalid constructor request succeeded",
        )?;
        require(
            package.save(&model).map_err(|error| error.to_string())? == baseline,
            "Failed constructor did not roll back exact model/resources/receipts",
        )?;
    }
    let mut tampered: serde_json::Value =
        serde_json::from_slice(&baseline).map_err(|error| error.to_string())?;
    tampered["model"]["elements"][0]["initialization"]["contract"]["provider"]["name"] =
        json!("host.unselected");
    require(
        RcorePackage::reload(&serde_json::to_vec(&tampered).map_err(|error| error.to_string())?)
            .is_err(),
        "Serialized receipt authorized a different provider",
    )?;
    let (reloaded, mut restored) =
        RcorePackage::reload(&baseline).map_err(|error| error.to_string())?;
    require(
        reloaded
            .save(&restored)
            .map_err(|error| error.to_string())?
            == baseline,
        "Reload changed exact source-bound initialization evidence",
    )?;
    require(
        restored
            .create("plain-reloaded", "Item", "memory:initialized")
            .is_err(),
        "Reload lost required-constructor default-creation guard",
    )?;
    reloaded
        .create_initialized(
            &mut restored,
            "Item",
            Construction {
                id: "after-reload",
                resource: "memory:initialized",
                authored: &[],
            },
            &mut QueryBudget::new(1000),
        )
        .map_err(|error| error.to_string())?;
    require(
        restored.value("after-reload", "label")? == Some(json!("constructor")),
        "Reload did not rebuild constructor behavior from source",
    )?;
    let foreign = compile_rcore_at("memory:foreign-direct-init.rcore", SOURCE)
        .map_err(|error| error.to_string())?;
    require(
        foreign
            .create_initialized(
                &mut model,
                "Item",
                Construction {
                    id: "foreign",
                    resource: "memory:initialized",
                    authored: &[],
                },
                &mut QueryBudget::new(1000),
            )
            .is_err(),
        "Foreign source schema admitted",
    )?;
    let unsupported = SOURCE.replace("intersect.constant-attributes", "host.external");
    require(
        compile_rcore_at("memory:external-init.rcore", &unsupported).is_err(),
        "An undeclared external provider was activated",
    )?;
    // The raw checked API owns structure, not package constructor selection.
    // The package boundary must reject a raw object without its exact receipt.
    let key = receipt.contract.class.clone();
    ModelEdit::create(&mut *model, "raw", &key, "memory:initialized")?;
    require(
        package.save(&model).is_err(),
        "Package accepted a missing initializer receipt",
    )?;
    Ok(())
}
