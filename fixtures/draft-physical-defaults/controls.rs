//! Authored low-level construction controls, shared with raw WASM consumers.
use intersect_model::{
    compile_rcore_at,
    core::{
        model::Element,
        query::QueryBudget,
        storage::{
            self, Construction, DraftClassContract, DraftConstructionContext, DraftDisposition,
            DraftMemberDisposition, DraftPhysicalDefaults, ModelRead, ModelStore, ObjectView,
            Query, Row, StorageResult,
        },
    },
};
use serde_json::{Value, json};
use std::cell::RefCell;

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
struct Policy(Vec<DraftClassContract>);
impl DraftPhysicalDefaults for Policy {
    fn contracts(&self) -> &[DraftClassContract] {
        &self.0
    }
}
struct Observed<'a> {
    inner: &'a dyn storage::ModelRead,
    stale: bool,
    calls: RefCell<Vec<&'static str>>,
}
impl ModelRead for Observed<'_> {
    fn revision(&self) -> StorageResult<String> {
        self.calls.borrow_mut().push("revision");
        if self.stale {
            Ok("stale".into())
        } else {
            self.inner.revision()
        }
    }
    fn get(&self, id: &str) -> StorageResult<Option<Element>> {
        self.calls.borrow_mut().push("get");
        self.inner.get(id)
    }
    fn class(&self, id: &str) -> StorageResult<Option<String>> {
        self.calls.borrow_mut().push("class");
        self.inner.class(id)
    }
    fn page(&self, after: Option<u64>, limit: usize) -> StorageResult<Vec<Row>> {
        self.calls.borrow_mut().push("page");
        self.inner.page(after, limit)
    }
    fn query(&self, q: Query<'_>) -> StorageResult<Vec<String>> {
        self.calls.borrow_mut().push("query");
        self.inner.query(q)
    }
    fn matches(&self, id: &str, q: Query<'_>) -> StorageResult<bool> {
        self.calls.borrow_mut().push("matches");
        self.inner.matches(id, q)
    }
    fn inspect(
        &self,
        id: &str,
        f: &mut dyn FnMut(ObjectView<'_>) -> StorageResult<()>,
    ) -> StorageResult<bool> {
        self.calls.borrow_mut().push("inspect");
        self.inner.inspect(id, f)
    }
}
pub fn verify() -> Result<Value, String> {
    let package = compile_rcore_at(
        "memory:draft-physical.rcore",
        r#"rcore 1; package draftPhysical version "1";
        metamodel "urn:draft-physical" prefix d { class Item {
            attr backing: String [0,1] default "stored";
            attr requiredValue: String [1,1];
        } }"#,
    )
    .map_err(|error| error.to_string())?;
    let schema = package.schema();
    let entry = &package
        .sources()
        .sources
        .first()
        .ok_or("Missing authored source")?
        .id;
    let class = schema.class_key(entry, "urn:draft-physical", "Item")?;
    let contract = DraftClassContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class: class.into(),
        class_identity: schema.identity(class),
        sources: package
            .sources()
            .sources
            .iter()
            .map(|source| source.id.clone())
            .collect(),
        members: vec![
            DraftMemberDisposition {
                name: "backing".into(),
                identity: schema.effective_feature(class, "backing")?.identity,
                disposition: DraftDisposition::StoredDefault,
            },
            DraftMemberDisposition {
                name: "requiredValue".into(),
                identity: schema.effective_feature(class, "requiredValue")?.identity,
                disposition: DraftDisposition::DelegatedAbsent,
            },
        ],
    };
    let policy = Policy(vec![contract.clone()]);
    let context = DraftConstructionContext::bind(
        schema,
        package.sources(),
        &policy,
        &mut QueryBudget::new(1000),
    )?;
    let mut store =
        intersect_model::frozen::FrozenStore::for_schema(schema).map_err(|e| e.to_string())?;
    storage::atomic(&mut store, |store| {
        context.create(
            store,
            Construction {
                id: "a",
                resource: "memory:draft",
                authored: &[],
            },
            class,
            0,
            1,
            &mut QueryBudget::new(1000),
        )
    })?;
    let baseline = storage::export(&store)?;
    require(
        baseline.elements.len() == 1
            && baseline.elements[0].values.get("backing") == Some(&json!("stored"))
            && !baseline.elements[0].values.contains_key("requiredValue"),
        "Physical dispositions were not applied by the shared factory",
    )?;
    storage::validate_draft(&store, schema)?;
    require(
        storage::validate(&store, schema).is_err(),
        "Raw full lower-bound contract changed",
    )?;
    let observed = Observed {
        inner: &store,
        stale: false,
        calls: RefCell::new(Vec::new()),
    };
    let mut zero = QueryBudget::new(0);
    require(
        context.observe(&observed, "a", &mut zero).err().as_deref()
            == Some("Query work budget exhausted; no complete result")
            && zero.reads == 0
            && observed.calls.borrow().is_empty(),
        "Zero observation budget called storage",
    )?;
    let stale = Observed {
        inner: &store,
        stale: true,
        calls: RefCell::new(Vec::new()),
    };
    let mut stale_work = QueryBudget::new(10);
    require(
        context
            .observe(&stale, "a", &mut stale_work)
            .err()
            .as_deref()
            == Some("Draft observation store revision mismatch")
            && stale_work.reads == 1
            && *stale.calls.borrow() == ["revision"],
        "Stale revision reached object inspection or emitted a receipt",
    )?;
    let mut exact = QueryBudget::new(4);
    let receipt = context.observe(&observed, "a", &mut exact)?;
    require(
        exact.reads == 4
            && *observed.calls.borrow() == ["revision", "inspect"]
            && receipt.object == "a",
        "Exact observation work/callback result changed",
    )?;
    observed.calls.borrow_mut().clear();
    let mut short = QueryBudget::new(3);
    require(
        context.observe(&observed, "a", &mut short).err().as_deref()
            == Some("Query work budget exhausted; no complete result")
            && short.reads == 3
            && *observed.calls.borrow() == ["revision", "inspect"],
        "One-short observation callback/work contract changed",
    )?;
    let mut missing = contract.clone();
    missing.members.pop();
    require(
        DraftConstructionContext::bind(
            schema,
            package.sources(),
            &Policy(vec![missing]),
            &mut QueryBudget::new(1000),
        )
        .is_err(),
        "Missing disposition was accepted",
    )?;
    let mut wrong = contract;
    wrong.members[0].identity = schema.effective_feature(class, "requiredValue")?.identity;
    require(
        DraftConstructionContext::bind(
            schema,
            package.sources(),
            &Policy(vec![wrong]),
            &mut QueryBudget::new(1000),
        )
        .is_err(),
        "Wrong member identity was accepted",
    )?;
    let outcome = storage::atomic(&mut store, |store| {
        store.set_value("a", "requiredValue", Some(json!("fake raw storage")))?;
        context
            .observe(store, "a", &mut QueryBudget::new(1000))
            .map(|_| ())
    });
    require(
        outcome.err().as_deref() == Some("Draft observation physical member disposition mismatch")
            && storage::export(&store)? == baseline,
        "Adversarial delegated slot was accepted or failed rollback",
    )?;
    require(
        context
            .observe(&store, "a", &mut QueryBudget::new(4))?
            .object
            == "a",
        "Observation did not recover after failures",
    )?;
    Ok(
        json!({"state":"authored_physical_draft_controls_only","exact_work":exact.reads,"short_work":short.reads,
        "exact_callbacks":["revision","inspect"],"zero_work":zero.reads,"stale_work":stale_work.reads,
        "receipt":receipt,"model":baseline,"language_semantics_verified":false}),
    )
}
