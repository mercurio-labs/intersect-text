//! Authored controls for the new read boundary; no imported-language support claim.
use intersect_model::definitions::preparation::PreparedModelDefinition;
use intersect_model::{
    core::{
        frozen::{FrozenStore, Metamodel},
        query::*,
        storage::{ModelRead, ModelStore, Query, Row, StorageResult},
    },
    definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceId, SourceKind},
        ecore_migration::BundleMigrationReport,
        model_definition::ModelCompilation,
        schema::{MemberIdentity, Schema},
    },
    semantic_reads::{
        ReadDependencies, ReadMember, ReadPlan, SemanticReadPolicy, SemanticReadSession,
    },
};
use serde_json::{Value, json};
use std::cell::RefCell;

pub fn inputs() -> Result<(ModelCompilation, BundleMigrationReport), String> {
    let ecore = DefinitionSource::new(
        "memory:read/model.ecore",
        SourceKind::Ecore,
        r##"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="reads" nsURI="urn:reads" nsPrefix="r">
<eClassifiers xsi:type="ecore:EClass" name="Actor">
<eStructuralFeatures xsi:type="ecore:EAttribute" name="backing" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures xsi:type="ecore:EAttribute" name="label" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures xsi:type="ecore:EAttribute" name="backingList" upperBound="-1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures xsi:type="ecore:EAttribute" name="labels" upperBound="-1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures xsi:type="ecore:EAttribute" name="derivedLabel" derived="true" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures xsi:type="ecore:EAttribute" name="secret" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures xsi:type="ecore:EAttribute" name="brokenGetter" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures xsi:type="ecore:EAttribute" name="undeclaredGetter" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
<eStructuralFeatures xsi:type="ecore:EReference" name="link" eType="#//Actor"/>
</eClassifiers><eClassifiers xsi:type="ecore:EClass" name="Special" eSuperTypes="#//Actor"/>
</ecore:EPackage>"##,
    );
    let mut behavior = DefinitionSource::new(
        "memory:read/behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package readRules version \"1\";",
    );
    behavior.dependencies = vec![ecore.id.clone()];
    let original = ModelCompilation {
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), behavior.clone()],
            namespaces: vec![],
        },
        structure: ecore.id.clone(),
        behavior: behavior.id,
        namespace: "urn:reads".into(),
        materialized_types: vec![],
    };
    let origin = intersect_model::migration::migrate_ecore_bundle(
        original.bundle.clone(),
        &[(ecore.id, "memory:read/model.rcore".into())].into(),
    )
    .map_err(|e| e.to_string())?;
    Ok((origin.remap_model_compilation(&original)?, origin))
}
pub fn prepare<'a>(
    input: &'a ModelCompilation,
    origin: &'a BundleMigrationReport,
) -> Result<PreparedModelDefinition<'a>, String> {
    use intersect_model::definitions::{
        declarations::DeclarationCollection,
        ecore::ParseLimits,
        model_definition::{
            CompiledStructure, ModelPreparationMode, PhysicalInputCapture, StructureSelection,
        },
        preparation::CheckedModelOrigin,
    };
    let limits = ParseLimits::default();
    let checked_origin = CheckedModelOrigin::check(&input.bundle, origin, limits)?;
    CompiledStructure::prepare(
        StructureSelection {
            bundle: &input.bundle,
            entry: &input.structure,
            namespace: &input.namespace,
            materialized_types: &input.materialized_types,
        },
        DeclarationCollection::Classifiers,
        PhysicalInputCapture::None,
        None,
        limits,
    )
    .map_err(|error| error.to_string())?
    .check_behavior(&input.behavior, None, limits)
    .and_then(|checked| checked.finish(limits, false, ModelPreparationMode::Inspection, &[]))
    .map_err(|errors| format!("{errors:?}"))?
    .into_prepared_definition()
    .with_origin(checked_origin)
}
pub struct Policy {
    pub source: SourceId,
    pub stored_base: bool,
    pub reject_special: bool,
}
impl SemanticReadPolicy for Policy {
    fn validate(&self, definition: &PreparedModelDefinition<'_>) -> QueryResult<()> {
        if *definition.structure_selection().entry != self.source {
            return Err("Authored read source mismatch".into());
        }
        Ok(())
    }
    fn plan(&self, member: ReadMember<'_>) -> ReadPlan {
        if !matches!(member.identity,MemberIdentity::Source{declaration} if declaration.source==self.source)
            || (member.actual == "Special" && self.reject_special)
        {
            return ReadPlan::Unsupported;
        }
        match member.feature.name.as_str() {
            "backing" | "backingList" | "link" => ReadPlan::Stored,
            "label" if self.stored_base && member.actual == "Actor" => ReadPlan::Stored,
            "label" | "labels" | "brokenGetter" | "undeclaredGetter" => ReadPlan::Delegated,
            "derivedLabel" => ReadPlan::Derived,
            _ => ReadPlan::Unsupported,
        }
    }
    fn dependencies(&self, member: ReadMember<'_>) -> &[&str] {
        match member.feature.name.as_str() {
            "label" if self.plan(member) == ReadPlan::Stored => &[],
            "label" | "derivedLabel" => &["backing"],
            "labels" => &["backingList"],
            "brokenGetter" => &["secret"],
            _ => &[],
        }
    }
    fn read(
        &self,
        member: ReadMember<'_>,
        dependencies: &dyn ReadDependencies,
        budget: &mut QueryBudget,
    ) -> QueryResult<Value> {
        match member.feature.name.as_str() {
            "label" | "derivedLabel" => dependencies.read("backing", budget),
            "labels" => dependencies.read("backingList", budget),
            "brokenGetter" | "undeclaredGetter" => dependencies.read("secret", budget),
            _ => Err("No authored getter".into()),
        }
    }
}
pub fn store(schema: &Schema) -> Result<FrozenStore, String> {
    store_with_policy(
        schema,
        intersect_model::core::storage::IndexPolicy::AllValues,
    )
}
fn store_with_policy(
    schema: &Schema,
    policy: intersect_model::core::storage::IndexPolicy,
) -> Result<FrozenStore, String> {
    let metamodel = Metamodel::freeze(schema).map_err(|e| e.to_string())?;
    let mut store = FrozenStore::with_index_policy(std::sync::Arc::new(metamodel), policy);
    intersect_model::core::storage::atomic(&mut store, |s| {
        for (id, class) in [("a", "Actor"), ("b", "Special")] {
            s.create_element(schema, id, class, "memory:model", 0, 1)?;
            s.set_value(id, "backing", Some(json!("semantic")))?;
            s.set_value(id, "backingList", Some(json!(["one", "two"])))?;
            s.set_value(id, "label", Some(json!("conflicting raw")))?;
            s.set_value(id, "labels", Some(json!(["raw"])))?;
            s.set_value(id, "secret", Some(json!("must not leak")))?;
            s.set_value(id, "link", Some(Value::Null))?;
        }
        s.set_value("a", "link", Some(json!("b")))?;
        Ok(())
    })?;
    Ok(store)
}
pub struct Trace<'a> {
    pub inner: &'a dyn ModelRead,
    pub calls: RefCell<Vec<String>>,
}
impl Trace<'_> {
    fn note(&self, value: String) {
        self.calls.borrow_mut().push(value);
    }
}
impl ModelRead for Trace<'_> {
    fn revision(&self) -> StorageResult<String> {
        self.note("revision".into());
        self.inner.revision()
    }
    fn get(&self, id: &str) -> StorageResult<Option<intersect_model::core::model::Element>> {
        self.note(format!("get:{id}"));
        self.inner.get(id)
    }
    fn class(&self, id: &str) -> StorageResult<Option<String>> {
        self.note(format!("class:{id}"));
        self.inner.class(id)
    }
    fn page(&self, after: Option<u64>, limit: usize) -> StorageResult<Vec<Row>> {
        self.note(format!("page:{after:?}:{limit}"));
        self.inner.page(after, limit)
    }
    fn query(&self, query: Query<'_>) -> StorageResult<Vec<String>> {
        self.note(match query {
            Query::Class(c) => format!("query-class:{c}"),
            Query::Resource(r) => format!("query-resource:{r}"),
            Query::Value(k, v) => format!("query-value:{k}:{v}"),
        });
        self.inner.query(query)
    }
    fn matches(&self, id: &str, query: Query<'_>) -> StorageResult<bool> {
        self.note(format!("matches:{id}"));
        self.inner.matches(id, query)
    }
    fn value(&self, id: &str, name: &str) -> StorageResult<Option<Value>> {
        self.note(format!("value:{id}:{name}"));
        self.inner.value(id, name)
    }
    fn value_len(&self, id: &str, name: &str) -> StorageResult<Option<usize>> {
        self.note(format!("length:{id}:{name}"));
        self.inner.value_len(id, name)
    }
}
pub fn plan(schema: &Schema, path: &str, count: bool) -> QueryPlan {
    QueryPlan {
        schema_uri: schema.uri.clone(),
        schema_revision: schema.revision.clone(),
        from: "Actor".into(),
        filter: None,
        select: vec![Column {
            name: "answer".into(),
            expression: if count {
                Expression::Count { path: path.into() }
            } else {
                Expression::Field { path: path.into() }
            },
        }],
        order_by: vec![],
        limit: 10,
    }
}
pub fn delegated_value_length_projection_navigation_and_count_share_policy() -> Result<(), String> {
    let (input, origin) = inputs()?;
    let prepared = prepare(&input, &origin)?;
    let store = store(prepared.schema())?;
    let policy = Policy {
        source: input.structure.clone(),
        stored_base: false,
        reject_special: false,
    };
    let session = SemanticReadSession::new(&prepared, &store, &policy)?;
    let mut budget = QueryBudget::new(1_000_000);
    assert_eq!(
        session.value("Actor", "a", "label", &mut budget)?,
        json!("semantic")
    );
    assert_eq!(
        session.value("Actor", "a", "derivedLabel", &mut budget)?,
        json!("semantic")
    );
    assert_eq!(session.value_len("Actor", "a", "labels", &mut budget)?, 2);
    assert_eq!(
        session.navigate("Actor", "a", "link.label", &mut budget)?,
        json!("semantic")
    );
    assert_eq!(
        session.project("Actor", "a", &["label", "labels"], &mut budget)?,
        vec![
            ("label".into(), json!("semantic")),
            ("labels".into(), json!(["one", "two"]))
        ]
    );
    let output = session.query(
        &plan(prepared.schema(), "labels", true),
        &Parameters::new(),
        QueryLimits {
            max_steps: 1_000_000,
            max_results: 10,
        },
    )?;
    assert_eq!(output.rows, vec![vec![json!(2)], vec![json!(2)]]);
    Ok(())
}
pub fn unsupported_fields_dependencies_and_complete_projections_reject_before_store()
-> Result<(), String> {
    let (input, origin) = inputs()?;
    let prepared = prepare(&input, &origin)?;
    let store = store(prepared.schema())?;
    let trace = Trace {
        inner: &store,
        calls: RefCell::new(vec![]),
    };
    let policy = Policy {
        source: input.structure.clone(),
        stored_base: false,
        reject_special: false,
    };
    let session = SemanticReadSession::new(&prepared, &trace, &policy)?;
    let mut budget = QueryBudget::new(1_000_000);
    assert!(
        session
            .project("Actor", "a", &["label", "secret"], &mut budget)
            .unwrap_err()
            .contains("Unsupported semantic read")
    );
    assert!(session.inspect_all("Actor", "a", &mut budget).is_err());
    assert!(
        session
            .navigate("Actor", "a", "link.secret", &mut budget)
            .is_err()
    );
    assert!(
        session
            .value("Actor", "a", "brokenGetter", &mut budget)
            .is_err()
    );
    let mut query = plan(prepared.schema(), "label", false);
    query.filter = Some(Predicate::Compare {
        left: Expression::Field {
            path: "secret".into(),
        },
        comparison: Comparison::Eq,
        right: Expression::Literal {
            value: json!("absent"),
        },
    });
    assert!(
        session
            .query(&query, &Parameters::new(), QueryLimits::default())
            .is_err()
    );
    assert!(
        trace.calls.borrow().is_empty(),
        "{:?}",
        trace.calls.borrow()
    );
    assert!(
        session
            .value("Actor", "a", "undeclaredGetter", &mut budget)
            .unwrap_err()
            .contains("Undeclared semantic read dependency")
    );
    assert!(!trace.calls.borrow().iter().any(|s| s == "value:a:secret"));
    Ok(())
}
pub fn unsafe_equality_anchor_is_disabled_for_computed_subtype_and_unsupported_fails_early()
-> Result<(), String> {
    let (input, origin) = inputs()?;
    let prepared = prepare(&input, &origin)?;
    let store = store(prepared.schema())?;
    let trace = Trace {
        inner: &store,
        calls: RefCell::new(vec![]),
    };
    let policy = Policy {
        source: input.structure.clone(),
        stored_base: true,
        reject_special: false,
    };
    let session = SemanticReadSession::new(&prepared, &trace, &policy)?;
    let mut query = plan(prepared.schema(), "label", false);
    query.filter = Some(Predicate::Compare {
        left: Expression::Field {
            path: "label".into(),
        },
        comparison: Comparison::Eq,
        right: Expression::Literal {
            value: json!("semantic"),
        },
    });
    let result = session.query(
        &query,
        &Parameters::new(),
        QueryLimits {
            max_steps: 1_000_000,
            max_results: 10,
        },
    )?;
    assert_eq!(result.rows, vec![vec![json!("semantic")]]);
    assert!(result.stats.access.starts_with("identity pages"));
    assert!(
        !trace
            .calls
            .borrow()
            .iter()
            .any(|s| s.starts_with("query-value:"))
    );
    trace.calls.borrow_mut().clear();
    let denied = Policy {
        source: input.structure.clone(),
        stored_base: true,
        reject_special: true,
    };
    let session = SemanticReadSession::new(&prepared, &trace, &denied)?;
    assert!(
        session
            .query(&query, &Parameters::new(), QueryLimits::default())
            .is_err()
    );
    assert!(trace.calls.borrow().is_empty());
    Ok(())
}
pub fn source_actual_identity_budget_failure_and_recovery_keep_raw_values() -> Result<(), String> {
    let (input, origin) = inputs()?;
    let prepared = prepare(&input, &origin)?;
    let store = store(prepared.schema())?;
    let mut policy = Policy {
        source: input.structure.clone(),
        stored_base: false,
        reject_special: false,
    };
    policy.source.sha256 = "f".repeat(64);
    assert!(SemanticReadSession::new(&prepared, &store, &policy).is_err());
    policy.source = input.structure.clone();
    let session = SemanticReadSession::new(&prepared, &store, &policy)?;
    let mut budget = QueryBudget::new(1_000_000);
    assert!(
        session
            .value("Actor", "b", "label", &mut budget)
            .unwrap_err()
            .contains("actual receiver identity")
    );
    let mut measurement = QueryBudget::new(1_000_000);
    assert_eq!(
        session.value("Actor", "a", "label", &mut measurement)?,
        json!("semantic")
    );
    assert_eq!(
        session.value(
            "Actor",
            "a",
            "label",
            &mut QueryBudget::new(measurement.reads)
        )?,
        json!("semantic")
    );
    assert!(
        session
            .value(
                "Actor",
                "a",
                "label",
                &mut QueryBudget::new(measurement.reads - 1)
            )
            .unwrap_err()
            .contains("budget exhausted")
    );
    assert_eq!(
        session.value(
            "Actor",
            "a",
            "label",
            &mut QueryBudget::new(measurement.reads)
        )?,
        json!("semantic")
    );
    assert_eq!(store.value("a", "label")?, Some(json!("conflicting raw")));
    Ok(())
}

pub fn index_on_and_off_preserve_whole_semantic_results_and_callback_work() -> Result<(), String> {
    use intersect_model::core::storage::IndexPolicy;
    let (input, origin) = inputs()?;
    let prepared = prepare(&input, &origin)?;
    let policy = Policy {
        source: input.structure.clone(),
        stored_base: true,
        reject_special: false,
    };
    let mut outcomes = Vec::new();
    for indexes in [
        IndexPolicy::AllValues,
        IndexPolicy::selected(Vec::<String>::new()),
    ] {
        let store = store_with_policy(prepared.schema(), indexes)?;
        let trace = Trace {
            inner: &store,
            calls: RefCell::new(vec![]),
        };
        let session = SemanticReadSession::new(&prepared, &trace, &policy)?;
        let mut computed = plan(prepared.schema(), "label", false);
        computed.filter = Some(Predicate::Compare {
            left: Expression::Field {
                path: "label".into(),
            },
            comparison: Comparison::Eq,
            right: Expression::Literal {
                value: json!("semantic"),
            },
        });
        let limits = QueryLimits {
            max_steps: 1_000_000,
            max_results: 10,
        };
        let answer = session.query(&computed, &Parameters::new(), limits)?;
        assert_eq!(answer.rows, vec![vec![json!("semantic")]]);
        let counts = session.query(
            &plan(prepared.schema(), "labels", true),
            &Parameters::new(),
            limits,
        )?;
        assert_eq!(counts.rows, vec![vec![json!(2)], vec![json!(2)]]);
        assert!(
            !trace
                .calls
                .borrow()
                .iter()
                .any(|call| call.starts_with("length:") || call.starts_with("query-value:"))
        );
        // Stored equality remains eligible for the shared existing lookup.
        let mut stored = plan(prepared.schema(), "backing", false);
        stored.filter = Some(Predicate::Compare {
            left: Expression::Field {
                path: "backing".into(),
            },
            comparison: Comparison::Eq,
            right: Expression::Literal {
                value: json!("semantic"),
            },
        });
        let stored_answer = session.query(&stored, &Parameters::new(), limits)?;
        assert_eq!(
            stored_answer.rows,
            vec![vec![json!("semantic")], vec![json!("semantic")]]
        );
        assert!(
            trace
                .calls
                .borrow()
                .iter()
                .any(|call| call.starts_with("query-value:backing:"))
        );
        outcomes.push(json!({ "computed": answer, "count": counts, "stored": stored_answer, "callbacks": trace.calls.into_inner() }));
    }
    assert_eq!(outcomes[0], outcomes[1]);
    Ok(())
}

pub fn zero_budget_precedes_callbacks_and_exact_one_short_work_recovers() -> Result<(), String> {
    let (input, origin) = inputs()?;
    let prepared = prepare(&input, &origin)?;
    let store = store(prepared.schema())?;
    let trace = Trace {
        inner: &store,
        calls: RefCell::new(vec![]),
    };
    let policy = Policy {
        source: input.structure.clone(),
        stored_base: false,
        reject_special: false,
    };
    let session = SemanticReadSession::new(&prepared, &trace, &policy)?;
    let exhausted = "Query work budget exhausted; no complete result";
    let mut zero = QueryBudget::new(0);
    assert_eq!(
        session.value("Actor", "a", "label", &mut zero).unwrap_err(),
        exhausted
    );
    assert_eq!(zero.reads, 0);
    assert!(trace.calls.borrow().is_empty());
    assert_eq!(
        session
            .navigate("Actor", "a", "link.label", &mut zero)
            .unwrap_err(),
        exhausted
    );
    assert_eq!(zero.reads, 0);
    assert!(trace.calls.borrow().is_empty());
    let limits = QueryLimits {
        max_steps: 0,
        max_results: 10,
    };
    assert_eq!(
        session
            .query(
                &plan(prepared.schema(), "label", false),
                &Parameters::new(),
                limits
            )
            .err()
            .ok_or("Expected zero-budget query rejection")?,
        exhausted
    );
    assert!(trace.calls.borrow().is_empty());
    // Metadata errors keep precedence over the new semantic pre-store guard.
    assert!(
        session
            .query(
                &plan(prepared.schema(), "secret", false),
                &Parameters::new(),
                limits
            )
            .err()
            .ok_or("Expected unsupported query rejection")?
            .contains("Unsupported semantic read")
    );
    assert!(trace.calls.borrow().is_empty());
    let mut measurement = QueryBudget::new(1_000_000);
    assert_eq!(
        session.value("Actor", "a", "label", &mut measurement)?,
        json!("semantic")
    );
    let complete_callbacks = trace.calls.borrow().clone();
    // The preserved revision3 profile was 1533. The ordinary Stored leaf now
    // reuses its complete preflight metadata; the recorded current fee is 826.
    let complete_work = 826;
    assert_eq!(measurement.reads, complete_work);
    assert_eq!(
        complete_callbacks,
        [
            "revision",
            "get:a",
            "revision",
            "get:a",
            "revision",
            "class:a",
            "value:a:backing"
        ]
    );

    trace.calls.borrow_mut().clear();
    let mut exact = QueryBudget::new(measurement.reads);
    assert_eq!(
        session.value("Actor", "a", "label", &mut exact)?,
        json!("semantic")
    );
    assert_eq!(exact.reads, measurement.reads);
    assert_eq!(*trace.calls.borrow(), complete_callbacks);
    trace.calls.borrow_mut().clear();
    let mut short = QueryBudget::new(measurement.reads - 1);
    assert_eq!(
        session
            .value("Actor", "a", "label", &mut short)
            .unwrap_err(),
        exhausted
    );
    assert_eq!(short.reads, measurement.reads - 1);
    let failed_callbacks = trace.calls.borrow().clone();
    assert_eq!(short.reads, complete_work - 1);
    assert_eq!(
        failed_callbacks,
        [
            "revision", "get:a", "revision", "get:a", "revision", "class:a"
        ]
    );

    assert!(complete_callbacks.starts_with(&failed_callbacks));
    trace.calls.borrow_mut().clear();
    let mut recovery = QueryBudget::new(measurement.reads);
    assert_eq!(
        session.value("Actor", "a", "label", &mut recovery)?,
        json!("semantic")
    );
    assert_eq!(recovery.reads, measurement.reads);
    assert_eq!(*trace.calls.borrow(), complete_callbacks);
    // Header paths also pass through the shared charged identity hook.
    trace.calls.borrow_mut().clear();
    assert_eq!(
        session.navigate("Actor", "a", "$class", &mut QueryBudget::new(1))?,
        json!("Actor")
    );
    assert!(
        session
            .navigate("Actor", "b", "$class", &mut QueryBudget::new(1))
            .unwrap_err()
            .contains("actual receiver identity")
    );
    #[cfg(not(target_arch = "wasm32"))]
    println!(
        "READ_BUDGET_WITNESS:{}",
        json!({ "zero": {"work":0,"callbacks":[]}, "exact": {"work":measurement.reads,"callbacks":complete_callbacks}, "one_short": {"work":short.reads,"callbacks":failed_callbacks}, "recovery_work":recovery.reads })
    );
    Ok(())
}
