//! A serialization view of stored and grammar-assigned computed state. Core owns the graph;
//! this module selects values to persist without mutating the caller's model.
use super::*;
use intersect_model::core::{model::AssignmentState, query::QueryBudget};

/// Native policy for state that is intentionally absent from generated text.
/// Per-item policies must remain meaningful after transient items are removed
/// and parser IDs change. Values with their default are optional, not transient.
pub trait TransientValues {
    fn is_transient(
        &self,
        package: &crate::package::Package,
        model: &Model,
        value: TransientValue<'_>,
    ) -> Result<bool, String>;
}
#[derive(Clone, Copy)]
pub struct TransientValue<'a> {
    pub owner: &'a Element,
    pub feature: &'a str,
    /// None selects the entire feature; Some selects a collection member.
    pub index: Option<usize>,
    pub value: &'a Value,
}
pub fn default_transient(
    package: &crate::package::Package,
    value: TransientValue<'_>,
) -> Result<bool, String> {
    Ok(package
        .schema
        .effective_feature(&value.owner.class, value.feature)?
        .feature
        .transient)
}
pub(crate) fn transient(
    package: &crate::package::Package,
    model: &Model,
    policy: Option<&dyn TransientValues>,
    value: TransientValue<'_>,
) -> Result<bool, String> {
    match policy {
        Some(policy) => policy.is_transient(package, model, value),
        None => default_transient(package, value),
    }
}
#[derive(Default)]
pub(crate) struct Member {
    pub whole: bool,
    pub omitted: BTreeSet<usize>,
}
impl Member {
    fn prepare(
        package: &crate::package::Package,
        model: &Model,
        policy: Option<&dyn TransientValues>,
        owner: &Element,
        feature: &str,
        value: &Value,
        work: &mut QueryBudget,
    ) -> Result<Self, String> {
        work.charge()?;
        let mut member = Self {
            whole: transient(
                package,
                model,
                policy,
                TransientValue {
                    owner,
                    feature,
                    index: None,
                    value,
                },
            )?,
            omitted: BTreeSet::new(),
        };
        if !member.whole
            && package
                .schema
                .effective_feature(&owner.class, feature)?
                .feature
                .upper
                != 1
            && let Some(values) = value.as_array()
        {
            for (index, value) in values.iter().enumerate() {
                work.charge()?;
                if transient(
                    package,
                    model,
                    policy,
                    TransientValue {
                        owner,
                        feature,
                        index: Some(index),
                        value,
                    },
                )? {
                    member.omitted.insert(index);
                }
            }
        }
        Ok(member)
    }
    pub fn selected(&self, index: usize) -> bool {
        !self.whole && !self.omitted.contains(&index)
    }
    pub fn value(&self, value: &Value) -> Value {
        match value {
            Value::Array(items) if !self.omitted.is_empty() => Value::Array(
                items
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| self.selected(*index))
                    .map(|(_, value)| value.clone())
                    .collect(),
            ),
            _ => value.clone(),
        }
    }
}
pub(crate) struct State {
    pub members: BTreeMap<(String, String), Member>,
    pub computed: BTreeMap<(String, String), Value>,
    pub retained: BTreeSet<String>,
    pub steps: usize,
}
impl State {
    pub fn new(
        package: &crate::package::Package,
        model: &Model,
        services: Option<&crate::document::DocumentServices<'_, '_>>,
        steps: usize,
    ) -> Result<Self, String> {
        let mut work = QueryBudget::new(steps);
        let policy = services.and_then(|services| services.transient);
        let mut result = Self {
            members: BTreeMap::new(),
            computed: BTreeMap::new(),
            retained: BTreeSet::new(),
            steps: 0,
        };
        for object in &model.elements {
            work.charge()?;
            for (name, field) in package.schema.features(&object.class)? {
                work.charge()?;
                if field.derived {
                    continue;
                }
                let value = object.values.get(&name).ok_or_else(|| {
                    format!("Missing stored serialization value {}.{name}", object.id)
                })?;
                let member =
                    Member::prepare(package, model, policy, object, &name, value, &mut work)?;
                result.members.insert((object.id.clone(), name), member);
            }
        }
        let objects: BTreeMap<_, _> = model
            .elements
            .iter()
            .map(|object| (object.id.as_str(), object))
            .collect();
        let mut pending: Vec<_> = model
            .resources
            .values()
            .flat_map(|resource| resource.roots.iter().cloned())
            .collect();
        while let Some(id) = pending.pop() {
            work.charge()?;
            if !result.retained.insert(id.clone()) {
                // Computed containment may expose the same stored children.
                // Core has already validated the physical containment forest.
                continue;
            }
            let object = objects
                .get(id.as_str())
                .ok_or("Missing persistent serialization object")?;
            let (computed, reads) = super::read_computed(
                package,
                model,
                services,
                &BTreeSet::from([id.clone()]),
                steps.saturating_sub(work.reads),
            )?;
            for _ in 0..reads {
                work.charge()?;
            }
            for ((owner, name), value) in computed {
                let member =
                    Member::prepare(package, model, policy, object, &name, &value, &mut work)?;
                result.members.insert((owner.clone(), name.clone()), member);
                result.computed.insert((owner, name), value);
            }
            for (name, field) in package.schema.features(&object.class)? {
                work.charge()?;
                if !field.containment
                    || (field.derived && !result.computed.contains_key(&(id.clone(), name.clone())))
                {
                    continue;
                }
                let member = result.member(&id, &name)?;
                if member.whole {
                    continue;
                }
                let value = result
                    .value(object, &name)
                    .ok_or("Missing serialization containment")?;
                match value {
                    Value::Null => {}
                    Value::String(target) => pending.push(target.clone()),
                    Value::Array(values) => {
                        for (index, value) in values.iter().enumerate() {
                            work.charge()?;
                            if member.selected(index) {
                                pending.push(
                                    value
                                        .as_str()
                                        .ok_or("Invalid serialization containment identity")?
                                        .into(),
                                );
                            }
                        }
                    }
                    _ => return Err("Invalid serialization containment value".into()),
                }
            }
        }
        result.steps = work.reads;
        Ok(result)
    }
    pub fn value<'a>(&'a self, object: &'a Element, feature: &str) -> Option<&'a Value> {
        self.computed
            .get(&(object.id.clone(), feature.into()))
            .or_else(|| object.values.get(feature))
    }
    pub fn member(&self, object: &str, feature: &str) -> Result<&Member, String> {
        self.members
            .get(&(object.into(), feature.into()))
            .ok_or_else(|| "Missing persistent feature selection".into())
    }
    pub fn assignment_state(
        &self,
        object: &Element,
        schema: &Schema,
        ignored: &BTreeSet<String>,
    ) -> Result<Option<AssignmentState>, String> {
        if !schema
            .features(&object.class)?
            .iter()
            .any(|(name, field)| field.unsettable && !field.derived && !ignored.contains(name))
        {
            return Ok(None);
        }
        object
            .assignment_state
            .as_ref()
            .map(|state| {
                AssignmentState::new(state.set_features().filter(|name| !ignored.contains(*name)))
            })
            .transpose()
    }
    pub fn ignored(&self, object: &str) -> BTreeSet<String> {
        self.members
            .range((object.into(), String::new())..)
            .take_while(|((owner, _), _)| owner == object)
            .filter(|(_, member)| member.whole)
            .map(|((_, feature), _)| feature.clone())
            .collect()
    }
}
