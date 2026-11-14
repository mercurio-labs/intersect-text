//! Handwritten, bounded canonical serialization through the compiled grammar.
//! Language-owned spellings are checked by the authoritative parser/linker and
//! complete graph comparison. This is separate from source-preserving edits.
use crate::{
    EvaluationLimits, Source,
    document::{DocumentAnalysisSet, DocumentLanguage},
    grammar::{Expr, Grammar, Kind},
};
use intersect_model::{
    core::{
        model::{Element, Model},
        resources::ResourceLoadState,
    },
    definitions::schema::Schema,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[path = "serialization_graph.rs"]
mod graph;
pub(crate) use graph::{correspondence, persistent_correspondence};
#[path = "serialization_state.rs"]
mod state;
pub(crate) use state::State as SerializationState;
pub use state::{TransientValue, TransientValues, default_transient};
#[path = "serialization_spelling.rs"]
mod spelling;
pub use spelling::default_spelling;
pub(crate) use spelling::default_spelling_with_conversions;

/// Optional native layout policy. Output is still parsed, linked and compared
/// with the complete candidate graph; formatting cannot grant model authority.
pub trait TextFormatter {
    fn format(
        &self,
        package: &crate::package::Package,
        model: &Model,
        source: &Source,
    ) -> Result<String, String>;
}

/// Native decision for a grammar call that stores no semantic value. Returning
/// None retains a verified source spelling or uses a grammar-derived witness.
pub trait UnassignedText {
    fn spelling(
        &self,
        package: &crate::package::Package,
        model: &Model,
        value: UnassignedSpelling<'_>,
    ) -> Result<Option<String>, String>;
}
pub struct UnassignedSpelling<'a> {
    /// Empty object entries have no semantic owner.
    pub owner: Option<&'a Element>,
    pub resource: &'a str,
    pub rule: &'a str,
    pub path: Option<&'a str>,
    pub index: usize,
    pub original: Option<&'a str>,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum HintOwner {
    Object(String),
    Resource(String),
}
#[derive(Default)]
pub(crate) struct SerializationHints {
    pub counts: BTreeMap<(HintOwner, String), usize>,
    pub text: BTreeMap<(HintOwner, String), Vec<String>>,
}

/// Render a selected new object's declaration through the same bounded emitter.
/// Only the enclosing source-edit service can verify the resulting fragment.
pub(crate) fn render_fragment(
    package: &crate::package::Package,
    model: &Model,
    rule: &str,
    id: &str,
    limits: SerializationLimits,
    mut spelling: impl FnMut(TerminalSpelling<'_>) -> Result<String, String>,
) -> Result<String, String> {
    let grammar = package
        .grammar
        .as_ref()
        .ok_or("Insertion needs a text grammar")?;
    let mut emitter = Emitter {
        model,
        schema: &package.schema,
        grammar,
        objects: model.elements.iter().map(|e| (e.id.as_str(), e)).collect(),
        visited: BTreeSet::new(),
        spelling: &mut spelling,
        limits,
        steps: 0,
        spelling_bytes: 0,
        choices: Vec::new(),
        trace: Vec::new(),
        cursor: 0,
        hidden: grammar.hidden.as_deref(),
        active: BTreeSet::new(),
        hints: None,
        computed: &BTreeMap::new(),
        package,
        unassigned: None,
        unassigned_positions: BTreeMap::new(),
        state: None,
        transient_policy: None,
    };
    let tokens = emitter
        .object(rule, id, 0)?
        .ok_or("New object has no representation in the selected parser rule")?;
    format_tokens(&tokens, limits.output_bytes)
}

/// Finite rendering limits, independent of the subsequent document analysis.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SerializationLimits {
    pub steps: usize,
    pub output_bytes: usize,
    pub nesting: usize,
}
impl Default for SerializationLimits {
    fn default() -> Self {
        Self {
            steps: 1_000_000,
            output_bytes: 256_000,
            nesting: 64,
        }
    }
}

/// One terminal spelling. For a noncontainment reference, `value` is the original
/// target ID and `target` is the exact model object, never a name-based guess.
pub struct TerminalSpelling<'a> {
    pub owner: &'a Element,
    pub feature: &'a str,
    pub terminal: &'a str,
    /// Position within a stored collection; zero for a single-valued feature.
    pub index: usize,
    pub value: &'a Value,
    pub target: Option<&'a Element>,
}

/// Observations from verified canonical output. Instance IDs in the parsed model
/// belong to that new source; `identities` maps persisted original IDs to parsed IDs.
/// Later mutation of these DTOs does not retain the verification guarantee.
#[derive(Debug)]
pub struct SerializedModel {
    pub sources: Vec<Source>,
    pub analysis: DocumentAnalysisSet,
    pub identities: BTreeMap<String, String>,
}

pub struct CanonicalSerializer {
    language: std::sync::Arc<DocumentLanguage>,
    evaluation_limits: EvaluationLimits,
    limits: SerializationLimits,
}
impl CanonicalSerializer {
    /// Grammar-derived spelling and unique-name scope defaults. Languages with
    /// custom codecs or inverse scoping can use `serialize_with`.
    pub fn serialize(&self, model: &Model) -> Result<SerializedModel, String> {
        self.serialize_with(model, |value| default_spelling(self.package(), value))
    }
    pub fn new(
        language: DocumentLanguage,
        evaluation_limits: EvaluationLimits,
        limits: SerializationLimits,
    ) -> Self {
        Self::new_shared(std::sync::Arc::new(language), evaluation_limits, limits)
    }

    pub fn new_shared(
        language: std::sync::Arc<DocumentLanguage>,
        evaluation_limits: EvaluationLimits,
        limits: SerializationLimits,
    ) -> Self {
        Self {
            language,
            evaluation_limits,
            limits,
        }
    }

    /// Use explicit native parsing/conversion/scoping services for the same
    /// verification path. The spelling callback supplies the corresponding
    /// inverse conversion or scope policy; it receives actual target identities.
    pub fn serialize_with_services(
        &self,
        model: &Model,
        services: &mut crate::document::DocumentServices<'_, '_>,
        spelling: impl FnMut(TerminalSpelling<'_>) -> Result<String, String>,
    ) -> Result<SerializedModel, String> {
        serialize(
            &self.language,
            self.evaluation_limits,
            self.limits,
            model,
            None,
            Some(services),
            spelling,
        )
    }

    pub fn serialize_using_services(
        &self,
        model: &Model,
        services: &mut crate::document::DocumentServices<'_, '_>,
    ) -> Result<SerializedModel, String> {
        if services.linking.is_some() {
            return Err("A custom scope requires an explicit inverse spelling policy".into());
        }
        let conversions = services.conversions;
        self.serialize_with_services(model, services, |value| {
            default_spelling_with_conversions(self.package(), value, conversions)
        })
    }

    pub fn serialize_with_native_services(
        &self,
        model: &Model,
        services: &crate::text_services::TextServices,
    ) -> Result<SerializedModel, String> {
        services.with_context(&self.language, self.limits.steps, |context| {
            self.serialize_with_services(model, &mut context.services, |value| {
                services.spelling(self.package(), model, value)
            })
        })
    }

    pub fn package(&self) -> &crate::package::Package {
        self.language.package()
    }

    /// Render each resource's root or representable empty entry. Resources with
    /// multiple entry roots require a containing syntax object. Declaration-order
    /// alternatives define canonical choice; all resulting state is verified.
    pub fn serialize_with(
        &self,
        model: &Model,
        mut spelling: impl FnMut(TerminalSpelling<'_>) -> Result<String, String>,
    ) -> Result<SerializedModel, String> {
        serialize(
            &self.language,
            self.evaluation_limits,
            self.limits,
            model,
            None,
            None,
            &mut spelling,
        )
    }
}

pub(crate) fn serialize(
    language: &DocumentLanguage,
    evaluation_limits: EvaluationLimits,
    limits: SerializationLimits,
    model: &Model,
    hints: Option<&SerializationHints>,
    mut services: Option<&mut crate::document::DocumentServices<'_, '_>>,
    mut spelling: impl FnMut(TerminalSpelling<'_>) -> Result<String, String>,
) -> Result<SerializedModel, String> {
    let package = language.package();
    match services.as_ref().and_then(|services| services.conversions) {
        Some(registry) => model.validate_with_conversions(&package.schema, registry)?,
        None => model.validate(&package.schema)?,
    }
    let grammar = package
        .grammar
        .as_ref()
        .ok_or("Serialization needs a text grammar")?;
    if let Some(linking) = services
        .as_ref()
        .and_then(|services| services.linking.as_ref())
    {
        linking.resolver.validate_package(package)?;
    }
    if services
        .as_ref()
        .and_then(|services| services.settings)
        .is_some_and(|settings| !settings.belongs_to(package))
    {
        return Err("Text setting context belongs to another package".into());
    }
    if services
        .as_ref()
        .and_then(|services| services.derived)
        .and_then(|provider| provider.schema_revision())
        .is_some_and(|revision| revision != package.schema.revision)
    {
        return Err("Derived serialization provider belongs to another schema revision".into());
    }

    if model.resources.is_empty() {
        return Err("Canonical serialization requires an explicit model resource".into());
    }
    let transient_policy = services.as_ref().and_then(|services| services.transient);
    let persistent = SerializationState::new(package, model, services.as_deref(), limits.steps)?;
    let mut emitter = Emitter {
        model,
        schema: &package.schema,
        grammar,
        objects: model.elements.iter().map(|e| (e.id.as_str(), e)).collect(),
        visited: BTreeSet::new(),
        spelling: &mut spelling,
        limits,
        steps: persistent.steps,
        spelling_bytes: 0,
        choices: Vec::new(),
        trace: Vec::new(),
        cursor: 0,
        hidden: grammar.hidden.as_deref(),
        active: BTreeSet::new(),
        hints,
        computed: &persistent.computed,
        package,
        unassigned: services.as_ref().and_then(|services| services.unassigned),
        unassigned_positions: BTreeMap::new(),
        state: Some(&persistent),
        transient_policy,
    };
    let mut best_error: Option<(usize, String)> = None;
    loop {
        let mut progress = 0;
        emitter.visited.clear();
        emitter.trace.clear();
        emitter.cursor = 0;
        emitter.unassigned_positions.clear();
        let attempt = (|| {
            let mut sources = Vec::new();
            let mut total = 0usize;
            for (uri, resource) in &model.resources {
                if resource.load_state != ResourceLoadState::Loaded
                    || !resource.diagnostics.is_empty()
                {
                    return Err(format!(
                        "Cannot serialize unavailable or diagnosed resource {uri}"
                    ));
                }
                if resource.roots.len() > 1 {
                    return Err(format!("Text resource {uri} has more than one entry root"));
                }
                let entry = services
                    .as_ref()
                    .and_then(|services| services.entry)
                    .unwrap_or(&grammar.entry);
                let tokens = match resource.roots.first() {
                    Some(root) => emitter.object(entry, root, 0)?,
                    None => emitter.empty_entry(entry, uri, 0)?,
                }
                .ok_or_else(|| {
                    format!("Model entry in {uri} has no representation in rule {entry}")
                })?;
                let mut text = format_tokens(&tokens, limits.output_bytes.saturating_sub(total))?;
                if let Some(formatter) = services.as_ref().and_then(|services| services.formatter) {
                    text = formatter.format(
                        package,
                        model,
                        &Source {
                            uri: uri.clone(),
                            text,
                        },
                    )?;
                    emitter.charge_text(text.len())?;
                }
                total = total
                    .checked_add(text.len())
                    .ok_or("Serialized output length overflow")?;
                if total > limits.output_bytes {
                    return Err("Serialization output byte budget exceeded".into());
                }
                sources.push(Source {
                    uri: uri.clone(),
                    text,
                });
            }
            if emitter.visited.len() != persistent.retained.len() {
                return Err("Canonical grammar did not consume every model object".into());
            }
            progress = 1;
            let analysis = match services.as_deref_mut() {
                Some(services) => {
                    language.analyze_sources_with_services(&sources, evaluation_limits, services)?
                }
                None => language.analyze_sources(&sources, None, evaluation_limits)?,
            };
            if !analysis.diagnostics.is_empty() || analysis.recovered.is_some() {
                return Err(format!(
                    "Generated text failed parse/link/validation: {:?}",
                    analysis.diagnostics
                ));
            }
            progress = 2;
            let actual = &analysis
                .valid_model
                .as_ref()
                .ok_or("Generated text has no accepted model")?
                .model;
            let actual_state = SerializationState::new(
                package,
                actual,
                services.as_deref(),
                limits.steps.saturating_sub(emitter.steps),
            )?;
            emitter.steps = emitter.steps.saturating_add(actual_state.steps);
            let identities = graph::persistent_correspondence(
                model,
                actual,
                &package.schema,
                &persistent,
                &actual_state,
            )?;
            Ok(SerializedModel {
                sources,
                analysis,
                identities,
            })
        })();
        match attempt {
            Ok(output) => return Ok(output),
            Err(error) => {
                if best_error.as_ref().is_none_or(|(old, _)| progress > *old) {
                    best_error = Some((progress, error.clone()));
                }
                if emitter.steps > limits.steps || emitter.spelling_bytes > limits.output_bytes {
                    return Err(best_error.map_or_else(
                        || error.clone(),
                        |(_, best)| {
                            if best == error {
                                error.clone()
                            } else {
                                format!("{best}; serialization search stopped: {error}")
                            }
                        },
                    ));
                }
                if !emitter.advance() {
                    return Err(best_error.map_or(error, |(_, message)| message));
                }
            }
        }
    }
}

type ComputedValues = BTreeMap<(String, String), Value>;

fn read_computed(
    package: &crate::package::Package,
    model: &Model,
    services: Option<&crate::document::DocumentServices<'_, '_>>,
    retained: &BTreeSet<String>,
    steps: usize,
) -> Result<(ComputedValues, usize), String> {
    use intersect_model::{
        core::{query::QueryBudget, storage::ModelView},
        query::ModelDerived,
    };
    if package
        .grammar
        .as_ref()
        .is_some_and(|grammar| grammar.computed_settings.is_empty())
    {
        return Ok((BTreeMap::new(), 0));
    }
    let mut work = QueryBudget::new(steps);
    let mut assigned = BTreeSet::new();
    fn collect(
        expression: &Expr,
        names: &mut BTreeSet<String>,
        work: &mut QueryBudget,
    ) -> Result<(), String> {
        work.charge()?;
        match &expression.kind {
            Kind::Assign { feature, value, .. } | Kind::BooleanAssign { feature, value } => {
                names.insert(feature.clone());
                collect(value, names, work)?;
            }
            Kind::Sequence(items) | Kind::Alternative(items) => {
                for item in items {
                    collect(item, names, work)?;
                }
            }
            Kind::SyntacticPredicate(value) => collect(value, names, work)?,
            _ => {}
        }
        Ok(())
    }
    for rule in package
        .grammar
        .as_ref()
        .ok_or("Missing grammar")?
        .rules
        .values()
    {
        collect(&rule.body, &mut assigned, &mut work)?;
    }
    let view = ModelView::new(model)?;
    let default = ModelDerived(package.model_definition());
    let derived = services.and_then(|s| s.derived).unwrap_or(&default);
    let settings = services.and_then(|s| s.settings);
    if settings.is_some_and(|settings| !settings.belongs_to(package)) {
        return Err("Text setting context belongs to another package".into());
    }
    let mut result = BTreeMap::new();
    for object in &model.elements {
        work.charge()?;
        if !retained.contains(&object.id) {
            continue;
        }
        for (name, field) in package.schema.features(&object.class)? {
            work.charge()?;
            if !field.derived
                || !assigned.contains(&name)
                || !package.grammar.as_ref().is_some_and(|grammar| {
                    grammar
                        .computed_settings
                        .iter()
                        .any(|contract| contract.class == object.class && contract.feature == name)
                })
            {
                continue;
            }
            let value =
                if let Some(settings) = settings.filter(|s| s.supports(&object.class, &name)) {
                    settings.read(&view, &object.id, &object.class, &name, &mut work)?
                } else if derived.supports(&object.class, &name) {
                    derived.read(&view, &object.id, &name, &mut work)?
                } else {
                    return Err(format!(
                        "No inverse getter for grammar-assigned computed feature {}.{name}",
                        object.id
                    ));
                };
            result.insert((object.id.clone(), name), value);
        }
    }
    Ok((result, work.reads))
}

#[derive(Clone)]
struct Frame<'a> {
    object: &'a Element,
    positions: BTreeMap<(String, String), usize>,
}
struct Emitter<'a, F> {
    model: &'a Model,
    schema: &'a Schema,
    grammar: &'a Grammar,
    objects: BTreeMap<&'a str, &'a Element>,
    visited: BTreeSet<String>,
    spelling: &'a mut F,
    limits: SerializationLimits,
    steps: usize,
    spelling_bytes: usize,
    choices: Vec<usize>,
    trace: Vec<(usize, usize)>,
    cursor: usize,
    hidden: Option<&'a [String]>,
    active: BTreeSet<(String, String)>,
    hints: Option<&'a SerializationHints>,
    computed: &'a BTreeMap<(String, String), Value>,
    package: &'a crate::package::Package,
    unassigned: Option<&'a dyn UnassignedText>,
    unassigned_positions: BTreeMap<(HintOwner, String), usize>,
    state: Option<&'a SerializationState>,
    transient_policy: Option<&'a dyn TransientValues>,
}
impl<'a, F: FnMut(TerminalSpelling<'_>) -> Result<String, String>> Emitter<'a, F> {
    fn selected(
        &mut self,
        object: &Element,
        feature: &str,
        index: Option<usize>,
        value: &Value,
    ) -> Result<bool, String> {
        self.step(0)?;
        if let Some(member) = self
            .state
            .and_then(|state| state.members.get(&(object.id.clone(), feature.into())))
        {
            return Ok(match index {
                Some(index) => member.selected(index),
                None => !member.whole,
            });
        }
        Ok(!state::transient(
            self.package,
            self.model,
            self.transient_policy,
            TransientValue {
                owner: object,
                feature,
                index,
                value,
            },
        )?)
    }
    fn feature_value(&self, object: &Element, feature: &str) -> Option<&'a Value> {
        self.computed
            .get(&(object.id.clone(), feature.into()))
            .or_else(|| {
                self.objects
                    .get(object.id.as_str())
                    .and_then(|e| e.values.get(feature))
            })
    }
    fn accepts_hidden(&mut self, value: &str) -> Result<bool, String> {
        let Some(hidden) = self.hidden else {
            return Ok(true);
        };
        for name in hidden {
            self.step(0)?;
            if let Some(terminal) = self.grammar.terminals.get(name) {
                let mut remaining = Some(self.limits.steps.saturating_sub(self.steps));
                let result = terminal.match_len_with_work(value, &mut remaining);
                self.steps = self
                    .limits
                    .steps
                    .saturating_sub(remaining.ok_or("Lost separator work budget")?);
                if result.is_ok_and(|length| length == value.len()) {
                    return Ok(true);
                }
                if self.steps == self.limits.steps {
                    return Err("Separator matching work budget exceeded".into());
                }
            }
        }
        Ok(false)
    }
    fn token(&mut self, text: String) -> Result<RenderedToken, String> {
        self.step(0)?;
        let space = self.accepts_hidden(" ")?;
        let newline = self.accepts_hidden("\n")?;
        let mut separator = None;
        if !space {
            if newline {
                separator = Some("\n".into());
            } else if let Some(hidden) = self.hidden {
                // A language may hide punctuation or comments rather than
                // whitespace. Use that exact terminal program's witness.
                if !hidden.is_empty() {
                    let first = self.branch(hidden.len())?;
                    for offset in 0..hidden.len() {
                        self.step(0)?;
                        let name = &hidden[(first + offset) % hidden.len()];
                        let value = match self.witness(name, 0) {
                            Ok(value) => value,
                            Err(_) if self.steps <= self.limits.steps => continue,
                            Err(error) => return Err(error),
                        };
                        if !value.is_empty() && self.accepts_hidden(&value)? {
                            self.charge_text(value.len())?;
                            separator = Some(value);
                            break;
                        }
                    }
                }
            }
        }
        Ok(RenderedToken {
            text,
            space,
            newline,
            separator,
        })
    }

    fn step(&mut self, depth: usize) -> Result<(), String> {
        if depth > self.limits.nesting {
            return Err("Serialization nesting budget exceeded".into());
        }
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or("Serialization work overflow")?;
        if self.steps > self.limits.steps {
            return Err("Serialization work budget exceeded".into());
        }
        Ok(())
    }

    fn charge_text(&mut self, length: usize) -> Result<(), String> {
        self.spelling_bytes = self
            .spelling_bytes
            .checked_add(length)
            .ok_or("Serialized spelling length overflow")?;
        if self.spelling_bytes > self.limits.output_bytes {
            return Err("Serialization spelling exceeds the output byte budget".into());
        }
        Ok(())
    }

    fn unassigned_spelling(
        &mut self,
        expression: &Expr,
        name: &str,
        owner: Option<&Element>,
        resource: &str,
        depth: usize,
    ) -> Result<String, String> {
        self.step(depth)?;
        let hint_owner = owner.map_or_else(
            || HintOwner::Resource(resource.into()),
            |owner| HintOwner::Object(owner.id.clone()),
        );
        let key = (
            hint_owner,
            expression
                .prediction_path
                .clone()
                .unwrap_or_else(|| name.into()),
        );
        let index = *self.unassigned_positions.entry(key.clone()).or_insert(0);
        self.unassigned_positions.insert(
            key.clone(),
            index
                .checked_add(1)
                .ok_or("Unassigned occurrence overflow")?,
        );
        let original = self
            .hints
            .and_then(|hints| {
                hints.text.get(&key).or_else(|| {
                    owner
                        .filter(|owner| {
                            self.model
                                .resources
                                .get(resource)
                                .is_some_and(|record| record.roots.contains(&owner.id))
                        })
                        .and_then(|_| {
                            hints
                                .text
                                .get(&(HintOwner::Resource(resource.into()), key.1.clone()))
                        })
                })
            })
            .and_then(|values| values.get(index))
            .map(String::as_str);
        let selected = match self.unassigned {
            Some(policy) => policy.spelling(
                self.package,
                self.model,
                UnassignedSpelling {
                    owner,
                    resource,
                    rule: name,
                    path: expression.prediction_path.as_deref(),
                    index,
                    original,
                },
            )?,
            None => None,
        };
        let text = if let Some(text) = selected {
            text
        } else if let Some(original) = original {
            if self.branch(2)? == 0 {
                original.to_owned()
            } else {
                self.witness(name, depth + 1)?
            }
        } else {
            self.witness(name, depth + 1)?
        };
        self.charge_text(text.len())?;
        Ok(text)
    }

    fn empty_entry(
        &mut self,
        name: &str,
        resource: &str,
        depth: usize,
    ) -> Result<Option<Vec<RenderedToken>>, String> {
        self.step(depth)?;
        let rule = self
            .grammar
            .rules
            .get(name)
            .ok_or("Missing empty-entry rule")?;
        if rule.datatype.is_some() || rule.fragment {
            return Err("An empty model entry requires an object parser rule".into());
        }
        self.empty_rule(rule, resource, depth + 1)
    }
    fn empty_rule(
        &mut self,
        rule: &'a crate::grammar::Rule,
        resource: &str,
        depth: usize,
    ) -> Result<Option<Vec<RenderedToken>>, String> {
        self.step(depth)?;
        let hidden = self.hidden;
        if let Some(policy) = rule.hidden.as_deref() {
            self.hidden = Some(policy);
        }
        let result = self.empty_expression(&rule.body, resource, depth + 1);
        self.hidden = hidden;
        result
    }
    fn empty_expression(
        &mut self,
        expression: &Expr,
        resource: &str,
        depth: usize,
    ) -> Result<Option<Vec<RenderedToken>>, String> {
        self.step(depth)?;
        let owner = HintOwner::Resource(resource.into());
        let hint = expression
            .prediction_path
            .as_ref()
            .and_then(|path| {
                self.hints
                    .and_then(|hints| hints.counts.get(&(owner, path.clone())))
            })
            .copied()
            .unwrap_or(0);
        let minimum = usize::from(matches!(expression.cardinality, '1' | '+'));
        let count = match expression.cardinality {
            '1' => 1,
            '?' => usize::from(self.branch(2)? == usize::from(hint == 0)),
            '*' | '+' => {
                let count = hint.max(minimum);
                if self.branch(2)? == 0 { count } else { minimum }
            }
            _ => return Err("Unknown empty-entry cardinality".into()),
        };
        let mut output = Vec::new();
        for _ in 0..count {
            self.step(depth)?;
            let next = match &expression.kind {
                Kind::Literal(text) => {
                    self.charge_text(text.len())?;
                    Some(vec![self.token(text.clone())?])
                }
                Kind::Sequence(items) => {
                    let mut tokens = Vec::new();
                    for item in items {
                        let Some(next) = self.empty_expression(item, resource, depth + 1)? else {
                            return Ok(None);
                        };
                        tokens.extend(next);
                    }
                    Some(tokens)
                }
                Kind::Alternative(items) => {
                    let index = self.branch(items.len())?;
                    self.empty_expression(&items[index], resource, depth + 1)?
                }
                Kind::SyntacticPredicate(value) => {
                    self.empty_expression(value, resource, depth + 1)?
                }
                Kind::Call(name) => {
                    if let Some(rule) = self
                        .grammar
                        .rules
                        .get(name)
                        .filter(|rule| rule.datatype.is_none())
                    {
                        self.empty_rule(rule, resource, depth + 1)?
                    } else {
                        let text =
                            self.unassigned_spelling(expression, name, None, resource, depth + 1)?;
                        Some(vec![self.token(text)?])
                    }
                }
                Kind::Action(_)
                | Kind::Capture { .. }
                | Kind::Assign { .. }
                | Kind::BooleanAssign { .. }
                | Kind::Reference(_)
                | Kind::ScalarReference { .. } => None,
            };
            let Some(next) = next else {
                return Ok(None);
            };
            output.extend(next);
        }
        Ok(Some(output))
    }

    fn object(
        &mut self,
        rule: &str,
        id: &str,
        depth: usize,
    ) -> Result<Option<Vec<RenderedToken>>, String> {
        self.step(depth)?;
        let grammar = self.grammar;
        let rule = grammar
            .rules
            .get(rule)
            .ok_or("Missing containment parser rule")?;
        if rule.datatype.is_some() || rule.fragment {
            return Err("Object serialization requires an object parser rule".into());
        }
        let object = self.starting_object(id, rule.has_current_actions(), depth + 1)?;
        if !rule.uses_current_flow(grammar)
            && !rule.is_dispatch(grammar)
            && rule.construction_class() != object.class
        {
            return Ok(None);
        }
        if !self.visited.insert(object.id.clone()) {
            return Ok(None);
        }
        let mut frame = Frame {
            object,
            positions: BTreeMap::new(),
        };
        let hidden = self.hidden;
        if let Some(policy) = rule.hidden.as_deref() {
            self.hidden = Some(policy);
        }
        let result = self.expression(&rule.body, &mut frame, depth + 1);
        self.hidden = hidden;
        let result = result?;
        Ok(if frame.object.id == id { result } else { None })
    }

    fn starting_object(
        &mut self,
        id: &str,
        captures: bool,
        depth: usize,
    ) -> Result<&'a Element, String> {
        let target = *self
            .objects
            .get(id)
            .ok_or("Missing serialized model object")?;
        let mut starts = vec![target];
        if captures {
            let mut at = 0;
            while at < starts.len() {
                self.step(depth)?;
                let parent = starts[at];
                for (name, field) in self.schema.features(&parent.class)? {
                    if field.containment {
                        let Some(value) = self.feature_value(parent, &name) else {
                            continue;
                        };
                        if !self.selected(parent, &name, None, value)? {
                            continue;
                        }
                        let ids: Vec<_> = match value {
                            Value::String(id) => vec![id.as_str()],
                            Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
                            _ => vec![],
                        };
                        for (index, id) in ids.into_iter().enumerate() {
                            if !self.selected(
                                parent,
                                &name,
                                Some(index),
                                &Value::String(id.into()),
                            )? || self.state.is_some_and(|state| !state.retained.contains(id))
                            {
                                continue;
                            }
                            let object =
                                *self.objects.get(id).ok_or("Missing capture descendant")?;
                            if starts.iter().any(|e| e.id == object.id) {
                                continue;
                            }
                            starts.push(object);
                        }
                    }
                }
                at += 1;
            }
        }
        let selected = self.branch(starts.len())?;
        Ok(starts[selected])
    }
    fn contains_capture(&mut self, expr: &Expr, depth: usize) -> Result<bool, String> {
        self.step(depth)?;
        match &expr.kind {
            Kind::Capture { .. } => Ok(true),
            Kind::Sequence(items) | Kind::Alternative(items) => {
                for item in items {
                    if self.contains_capture(item, depth + 1)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Kind::SyntacticPredicate(value) => self.contains_capture(value, depth + 1),
            _ => Ok(false),
        }
    }
    fn inline_object(
        &mut self,
        expr: &Expr,
        id: &str,
        depth: usize,
    ) -> Result<Option<Vec<RenderedToken>>, String> {
        let captures = self.contains_capture(expr, depth + 1)?;
        let object = self.starting_object(id, captures, depth + 1)?;
        if !self.visited.insert(object.id.clone()) {
            return Ok(None);
        }
        let mut frame = Frame {
            object,
            positions: BTreeMap::new(),
        };
        let result = self.expression(expr, &mut frame, depth + 1)?;
        Ok(if frame.object.id == id { result } else { None })
    }

    fn branch(&mut self, options: usize) -> Result<usize, String> {
        self.step(0)?;
        if options == 0 {
            return Err("No serialization alternative".into());
        }
        let selected = self.choices.get(self.cursor).copied().unwrap_or(0);
        if selected >= options {
            return Err("Invalid serialization search continuation".into());
        }
        self.trace.push((selected, options));
        self.cursor += 1;
        Ok(selected)
    }
    fn advance(&mut self) -> bool {
        while let Some((selected, options)) = self.trace.pop() {
            if selected + 1 < options {
                self.choices = self.trace.iter().map(|(choice, _)| *choice).collect();
                self.choices.push(selected + 1);
                return true;
            }
        }
        false
    }

    fn hint_count(
        &mut self,
        expr: &Expr,
        frame: &Frame<'_>,
        depth: usize,
    ) -> Result<usize, String> {
        self.step(depth)?;
        if let Some(count) = expr.prediction_path.as_ref().and_then(|path| {
            self.hints.and_then(|hints| {
                hints
                    .counts
                    .get(&(HintOwner::Object(frame.object.id.clone()), path.clone()))
            })
        }) {
            return Ok(*count);
        }
        let children: &[Expr] = match &expr.kind {
            Kind::Sequence(items) | Kind::Alternative(items) => items,
            Kind::Assign { value, .. }
            | Kind::BooleanAssign { value, .. }
            | Kind::SyntacticPredicate(value) => std::slice::from_ref(value),
            _ => &[],
        };
        let mut count = 0;
        for child in children {
            let n = self.hint_count(child, frame, depth + 1)?;
            if n > 0 {
                count = if count == 0 { n } else { count.min(n) };
            }
        }
        Ok(count)
    }
    fn has_value(&mut self, expr: &Expr, frame: &Frame<'_>, depth: usize) -> Result<bool, String> {
        self.step(depth)?;
        match &expr.kind {
            Kind::Assign { feature, .. } | Kind::BooleanAssign { feature, .. } => {
                let Ok(effective) = self.schema.effective_feature(&frame.object.class, feature)
                else {
                    return Ok(false);
                };
                let field = effective.feature;
                Ok(self
                    .feature_value(frame.object, feature)
                    .is_some_and(|value| value != &field.default)
                    || (field.unsettable
                        && !field.derived
                        && self.model.is_set(self.schema, &frame.object.id, feature)?))
            }
            Kind::Sequence(items) | Kind::Alternative(items) => {
                for item in items {
                    if self.has_value(item, frame, depth + 1)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Kind::SyntacticPredicate(value) => self.has_value(value, frame, depth + 1),
            Kind::Capture { .. } => Ok(true),
            Kind::Call(name) => {
                if let Some(rule) = self.grammar.rules.get(name).filter(|r| r.fragment) {
                    self.has_value(&rule.body, frame, depth + 1)
                } else {
                    Ok(true)
                }
            }
            _ => Ok(false),
        }
    }

    fn witness(&mut self, name: &str, depth: usize) -> Result<String, String> {
        self.step(depth)?;
        if let Some(terminal) = self.grammar.terminals.get(name) {
            return self.lexical_witness(&terminal.body, depth + 1);
        }
        if let Some(rule) = self.grammar.rules.get(name) {
            return self.syntax_witness(&rule.body, depth + 1);
        }
        match name {
            "ID" => Ok("a".into()),
            "INT" => Ok("0".into()),
            "STRING" => Ok("\"\"".into()),
            _ => Err(format!("No unassigned spelling for {name}")),
        }
    }
    fn syntax_witness(&mut self, expr: &Expr, depth: usize) -> Result<String, String> {
        self.step(depth)?;
        if matches!(expr.cardinality, '?' | '*') {
            return Ok(String::new());
        }
        match &expr.kind {
            Kind::Literal(text) => Ok(text.clone()),
            Kind::Call(name) => self.witness(name, depth + 1),
            Kind::SyntacticPredicate(value) => self.syntax_witness(value, depth + 1),
            Kind::Sequence(items) => {
                let mut text = String::new();
                for item in items {
                    let next = self.syntax_witness(item, depth + 1)?;
                    append(&mut text, &next, self.limits.output_bytes)?;
                }
                Ok(text)
            }
            Kind::Alternative(items) => {
                let i = self.branch(items.len())?;
                self.syntax_witness(&items[i], depth + 1)
            }
            _ => Err("Unassigned datatype spelling requires a value policy".into()),
        }
    }
    fn lexical_witness(
        &mut self,
        expr: &crate::grammar::TerminalExpression,
        depth: usize,
    ) -> Result<String, String> {
        use crate::grammar::TerminalOperation as Operation;
        self.step(depth)?;
        if matches!(expr.cardinality, '?' | '*') {
            return Ok(String::new());
        }
        match &expr.kind {
            Operation::Literal(units) | Operation::Until(units) => String::from_utf16(units)
                .map_err(|_| {
                    "Unpaired UTF-16 terminal requires an explicit spelling policy".into()
                }),
            Operation::Range(start, end) => (*start..=*end)
                .find_map(|u| char::from_u32(u.into()))
                .map(|c| c.to_string())
                .ok_or_else(|| "No scalar terminal witness".into()),
            Operation::Wildcard => Ok("a".into()),
            Operation::Eof => Ok(String::new()),
            Operation::NotCharacters(excluded) | Operation::Negated { excluded, .. } => {
                // Every complement interval starts at zero or just after an
                // excluded interval. Include readable defaults, then those
                // boundaries; do not restrict a legal witness to ASCII.
                let mut candidates = vec![97, 48, 95, 0xA0, 32];
                for (_, end) in excluded {
                    self.step(depth)?;
                    if let Some(next) = end.checked_add(1) {
                        candidates.push(next);
                    }
                }
                candidates.extend([0xE000, 0]);
                let mut seen = BTreeSet::new();
                let mut allowed = Vec::new();
                for unit in candidates {
                    self.step(depth)?;
                    if !seen.insert(unit) {
                        continue;
                    }
                    let Some(character) = char::from_u32(unit.into()) else {
                        continue;
                    };
                    let mut accepted = true;
                    for (start, end) in excluded {
                        self.step(depth)?;
                        accepted &= !(start <= &unit && &unit <= end);
                    }
                    if accepted {
                        allowed.push(character);
                    }
                }
                let choice = self.branch(allowed.len())?;
                Ok(allowed[choice].to_string())
            }
            Operation::Call {
                body: Some(body), ..
            } => self.lexical_witness(body, depth + 1),
            Operation::Call { name, .. } => self.witness(name, depth + 1),
            Operation::UntilOperand { operand, .. } => self.lexical_witness(operand, depth + 1),
            Operation::Sequence(items) => {
                let mut text = String::new();
                for item in items {
                    let next = self.lexical_witness(item, depth + 1)?;
                    append(&mut text, &next, self.limits.output_bytes)?;
                }
                Ok(text)
            }
            Operation::Alternative(items) => {
                let i = self.branch(items.len())?;
                self.lexical_witness(&items[i], depth + 1)
            }
        }
    }

    fn expression(
        &mut self,
        expr: &Expr,
        frame: &mut Frame<'a>,
        depth: usize,
    ) -> Result<Option<Vec<RenderedToken>>, String> {
        self.step(depth)?;
        let baseline = frame.clone();
        let visited = self.visited.clone();
        let unassigned = self.unassigned_positions.clone();
        let result = match expr.cardinality {
            '1' => self.once(expr, frame, depth + 1),
            '?' if self.branch(2)?
                == usize::from(
                    self.hint_count(expr, frame, depth + 1)? > 0
                        || self.has_value(expr, frame, depth + 1)?,
                ) =>
            {
                Ok(Some(Vec::new()))
            }
            '?' => match self.once(expr, frame, depth + 1)? {
                Some(tokens) => Ok(Some(tokens)),
                None => {
                    *frame = baseline.clone();
                    self.visited = visited.clone();
                    Ok(Some(Vec::new()))
                }
            },
            '*' | '+' => {
                let mut output = Vec::new();
                let mut count = 0;
                loop {
                    let before = frame.clone();
                    if self.branch(2)? == 1 {
                        break;
                    }
                    let objects = self.visited.clone();
                    match self.once(expr, frame, depth + 1)? {
                        Some(tokens) => {
                            if before.positions == frame.positions
                                && before.object.id == frame.object.id
                                && objects == self.visited
                            {
                                let required = self
                                    .hint_count(expr, frame, depth + 1)?
                                    .max(usize::from(expr.cardinality == '+'));
                                if count < required {
                                    output.extend(tokens);
                                    count += 1;
                                    continue;
                                }
                                break;
                            }
                            count += 1;
                            output.extend(tokens);
                        }
                        None => {
                            *frame = before;
                            self.visited = objects;
                            break;
                        }
                    }
                }
                Ok((expr.cardinality == '*' || count > 0).then_some(output))
            }
            _ => Err("Unsupported serialization cardinality".into()),
        };
        if !matches!(result, Ok(Some(_))) {
            *frame = baseline;
            self.visited = visited;
            self.unassigned_positions = unassigned;
        }
        result
    }

    fn once(
        &mut self,
        expr: &Expr,
        frame: &mut Frame<'a>,
        depth: usize,
    ) -> Result<Option<Vec<RenderedToken>>, String> {
        self.step(depth)?;
        match &expr.kind {
            Kind::Literal(text) => {
                self.charge_text(text.len())?;
                Ok(Some(vec![self.token(text.clone())?]))
            }
            Kind::Sequence(items) => {
                let mut output = Vec::new();
                for item in items {
                    let Some(tokens) = self.expression(item, frame, depth + 1)? else {
                        return Ok(None);
                    };
                    output.extend(tokens);
                }
                Ok(Some(output))
            }
            Kind::Alternative(items) => {
                if items.is_empty() {
                    return Ok(None);
                }
                let selected = self.branch(items.len())?;
                let mut order = Vec::new();
                for (i, item) in items.iter().enumerate() {
                    order.push((self.hint_count(item, frame, depth + 1)? == 0, i));
                }
                order.sort();
                self.expression(&items[order[selected].1], frame, depth + 1)
            }
            Kind::SyntacticPredicate(value) => self.expression(value, frame, depth + 1),
            Kind::Action(class) => Ok((frame.object.class == *class).then(Vec::new)),
            Kind::Capture {
                class,
                feature,
                many,
            } => {
                let mut parent = None;
                let candidates: Vec<_> = self.objects.values().copied().collect();
                for candidate in candidates {
                    if candidate.class != *class
                        || self
                            .state
                            .is_some_and(|state| !state.retained.contains(&candidate.id))
                    {
                        continue;
                    }
                    let Some(stored) = self.feature_value(candidate, feature) else {
                        continue;
                    };
                    if !self.selected(candidate, feature, None, stored)? {
                        continue;
                    }
                    let position = match stored {
                        Value::String(id) if !many && id == &frame.object.id => Some(0),
                        Value::Array(ids) if *many => {
                            let mut position = None;
                            for (index, value) in ids.iter().enumerate() {
                                if value.as_str() == Some(&frame.object.id)
                                    && self.selected(candidate, feature, Some(index), value)?
                                {
                                    position = Some(index);
                                    break;
                                }
                            }
                            position
                        }
                        _ => None,
                    };
                    if let Some(position) = position
                        && parent.replace((candidate, position)).is_some()
                    {
                        return Err("Ambiguous capture parent".into());
                    }
                }
                let Some((parent, position)) = parent else {
                    return Ok(None);
                };
                if !self.visited.insert(parent.id.clone()) {
                    return Ok(None);
                }
                frame.positions.insert(
                    (parent.id.clone(), feature.clone()),
                    position.checked_add(1).ok_or("Capture index overflow")?,
                );
                frame.object = parent;
                Ok(Some(Vec::new()))
            }
            Kind::Call(name) => {
                if let Some(rule) = self
                    .grammar
                    .rules
                    .get(name)
                    .filter(|r| r.datatype.is_none())
                {
                    if !rule.fragment
                        && !rule.uses_current_flow(self.grammar)
                        && !rule.is_dispatch(self.grammar)
                        && rule.construction_class() != frame.object.class
                    {
                        return Ok(None);
                    }
                    let key = (name.clone(), frame.object.id.clone());
                    if !self.active.insert(key.clone()) {
                        return Ok(None);
                    }
                    let hidden = self.hidden;
                    if let Some(policy) = rule.hidden.as_deref() {
                        self.hidden = Some(policy);
                    }
                    let result = if rule.fragment {
                        self.expression(&rule.body, frame, depth + 1)
                    } else {
                        self.visited.remove(&frame.object.id);
                        self.object(name, &frame.object.id, depth + 1)
                    };
                    self.hidden = hidden;
                    self.active.remove(&key);
                    result
                } else {
                    let text = self.unassigned_spelling(
                        expr,
                        name,
                        Some(frame.object),
                        &frame.object.resource,
                        depth + 1,
                    )?;
                    Ok(Some(vec![self.token(text)?]))
                }
            }
            Kind::Assign {
                feature,
                many,
                value,
            } => {
                let Ok(effective) = self.schema.effective_feature(&frame.object.class, feature)
                else {
                    return Ok(None);
                };
                let field = effective.feature;
                if *many != (field.upper != 1) {
                    return Err("Unsupported assignment in canonical serialization".into());
                }
                let mut index = frame
                    .positions
                    .get(&(frame.object.id.clone(), feature.clone()))
                    .copied()
                    .unwrap_or(0);
                let stored = self
                    .feature_value(frame.object, feature)
                    .ok_or("Missing serialized feature value")?;
                if !self.selected(frame.object, feature, None, stored)? {
                    return Ok(None);
                }
                let next = if *many {
                    let values = stored
                        .as_array()
                        .ok_or("Serialized collection is not an array")?;
                    while let Some(value) = values.get(index) {
                        if self.selected(frame.object, feature, Some(index), value)? {
                            break;
                        }
                        index = index
                            .checked_add(1)
                            .ok_or("Serialized collection index overflow")?;
                    }
                    values.get(index)
                } else {
                    (index == 0 && !stored.is_null()).then_some(stored)
                };
                let Some(next) = next else { return Ok(None) };
                if field.unsettable
                    && !field.derived
                    && !self.model.is_set(self.schema, &frame.object.id, feature)?
                {
                    return Ok(None);
                }
                frame
                    .positions
                    .insert((frame.object.id.clone(), feature.clone()), index + 1);
                if field.containment {
                    let id = next.as_str().ok_or("Containment target is not an ID")?;
                    return if let Kind::Call(rule) = &value.kind {
                        self.object(rule, id, depth + 1)
                    } else {
                        self.inline_object(value, id, depth + 1)
                    };
                }
                let target = if field.reference {
                    Some(
                        *self
                            .objects
                            .get(next.as_str().ok_or("Reference target is not an ID")?)
                            .ok_or("Canonical references require an included target")?,
                    )
                } else {
                    None
                };
                self.value(value, frame.object, feature, index, next, target)
            }
            Kind::BooleanAssign { feature, value } => {
                let stored = self
                    .feature_value(frame.object, feature)
                    .ok_or("Missing Boolean serialization value")?;
                if !self.selected(frame.object, feature, None, stored)? {
                    return Ok(None);
                }
                if frame
                    .positions
                    .contains_key(&(frame.object.id.clone(), feature.clone()))
                    || self
                        .feature_value(frame.object, feature)
                        .and_then(Value::as_bool)
                        != Some(true)
                {
                    return Ok(None);
                }
                let Kind::Literal(text) = &value.kind else {
                    return Err("Boolean serialization requires a single keyword".into());
                };
                self.charge_text(text.len())?;
                frame
                    .positions
                    .insert((frame.object.id.clone(), feature.clone()), 1);
                Ok(Some(vec![self.token(text.clone())?]))
            }
            Kind::Reference(_) | Kind::ScalarReference { .. } => {
                Err("Unassigned reference is not serializable".into())
            }
        }
    }

    fn value(
        &mut self,
        expr: &Expr,
        owner: &Element,
        feature: &str,
        index: usize,
        value: &Value,
        target: Option<&Element>,
    ) -> Result<Option<Vec<RenderedToken>>, String> {
        if expr.cardinality != '1' {
            return Err("Scalar RHS cardinality is outside the canonical profile".into());
        }
        let terminal = match &expr.kind {
            Kind::Alternative(items) => {
                let i = self.branch(items.len())?;
                return self.value(&items[i], owner, feature, index, value, target);
            }
            Kind::SyntacticPredicate(inner) => {
                return self.value(inner, owner, feature, index, value, target);
            }
            Kind::Sequence(items) if items.len() == 1 => {
                return self.value(&items[0], owner, feature, index, value, target);
            }
            Kind::Call(name)
                if self
                    .grammar
                    .rules
                    .get(name)
                    .is_none_or(|r| r.datatype.is_some()) =>
            {
                name.as_str()
            }
            Kind::Reference(_) => "ID",
            Kind::ScalarReference { rule, .. } => rule.as_str(),
            Kind::Literal(text) if value.as_str() == Some(text) => {
                self.charge_text(text.len())?;
                return Ok(Some(vec![self.token(text.clone())?]));
            }
            Kind::Literal(_) => return Ok(None),
            _ => {
                return Err(
                    "Canonical scalar serialization requires a terminal call or reference".into(),
                );
            }
        };
        let text = (self.spelling)(TerminalSpelling {
            owner,
            feature,
            terminal,
            index,
            value,
            target,
        })?;
        if text.is_empty() {
            return Err("Serialization spelling is empty".into());
        }
        self.charge_text(text.len())?;
        Ok(Some(vec![self.token(text)?]))
    }
}

#[derive(Clone)]
struct RenderedToken {
    text: String,
    space: bool,
    newline: bool,
    separator: Option<String>,
}
fn format_tokens(tokens: &[RenderedToken], limit: usize) -> Result<String, String> {
    let mut text = String::new();
    let mut indent = 0usize;
    let mut line_start = true;
    for (index, token) in tokens.iter().enumerate() {
        if token.text == "}" {
            indent = indent.saturating_sub(1);
            if !line_start && token.newline {
                append(&mut text, "\n", limit)?;
                line_start = true;
            }
        }
        if line_start && token.space {
            for _ in 0..indent {
                append(&mut text, "  ", limit)?;
            }
        } else if !line_start && token.text != ";" {
            if token.space {
                append(&mut text, " ", limit)?;
            } else if let Some(separator) = &token.separator {
                append(&mut text, separator, limit)?;
            }
        }
        append(&mut text, &token.text, limit)?;
        line_start = false;
        if token.text == "{" {
            indent += 1;
        }
        // Whitespace preceding the next token is governed by that actual rule's
        // hidden policy, including local hidden() fragments.
        let next_newline = tokens.get(index + 1).is_some_and(|next| next.newline);
        if next_newline && matches!(token.text.as_str(), "{" | "}" | ";") {
            append(&mut text, "\n", limit)?;
            line_start = true;
        }
    }
    Ok(text)
}

fn append(output: &mut String, text: &str, limit: usize) -> Result<(), String> {
    if output
        .len()
        .checked_add(text.len())
        .is_none_or(|length| length > limit)
    {
        return Err("Serialization output byte budget exceeded".into());
    }
    output
        .try_reserve(text.len())
        .map_err(|_| "Serialization output allocation failed")?;
    output.push_str(text);
    Ok(())
}
