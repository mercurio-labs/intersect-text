//! Handwritten host transaction adapter. All structural and semantic work is
//! delegated to core and the active language package; no Python dependency.
use intersect_model::core::{
    frozen::FrozenStore,
    model::Model,
    transaction::{
        Change, ModelTransaction, TransactionContext, TransactionRecord, TransactionStatus,
    },
};
use intersect_model::definitions::schema::Diagnostic;
use intersect_text::Evaluation;
use intersect_text::package::Package;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap};

pub fn package(request: &Value) -> Result<Package, String> {
    let retained = &request["package"];
    if !retained["xtext_inference"].is_null() {
        return crate::inferred_context::with_rebuilt(retained, |package, _| Ok(package.clone()));
    }
    if [
        "conversion_contracts",
        "applied_conversion_contracts",
        "constructor_contracts",
    ]
    .iter()
    .any(|field| {
        retained["schema"][*field]
            .as_array()
            .is_some_and(|contracts| !contracts.is_empty())
    }) {
        return crate::conversions::rebuild(retained);
    }
    let package: Package =
        serde_json::from_value(retained.clone()).map_err(|e| format!("Invalid package: {e}"))?;
    intersect_text::recompile(&package).map_err(|d| format!("Invalid package: {d:?}"))
}

pub fn inspect(package: &Package, model: Model) -> Result<Evaluation, String> {
    if model.elements.len() > 4096 {
        return Err("Model element budget exceeded (4096)".into());
    }
    crate::conversions::with_package(package, |registry| {
        model.validate_with_conversions(&package.schema, registry)
    })?;
    let mut evaluation = Evaluation {
        model,
        diagnostics: vec![],
        references: vec![],
        derived: BTreeMap::new(),
    };
    intersect_text::apply_semantics_with_provider(
        package,
        &mut evaluation,
        &crate::semantic_bindings::Derived::new(package),
    );
    Ok(evaluation)
}

fn source_bound_settings(package: &Package) -> bool {
    package.xtext_inference.is_some() && !crate::inferred_context::contracts(package).is_empty()
}

fn edit_computed(
    package: &Package,
    model: &mut Model,
    id: &str,
    feature: &str,
    edit: intersect_model::core::storage::ComputedEdit,
    registry: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
) -> Result<(), String> {
    if source_bound_settings(package) {
        return crate::inferred_context::edit_computed(package, model, id, feature, edit, registry);
    }
    model.edit_computed(
        &intersect_model::core::storage::ComputedSettings {
            schema: &package.schema,
            provider: &intersect_model::query::PackageDerived(package),
            conversions: Some(registry),
        },
        id,
        feature,
        edit,
        &mut intersect_model::core::query::QueryBudget::new(100_000),
    )
}

fn checked(package: &Package, model: Model) -> Result<Evaluation, String> {
    let evaluation = inspect(package, model)?;
    if !evaluation.diagnostics.is_empty() {
        return Err(format!(
            "Semantic validation failed: {}",
            serde_json::to_string(&evaluation.diagnostics).map_err(|e| e.to_string())?
        ));
    }
    Ok(evaluation)
}

struct Transaction {
    package: Package,
    transaction: ModelTransaction,
    revision: String,
    calls: usize,
    next_id: u64,
}
thread_local! { static TRANSACTION: RefCell<Option<Transaction>> = const { RefCell::new(None) }; }

pub fn dispatch(method: &str, request: &Value) -> Result<Value, String> {
    if method == "script/finish" {
        let mut report: TransactionReport =
            serde_json::from_value(request["transaction"].clone()).map_err(|e| e.to_string())?;
        let status: TransactionStatus =
            serde_json::from_value(request["status"].clone()).map_err(|e| e.to_string())?;
        if !matches!(
            status,
            TransactionStatus::RolledBack | TransactionStatus::Cancelled
        ) {
            return Err("Host termination requires rolled_back or cancelled status".into());
        }
        if matches!(
            report.record.status,
            TransactionStatus::Running | TransactionStatus::Prepared
        ) {
            report.record.abort(
                status,
                request["reason"]
                    .as_str()
                    .unwrap_or("Host aborted transaction")
                    .into(),
            )?;
        } else if report.record.status != status {
            return Err("Transaction is already closed".into());
        }
        report.changes_complete = request["changes_complete"].as_bool().unwrap_or(false);
        return Ok(json!(report));
    }
    if method == "script/report" || method == "script/publish" {
        let package = package(request)?;
        let before: Model =
            serde_json::from_value(request["before"].clone()).map_err(|e| e.to_string())?;
        let context: TransactionContext =
            serde_json::from_value(request["context"].clone()).map_err(|e| e.to_string())?;
        let proposed: Model =
            serde_json::from_value(request["model"].clone()).map_err(|e| e.to_string())?;
        let mut transaction = crate::conversions::with_package(&package, |registry| {
            before.begin_transaction_with_conversions(&package.schema, context, registry)
        })?;
        transaction.stage(|m| {
            *m = proposed;
            Ok(())
        })?;
        if method == "script/publish" {
            crate::conversions::with_package(&package, |registry| {
                transaction.prepare_with_conversions(
                    &package.schema,
                    |m| checked(&package, m.clone()).map(|_| ()),
                    registry,
                )
            })?;
            let mut current: Model =
                serde_json::from_value(request["current"].clone()).map_err(|e| e.to_string())?;
            transaction.commit(
                &mut current,
                request["revision"].as_str().ok_or("Missing revision")?,
                request["generation"].as_u64().ok_or("Missing generation")?,
            )?;
            let mut result = json!(inspect(&package, current)?);
            result["transaction"] = json!(report(&package, &transaction));
            return Ok(result);
        }
        return Ok(json!(report(&package, &transaction)));
    }
    if method == "script/check" {
        let package = package(request)?;
        let model = serde_json::from_value(request["model"].clone()).map_err(|e| e.to_string())?;
        return Ok(json!(checked(&package, model)?));
    }
    if method == "script/begin" {
        // A failed begin must not leave an older transaction available.
        TRANSACTION.with(|cell| *cell.borrow_mut() = None);
        let package = package(request)?;
        let model = serde_json::from_value(request["model"].clone()).map_err(|e| e.to_string())?;
        let revision = request["revision"]
            .as_str()
            .ok_or("Missing interface revision")?
            .to_owned();
        if request.get("bindings").is_some() {
            crate::semantic_bindings::install_for_package(&package, request.get("bindings"))?;
        }
        let evaluation = checked(&package, model)?;
        let context = if request["context"].is_null() {
            TransactionContext {
                id: format!("script:{revision}"),
                language_revision: revision.clone(),
                base_generation: 0,
            }
        } else {
            serde_json::from_value(request["context"].clone()).map_err(|e| e.to_string())?
        };
        if context.language_revision != revision {
            return Err("Stale transaction context".into());
        }
        let transaction = crate::conversions::with_package(&package, |registry| {
            evaluation
                .model
                .begin_transaction_with_conversions(&package.schema, context, registry)
        })?;
        TRANSACTION.with(|cell| {
            *cell.borrow_mut() = Some(Transaction {
                package,
                transaction,
                revision,
                calls: 0,
                next_id: 0,
            })
        });
        return Ok(Value::Null);
    }
    TRANSACTION.with(|cell| {
        let mut slot = cell.borrow_mut();
        let tx = slot.as_mut().ok_or("No active script transaction")?;
        if method == "script/transaction" {
            if request["revision"].as_str() != Some(&tx.revision) {
                return Err("Stale Python interface revision".into());
            }
            return Ok(json!(report(&tx.package, &tx.transaction)));
        }
        if tx.transaction.status() != TransactionStatus::Running {
            return Err("Transaction aborted by an earlier error".into());
        }
        tx.calls += 1;
        let result = if request["revision"].as_str() != Some(&tx.revision) {
            Err("Stale Python interface revision".into())
        } else if tx.calls > 10000 {
            Err("Script model call budget exceeded (10000)".into())
        } else {
            tx.call(method, request)
        };
        if let Err(error) = &result
            && tx.transaction.status() == TransactionStatus::Running
        {
            tx.transaction
                .abort(TransactionStatus::RolledBack, error.clone())?;
        }
        result
    })
}

impl Transaction {
    fn structural(
        &mut self,
        operation: impl FnOnce(
            &intersect_model::core::storage::StructuralContext<'_>,
            &mut FrozenStore,
            &mut intersect_model::core::query::QueryBudget,
        ) -> Result<Value, String>,
    ) -> Result<Value, String> {
        self.transaction.stage(|model| {
            let mut store = FrozenStore::from_materialized(&self.package.schema, model)?;
            let result = crate::conversions::with_package(&self.package, |registry| {
                operation(
                    &intersect_model::core::storage::StructuralContext {
                        schema: &self.package.schema,
                        conversions: Some(registry),
                    },
                    &mut store,
                    &mut intersect_model::core::query::QueryBudget::new(100_000),
                )
            })?;
            *model = intersect_model::core::storage::export(&store)?;
            Ok(result)
        })
    }
    fn handle(&self, id: &str) -> Result<Value, String> {
        let element = self
            .transaction
            .model()
            .elements
            .iter()
            .find(|e| e.id == id)
            .ok_or("Unknown model handle")?;
        Ok(json!({"id":element.id,"class":element.class}))
    }
    fn call(&mut self, method: &str, request: &Value) -> Result<Value, String> {
        let string = |key| {
            request[key]
                .as_str()
                .ok_or_else(|| format!("Missing {key}"))
        };
        match method {
            "script/find" => {
                let class = string("class")?;
                let name = string("name")?;
                let matches: Vec<_> = self
                    .transaction
                    .model()
                    .elements
                    .iter()
                    .filter(|e| {
                        self.package.schema.conforms(&e.class, class)
                            && e.values.get("name").and_then(Value::as_str) == Some(name)
                    })
                    .collect();
                if matches.len() != 1 {
                    return Err(format!(
                        "Expected one {class} named {name}; found {}",
                        matches.len()
                    ));
                }
                self.handle(&matches[0].id)
            }
            "script/all" => {
                let class = string("class")?;
                if !self.package.schema.classes.contains_key(class) {
                    return Err("Unknown class".into());
                }
                Ok(json!(
                    self.transaction
                        .model()
                        .elements
                        .iter()
                        .filter(|e| self.package.schema.conforms(&e.class, class))
                        .map(|e| json!({"id":e.id,"class":e.class}))
                        .collect::<Vec<_>>()
                ))
            }
            "script/create" => {
                if self.transaction.model().elements.len() >= 4096 {
                    return Err("Model element budget exceeded (4096)".into());
                }
                // A monotonic transaction-local ordinal must survive deletion.
                // The host context separates transactions; collision checks also
                // support legacy callers that reuse their context identity.
                let id = loop {
                    let context = self.transaction.context();
                    let id = format!(
                        "script:{}:{}:{}#{}",
                        self.revision, context.base_generation, context.id, self.next_id
                    );
                    self.next_id = self
                        .next_id
                        .checked_add(1)
                        .ok_or("Script identity counter exhausted")?;
                    if !self.transaction.model().elements.iter().any(|e| e.id == id) {
                        break id;
                    }
                };
                let class = string("class")?;
                let values = request["values"]
                    .as_object()
                    .ok_or("Constructor requires feature values")?;
                self.transaction.stage(|staged| {
                    if self
                        .package
                        .semantics
                        .initializers
                        .iter()
                        .any(|rule| rule.class == class)
                    {
                        let prepared = intersect_model::initialization::PackageInitializers::new(
                            &self.package,
                        )?;
                        let mut store =
                            FrozenStore::from_materialized(&self.package.schema, staged)?;
                        let authored = values
                            .iter()
                            .map(|(name, value)| (name.clone(), value.clone()))
                            .collect::<Vec<_>>();
                        crate::conversions::with_package(&self.package, |registry| {
                            prepared.create_installed(
                                &mut store,
                                &self.package.schema,
                                class,
                                intersect_model::core::storage::Construction {
                                    id: &id,
                                    resource: "script.py",
                                    authored: &authored,
                                },
                                Some(registry),
                                &mut intersect_model::core::query::QueryBudget::new(100_000),
                            )
                        })?;
                        *staged = intersect_model::core::storage::export(&store)?;
                    } else {
                        staged.create(&self.package.schema, &id, class, "script.py", 0, 0)?;
                        for (name, value) in values {
                            crate::conversions::with_package(&self.package, |registry| {
                                staged.set_with_opposite_and_conversions(
                                    &self.package.schema,
                                    &id,
                                    name,
                                    value.clone(),
                                    registry,
                                )
                            })?;
                        }
                    }
                    Ok(())
                })?;
                self.handle(&id)
            }
            "script/delete" => {
                use intersect_model::core::storage::{
                    DeleteRequest, DeleteSelection, IncomingReferences,
                };
                let roots = request["ids"]
                    .as_array()
                    .ok_or("Deletion requires an ids array")?
                    .iter()
                    .map(|id| id.as_str().ok_or("Deletion IDs must be strings"))
                    .collect::<Result<Vec<_>, _>>()?;
                let selection = match string("selection")? {
                    "objects" => DeleteSelection::Objects,
                    "subtrees" => DeleteSelection::Subtrees,
                    _ => return Err("Unknown deletion selection".into()),
                };
                let incoming = match string("incoming")? {
                    "reject" => IncomingReferences::Reject,
                    "detach" => IncomingReferences::Detach,
                    _ => return Err("Unknown incoming-reference policy".into()),
                };
                self.structural(|context, store, budget| {
                    context
                        .delete(
                            store,
                            DeleteRequest {
                                roots: &roots,
                                selection,
                                incoming,
                            },
                            budget,
                        )
                        .map(|removed| json!(removed))
                })
            }
            "script/reparent" => {
                use intersect_model::core::storage::{ContainmentDestination, ReparentRequest};
                let object = string("id")?;
                let destination = if request["owner"].is_null() {
                    if !request["feature"].is_null() || !request["position"].is_null() {
                        return Err("Root detachment cannot specify a feature or position".into());
                    }
                    None
                } else {
                    let position =
                        if request["position"].is_null() {
                            None
                        } else {
                            Some(
                                usize::try_from(request["position"].as_u64().ok_or(
                                    "Containment position must be a non-negative integer",
                                )?)
                                .map_err(|_| "Containment position exceeds host bounds")?,
                            )
                        };
                    Some(ContainmentDestination {
                        owner: string("owner")?,
                        feature: string("feature")?,
                        position,
                    })
                };
                self.structural(|context, store, budget| {
                    context
                        .reparent(
                            store,
                            ReparentRequest {
                                object,
                                destination,
                            },
                            budget,
                        )
                        .map(|changed| json!(changed))
                })
            }
            "script/collection" => {
                use intersect_model::core::storage::CollectionEdit;
                let id = string("id")?;
                let feature = string("feature")?;
                let index = |name: &str| -> Result<usize, String> {
                    usize::try_from(
                        request[name]
                            .as_u64()
                            .ok_or("Collection index must be a non-negative integer")?,
                    )
                    .map_err(|_| "Collection index exceeds host bounds".into())
                };
                let edit = match string("operation")? {
                    "add" => CollectionEdit::Add(
                        request
                            .get("value")
                            .ok_or("Missing collection value")?
                            .clone(),
                    ),
                    "remove" => CollectionEdit::Remove(
                        request
                            .get("value")
                            .ok_or("Missing collection value")?
                            .clone(),
                    ),
                    "clear" => CollectionEdit::Clear,
                    "insert" => CollectionEdit::Insert {
                        index: index("index")?,
                        value: request
                            .get("value")
                            .ok_or("Missing collection value")?
                            .clone(),
                    },
                    "replace" => CollectionEdit::Replace {
                        index: index("index")?,
                        value: request
                            .get("value")
                            .ok_or("Missing collection value")?
                            .clone(),
                    },
                    "remove_at" => CollectionEdit::RemoveAt(index("index")?),
                    "move" => CollectionEdit::Move {
                        from: index("from")?,
                        to: index("to")?,
                    },
                    _ => return Err("Unsupported collection operation".into()),
                };
                let changed = self.transaction.stage(|model| {
                    crate::conversions::with_package(&self.package, |registry| {
                        if source_bound_settings(&self.package) {
                            let class = &model.elements.iter().find(|object| object.id == id).ok_or("Missing element")?.class;
                            let fields = self.package.schema.features(class)?;
                            if fields.get(feature).ok_or("Unknown feature")?.derived {
                                let CollectionEdit::Add(value) = edit else {
                                    return Err("Unsupported computed collection operation; use add, set or unset".into());
                                };
                                edit_computed(&self.package, model, id, feature, intersect_model::core::storage::ComputedEdit::Append(value), registry)?;
                                return Ok(true);
                            }
                        }
                        model.edit_collection_with_conversions(
                            &self.package.schema,
                            id,
                            feature,
                            edit,
                            registry,
                        )
                    })
                })?;
                Ok(json!(changed))
            }
            "script/set" => {
                let id = string("id")?;
                let feature = string("feature")?;
                self.transaction.stage(|m| {
                    crate::conversions::with_package(&self.package, |registry| {
                        let class = m
                            .elements
                            .iter()
                            .find(|object| object.id == id)
                            .ok_or("Missing element")?
                            .class
                            .clone();
                        let fields = self.package.schema.features(&class)?;
                        if fields.get(feature).ok_or("Unknown feature")?.derived {
                            return edit_computed(
                                &self.package,
                                m,
                                id,
                                feature,
                                intersect_model::core::storage::ComputedEdit::Set(
                                    request["value"].clone(),
                                ),
                                registry,
                            );
                        }
                        m.set_with_opposite_and_conversions(
                            &self.package.schema,
                            id,
                            feature,
                            request["value"].clone(),
                            registry,
                        )
                    })
                })?;
                Ok(Value::Null)
            }
            "script/unset" => {
                let id = string("id")?;
                let feature = string("feature")?;
                self.transaction.stage(|model| {
                    crate::conversions::with_package(&self.package, |registry| {
                        let class = model
                            .elements
                            .iter()
                            .find(|object| object.id == id)
                            .ok_or("Missing element")?
                            .class
                            .clone();
                        let fields = self.package.schema.features(&class)?;
                        if fields.get(feature).ok_or("Unknown feature")?.derived {
                            return edit_computed(
                                &self.package,
                                model,
                                id,
                                feature,
                                intersect_model::core::storage::ComputedEdit::Unset,
                                registry,
                            );
                        }
                        model.unset_with_conversions(&self.package.schema, id, feature, registry)
                    })
                })?;
                Ok(Value::Null)
            }
            "script/is_set" => crate::conversions::with_package(&self.package, |registry| {
                Ok(json!(
                    intersect_model::core::storage::is_set_with_derived_and_conversions(
                        &intersect_model::core::storage::ModelView::new(self.transaction.model())?,
                        &self.package.schema,
                        string("id")?,
                        string("feature")?,
                        &crate::semantic_bindings::Derived::new(&self.package),
                        &mut intersect_model::core::query::QueryBudget::new(100_000),
                        registry,
                    )?
                ))
            }),
            "script/get" => {
                let id = string("id")?;
                let element = self
                    .transaction
                    .model()
                    .elements
                    .iter()
                    .find(|e| e.id == id)
                    .ok_or("Unknown model handle")?;
                let fields = self.package.schema.features(&element.class)?;
                let name = string("feature")?;
                let field = fields.get(name).ok_or("Unknown feature")?;
                let value = if field.derived {
                    let evaluation = inspect(&self.package, self.transaction.model().clone())?;
                    evaluation
                        .derived
                        .get(id)
                        .and_then(|v| v.get(name))
                        .cloned()
                        .ok_or("Missing derived value")?
                } else {
                    element.values.get(name).cloned().unwrap_or(Value::Null)
                };
                if !field.reference || value.is_null() {
                    return Ok(value);
                }
                if let Some(values) = value.as_array() {
                    return values
                        .iter()
                        .map(|v| self.handle(v.as_str().ok_or("Invalid reference")?))
                        .collect::<Result<Vec<_>, _>>()
                        .map(Value::Array);
                }
                self.handle(value.as_str().ok_or("Invalid reference")?)
            }
            "script/validate" => Ok(json!(
                inspect(&self.package, self.transaction.model().clone())?.diagnostics
            )),
            "script/inspect" => Ok(json!(inspect(
                &self.package,
                self.transaction.model().clone()
            )?)),
            "script/commit" => {
                crate::conversions::with_package(&self.package, |registry| {
                    self.transaction.prepare_with_conversions(
                        &self.package.schema,
                        |m| checked(&self.package, m.clone()).map(|_| ()),
                        registry,
                    )
                })?;
                let mut result = json!(inspect(&self.package, self.transaction.model().clone())?);
                result["transaction"] = json!(report(&self.package, &self.transaction));
                Ok(result)
            }
            _ => Err(format!("Unsupported model operation {method}")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionReport {
    #[serde(flatten)]
    pub record: TransactionRecord,
    pub derived_changes: Vec<Change>,
    pub diagnostics: Vec<Diagnostic>,
    pub validation: String,
    pub changes_complete: bool,
}

fn report(package: &Package, transaction: &ModelTransaction) -> TransactionReport {
    let mut result = TransactionReport {
        record: transaction.record(),
        derived_changes: vec![],
        diagnostics: vec![],
        validation: "passed".into(),
        changes_complete: true,
    };
    match inspect(package, transaction.model().clone()) {
        Err(error) => {
            result.validation = "failed".into();
            result
                .diagnostics
                .push(Diagnostic::new("model", 0, 0, error));
        }
        Ok(after) => {
            result.diagnostics = after.diagnostics;
            if !result.diagnostics.is_empty() {
                result.validation = "failed".into();
            }
            if let Ok(before) = inspect(package, transaction.before().clone()) {
                for element in &after.model.elements {
                    if let Some(fields) = after.derived.get(&element.id) {
                        for (name, value) in fields {
                            let previous = before
                                .derived
                                .get(&element.id)
                                .and_then(|f| f.get(name))
                                .cloned()
                                .unwrap_or(Value::Null);
                            if previous != *value {
                                result.derived_changes.push(Change {
                                    kind: "derived".into(),
                                    element: element.id.clone(),
                                    class: element.class.clone(),
                                    feature: Some(name.clone()),
                                    before: previous,
                                    after: value.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    result
}
