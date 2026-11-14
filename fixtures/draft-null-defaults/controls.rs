//! Authored optional-null physical constructor controls; no language award.
use intersect_model::{
    compile_rcore_at,
    core::{
        frozen::FrozenStore,
        model::Element,
        query::QueryBudget,
        storage::{
            self, Construction, DraftClassContract, DraftConstructionContext, DraftDisposition,
            DraftMemberDisposition, DraftPhysicalDefaults,
        },
    },
};
use serde_json::{Value, json};
struct Policy(Vec<DraftClassContract>);
impl DraftPhysicalDefaults for Policy {
    fn contracts(&self) -> &[DraftClassContract] {
        &self.0
    }
}
fn require(ok: bool, message: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(message.into()) }
}
pub fn verify() -> Result<Value, String> {
    let package = compile_rcore_at(
        "memory:draft-null.rcore",
        r#"rcore 1; package nullDraft version "1";
      metamodel "urn:draft-null" prefix n {
       enum Mode { Day = 1; Night = 2; }
       class Item {
        attr optional: Mode [0,1] default "Day";
        attr omitted: Mode [0,1] default "Day";
        attr requiredValue: Mode [1,1] default "Day";
        attr many: Mode [0,*];
        attr computed: Mode [0,1] derived;
        attr alreadyNull: EString [0,1];
        ref peer: Item [0,1];
       }
      }"#,
    )
    .map_err(|e| e.to_string())?;
    let schema = package.schema();
    let source = &package
        .sources()
        .sources
        .first()
        .ok_or("Missing authored source")?
        .id;
    let class = schema.class_key(source, "urn:draft-null", "Item")?;
    let members = schema
        .effective_features(class)?
        .into_iter()
        .filter(|f| !f.feature.derived)
        .map(|f| DraftMemberDisposition {
            name: f.feature.name.clone(),
            identity: f.identity,
            disposition: match f.feature.name.as_str() {
                "optional" => DraftDisposition::StoredNullDefault,
                "omitted" => DraftDisposition::DelegatedAbsent,
                _ => DraftDisposition::StoredDefault,
            },
        })
        .collect();
    let contract = DraftClassContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class: class.into(),
        class_identity: schema.identity(class),
        sources: package
            .sources()
            .sources
            .iter()
            .map(|s| s.id.clone())
            .collect(),
        members,
    };
    let policy = Policy(vec![contract.clone()]);
    let mut binding = QueryBudget::new(1000);
    let context = DraftConstructionContext::bind(schema, package.sources(), &policy, &mut binding)?;
    let optional = schema.effective_feature(class, "optional")?;
    let schema_default = optional.feature.default.clone();
    require(!schema_default.is_null(), "Imported enum default was lost")?;
    schema.validate_attribute(optional.feature, &schema_default)?;
    require(
        schema
            .validate_attribute(optional.feature, &Value::Null)
            .err()
            .as_deref()
            == Some("Null is absence, not a present attribute value"),
        "Null was treated as an enum codec payload",
    )?;
    require(
        schema
            .validate_attribute(optional.feature, &json!("invalid raw enum"))
            .is_err(),
        "Enum codec was bypassed",
    )?;
    let legacy = Element::new(schema, "old", class, "memory:old", 0, 1)?;
    require(
        legacy.values.get("optional") == Some(&schema_default),
        "Legacy factory default changed",
    )?;
    let mut outcomes = Vec::new();
    for limit in [0, 1, 2] {
        let mut store = FrozenStore::for_schema(schema).map_err(|e| e.to_string())?;
        let before = storage::export(&store)?;
        let mut work = QueryBudget::new(limit);
        let result = storage::atomic(&mut store, |store| {
            context.create(
                store,
                Construction {
                    id: "a",
                    resource: "memory:draft-null",
                    authored: &[],
                },
                class,
                0,
                1,
                &mut work,
            )
        });
        let after = storage::export(&store)?;
        if limit < 2 {
            require(
                result.is_err() && work.reads == limit && after == before,
                "Null override exhaustion failed rollback/debit",
            )?;
        } else {
            result.as_ref().map_err(Clone::clone)?;
            require(
                work.reads == 2 && after.elements.len() == 1,
                "Exact constructor logical work changed",
            )?;
            let values = &after.elements[0].values;
            require(
                values.contains_key("optional")
                    && values.get("optional") == Some(&Value::Null)
                    && !values.contains_key("omitted"),
                "Physical null and omitted disposition collapsed",
            )?;
            require(
                values.get("requiredValue") == Some(&schema_default),
                "Unrelated schema default changed",
            )?;
            storage::validate_draft(&store, schema)?;
            context.observe(&store, "a", &mut QueryBudget::new(100))?;
        }
        if limit < 2 {
            storage::atomic(&mut store, |store| {
                context.create(
                    store,
                    Construction {
                        id: "a",
                        resource: "memory:draft-null",
                        authored: &[],
                    },
                    class,
                    0,
                    1,
                    &mut QueryBudget::new(2),
                )
            })?;
            require(
                storage::export(&store)?.elements[0].values.get("optional") == Some(&Value::Null),
                "Constructor failed recovery",
            )?;
        }
        outcomes.push(json!({"grant":limit,"work":work.reads,"result":result,"state_before":before,"state_after":after}));
    }
    let mut forbidden = Vec::new();
    for field in ["requiredValue", "peer", "many", "alreadyNull"] {
        let mut bad = contract.clone();
        bad.members
            .iter_mut()
            .find(|m| m.name == field)
            .ok_or("Missing invalid field")?
            .disposition = DraftDisposition::StoredNullDefault;
        let bad_policy = Policy(vec![bad]);
        let error = DraftConstructionContext::bind(
            schema,
            package.sources(),
            &bad_policy,
            &mut QueryBudget::new(1000),
        )
        .err()
        .ok_or("Invalid null override admitted")?;
        require(
            error.contains("optional single attribute"),
            "Wrong invalid-null rejection",
        )?;
        forbidden.push(json!({"field":field,"error":error}));
    }
    let mut derived = contract.clone();
    derived.members.push(DraftMemberDisposition {
        name: "computed".into(),
        identity: schema.effective_feature(class, "computed")?.identity,
        disposition: DraftDisposition::StoredNullDefault,
    });
    let derived_policy = Policy(vec![derived]);
    require(
        DraftConstructionContext::bind(
            schema,
            package.sources(),
            &derived_policy,
            &mut QueryBudget::new(1000),
        )
        .is_err(),
        "Derived null override admitted",
    )?;
    let mut wrong = contract.clone();
    wrong
        .members
        .iter_mut()
        .find(|m| m.name == "optional")
        .ok_or("Missing optional")?
        .identity = schema.effective_feature(class, "omitted")?.identity;
    let wrong_policy = Policy(vec![wrong]);
    require(
        DraftConstructionContext::bind(
            schema,
            package.sources(),
            &wrong_policy,
            &mut QueryBudget::new(1000),
        )
        .is_err(),
        "Wrong null member identity admitted",
    )?;
    let mut stale = contract.clone();
    stale.schema_revision = "stale".into();
    let stale_policy = Policy(vec![stale]);
    require(
        DraftConstructionContext::bind(
            schema,
            package.sources(),
            &stale_policy,
            &mut QueryBudget::new(1000),
        )
        .is_err(),
        "Stale null source/schema binding admitted",
    )?;
    let mut wrong_source = contract.clone();
    wrong_source.sources[0].sha256 = "0".repeat(64);
    let wrong_source_policy = Policy(vec![wrong_source]);
    require(
        DraftConstructionContext::bind(
            schema,
            package.sources(),
            &wrong_source_policy,
            &mut QueryBudget::new(1000),
        )
        .is_err(),
        "Wrong null source admitted",
    )?;
    let mut store = FrozenStore::for_schema(schema).map_err(|e| e.to_string())?;
    storage::atomic(&mut store, |store| {
        context.create(
            store,
            Construction {
                id: "a",
                resource: "memory:draft-null",
                authored: &[],
            },
            class,
            0,
            1,
            &mut QueryBudget::new(2),
        )
    })?;
    let mut wrong_actual = storage::export(&store)?;
    wrong_actual.elements[0].type_identity = "forged actual".into();
    let wrong_store = storage::ModelView::new(&wrong_actual)?;
    require(
        context
            .observe(&wrong_store, "a", &mut QueryBudget::new(100))
            .is_err(),
        "Forged null receiver admitted",
    )?;
    Ok(
        json!({"source":source,"binding_work":binding.reads,"schema_enum_default_unchanged":schema_default,
      "contract":contract,"constructor_cases":outcomes,"forbidden_overrides":forbidden,
      "null_is_optional_absence_not_codec_payload":true,"physical_default_only":true,
      "whole_physical_work_bounded":false,"language_validated":false,"publication_permitted":false}),
    )
}
