//! Native Xtext metamodel inference. Handwritten language algorithms consume the
//! shared source-bound syntax tree and materialize through the Rcore compiler.
use crate::compiler;
use crate::grammar;
use crate::grammar::SourceLimits;
use crate::grammar::syntax;
use crate::grammar::syntax::Element;
use crate::grammar::syntax::ElementKind;
use crate::grammar::syntax::Located;
use crate::grammar::syntax::RuleKind;
use crate::grammar::syntax::Syntax;
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use intersect_model::definitions::ecore::ECORE;
use intersect_model::definitions::resources::ResourceSet;
use intersect_model::definitions::schema::Diagnostic;
use intersect_model::definitions::schema::DiagnosticOrigin;
use intersect_model::definitions::schema::Schema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[path = "inference_emit.rs"]
mod emit;
#[path = "inference_fragments.rs"]
mod fragments;
#[path = "hidden_policy.rs"]
mod hidden_policy;
#[path = "ordinary.rs"]
pub mod ordinary;
#[path = "parameter_program.rs"]
mod parameter_program;
#[path = "inference_parameters.rs"]
mod parameters;
#[path = "inference_providers.rs"]
pub mod providers;
#[path = "inference_sources.rs"]
mod sources;

type Result<T> = std::result::Result<T, Diagnostic>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TypeId {
    pub namespace: String,
    pub name: String,
}
impl TypeId {
    fn builtin(name: &str) -> Self {
        Self {
            namespace: ECORE.into(),
            name: name.into(),
        }
    }
    pub fn identity(&self) -> String {
        format!("{}#//{}", self.namespace, self.name)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Origin {
    pub kind: String,
    pub span: SourceSpan,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Feature {
    pub name: String,
    pub target: TypeId,
    pub reference: bool,
    pub containment: bool,
    pub many: bool,
    pub unique: bool,
    pub origins: Vec<Origin>,
}
impl Feature {
    fn same_shape(&self, other: &Self) -> bool {
        self.name == other.name
            && self.target == other.target
            && self.reference == other.reference
            && self.containment == other.containment
            && self.many == other.many
            && self.unique == other.unique
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Literal {
    pub name: String,
    pub literal: String,
    pub value: i32,
    pub origins: Vec<Origin>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ClassifierKind {
    Class,
    Enumeration,
    Datatype,
}
#[derive(Clone, Debug, Serialize)]
pub struct Classifier {
    pub id: TypeId,
    pub kind: ClassifierKind,
    pub parents: Vec<TypeId>,
    pub features: Vec<Feature>,
    pub literals: Vec<Literal>,
    /// Explicit TypeRefs and action refs, corresponding to upstream source adapters.
    pub origins: Vec<Origin>,
    /// All introducing rule/name declarations, including implicit return types.
    pub declarations: Vec<SourceSpan>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Metamodel {
    pub name: String,
    pub namespace: String,
    pub alias: Option<String>,
    pub classifiers: Vec<Classifier>,
}
#[derive(Clone, Debug, Serialize)]
pub struct GeneratedOrigin {
    pub generated: SourceSpan,
    pub original: Vec<SourceSpan>,
}
/// Source-bound return classification, separate from executable grammar DTOs.
#[derive(Clone, Debug, Serialize)]
pub struct RuleBinding {
    /// Declaration positions are identities even when authored names repeat.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Located<String>>,
    pub grammar: String,
    pub declaration: SourceSpan,
    pub kind: RuleKind,
    /// No return classifier exists for a wildcard parser fragment.
    pub target: Option<TypeId>,
    pub wildcard: bool,
    pub implicit: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct GrammarInference {
    #[serde(skip)]
    parameterized: bool,
    pub source: SourceId,
    pub name: String,
    pub metamodels: Vec<Metamodel>,
    pub generated_sources: Vec<DefinitionSource>,
    #[serde(skip)]
    generated_origins: Vec<GeneratedOrigin>,
    pub rules: BTreeMap<String, RuleBinding>,
    #[serde(skip)]
    scope: InferenceScope,
}
/// Additional correspondence to a sealed inherited feature, without rewriting it.
#[derive(Clone, Debug, Serialize)]
pub struct InheritedOrigin {
    pub declaration: intersect_model::definitions::resources::DeclarationId,
    pub origin: Origin,
}
/// Complete inferred definitions, not an executable grammar or provider registry.
#[derive(Clone, Debug, Serialize)]
pub struct InferredMetamodels {
    pub original: DefinitionSource,
    pub metamodels: Vec<Metamodel>,
    pub generated_sources: Vec<DefinitionSource>,
    pub origins: Vec<GeneratedOrigin>,
    pub bundle: DefinitionBundle,
    pub entry_source: SourceId,
    pub namespace: String,
    pub schema: Schema,
    pub materialized_types: Vec<intersect_model::definitions::generic_types::RuntimeType>,
    pub grammars: Vec<GrammarInference>,
    pub effective_rules: BTreeMap<String, RuleBinding>,
    pub inherited_origins: Vec<InheritedOrigin>,
    #[serde(skip)]
    diagnostic_origins: Vec<GeneratedOrigin>,
}
// A declaration library has no structural selection. It carries the same
// source-bound classifications into its child without a fabricated Schema.
struct LibraryInference {
    source: SourceId,
    bundle: DefinitionBundle,
    grammars: Vec<GrammarInference>,
    effective_rules: BTreeMap<String, RuleBinding>,
    classifiers: BTreeMap<TypeId, ClassifierKind>,
}
enum PreparedInference {
    Model(Box<InferredMetamodels>),
    Library(LibraryInference),
}
impl PreparedInference {
    fn model(&self) -> Option<&InferredMetamodels> {
        match self {
            Self::Model(model) => Some(model),
            Self::Library(_) => None,
        }
    }
    fn grammars(&self) -> &[GrammarInference] {
        match self {
            Self::Model(model) => &model.grammars,
            Self::Library(library) => &library.grammars,
        }
    }
    fn bundle(&self) -> &DefinitionBundle {
        match self {
            Self::Model(model) => &model.bundle,
            Self::Library(library) => &library.bundle,
        }
    }
    fn effective_rules(&self) -> &BTreeMap<String, RuleBinding> {
        match self {
            Self::Model(model) => &model.effective_rules,
            Self::Library(library) => &library.effective_rules,
        }
    }
    fn inherited_origins(&self) -> &[InheritedOrigin] {
        match self {
            Self::Model(model) => &model.inherited_origins,
            Self::Library(_) => &[],
        }
    }
    fn into_model(self) -> Result<InferredMetamodels> {
        match self {
            Self::Model(model) => Ok(*model),
            Self::Library(library) => Err(Diagnostic::at_source(
                &library.source,
                0,
                1,
                "Selected grammar is a declaration library; a generated model context is required",
            )),
        }
    }
}
#[derive(Clone, Copy)]
struct Location {
    package: usize,
    classifier: usize,
}
#[derive(Clone)]
struct Info {
    kind: ClassifierKind,
    generated: Option<Location>,
    imported: Option<(usize, String)>,
}
#[derive(Clone, Debug)]
struct InferenceScope {
    source: SourceId,
    aliases: BTreeMap<String, Vec<String>>,
    rules: BTreeMap<String, RuleBinding>,
    fragments: BTreeMap<String, Element>,
    referenced_ecore: bool,
}
// A schema storage key is never an authored classifier name. Resolve only an
// exact retained declaration through the existing source resolver. This is a
// synchronous reconstruction tail, not a cooperative allocation claim.
fn source_datatype_identity(
    bundle: &DefinitionBundle,
    declaration: &intersect_model::definitions::resources::DeclarationId,
    expected_name: &str,
    limits: SourceLimits,
) -> std::result::Result<TypeId, String> {
    let standard = compiler::standard_datatype_source();
    let standard_bundle;
    let selected = if declaration.source == standard.id {
        standard_bundle = DefinitionBundle {
            version: 1,
            sources: vec![standard.clone()],
            namespaces: vec![],
        };
        &standard_bundle
    } else {
        bundle
    };
    let resources = ResourceSet::new_with_limits(
        selected,
        intersect_model::definitions::ecore::ParseLimits {
            source_bytes: limits.source_bytes,
        },
    )
    .map_err(|error| error.to_string())?;
    let node = resources.node(declaration)?;
    if node.name.local != "eClassifiers"
        || !node.attributes.iter().any(|attribute| {
            attribute.type_name.as_ref().is_some_and(|kind| {
                kind.namespace.as_deref() == Some(ECORE) && kind.local == "EDataType"
            })
        })
    {
        return Err("Feature datatype declaration is not an exact EDataType".into());
    }
    let name = node
        .attributes
        .iter()
        .find(|attribute| attribute.name.namespace.is_none() && attribute.name.local == "name")
        .ok_or("Feature datatype declaration has no name")?;
    if name.value != expected_name {
        return Err("Feature datatype name differs from its source declaration".into());
    }
    let mut owner: Option<(&str, usize)> = None;
    for (namespace, package) in resources.packages(&declaration.source)? {
        if declaration.path.len() <= package.path.len()
            || !declaration.path.starts_with(&package.path)
        {
            continue;
        }
        match owner {
            Some((_, depth)) if depth > package.path.len() => {}
            Some((prior, depth)) if depth == package.path.len() && prior != namespace => {
                return Err("Feature datatype has ambiguous containing package identity".into());
            }
            _ => owner = Some((namespace, package.path.len())),
        }
    }
    let (namespace, _) = owner.ok_or("Feature datatype has no retained containing package")?;
    Ok(TypeId {
        namespace: namespace.into(),
        name: name.value.clone(),
    })
}

struct Engine<'a> {
    syntax: &'a Syntax,
    source: &'a DefinitionSource,
    bundle: &'a DefinitionBundle,
    limits: SourceLimits,
    context: String,
    packages: Vec<Metamodel>,
    aliases: BTreeMap<String, Vec<String>>,
    types: BTreeMap<TypeId, Info>,
    rules: BTreeMap<String, TypeId>,
    imports: Vec<Schema>,
    skeleton: Option<Schema>,
    inherited: Option<&'a PreparedInference>,
    inherited_origins: Vec<InheritedOrigin>,
    fragment_scope: Option<InferenceScope>,
    active_fragments: BTreeSet<(SourceId, String)>,
    conversions: Option<&'a intersect_model::definitions::conversion::ConversionRegistry<'a>>,
    contextual_first_rule: bool,
    scoped_terminal_returns: bool,
    scoped_hidden_tokens: bool,
    durable_documents: bool,
    imported_assignment_compatibility: bool,
    unordered_groups: bool,
}
impl Engine<'_> {
    fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(
            &self.source.id.uri,
            self.syntax.start,
            self.syntax.end,
            message,
        )
        .with_context(&self.source.id)
    }
    fn at<T>(&self, node: &Located<T>, message: impl Into<String>) -> Diagnostic {
        Diagnostic::from_span(self.span(node), message)
    }
    fn span<T>(&self, node: &Located<T>) -> SourceSpan {
        SourceSpan {
            source: self.current_source().clone(),
            start: node.start,
            end: node.end,
        }
    }
    fn origin<T>(&self, node: &Located<T>, kind: &str) -> Origin {
        Origin {
            kind: kind.into(),
            span: self.span(node),
        }
    }
    fn record(&self, id: &TypeId) -> Option<&Classifier> {
        let location = self.types.get(id)?.generated?;
        self.packages
            .get(location.package)?
            .classifiers
            .get(location.classifier)
    }
    fn record_mut(&mut self, id: &TypeId) -> Option<&mut Classifier> {
        let location = self.types.get(id)?.generated?;
        self.packages
            .get_mut(location.package)?
            .classifiers
            .get_mut(location.classifier)
    }
    fn class(&self, id: &TypeId) -> bool {
        id == &TypeId::builtin("EObject")
            || self
                .types
                .get(id)
                .is_some_and(|info| info.kind == ClassifierKind::Class)
    }
    fn default_namespace(&self) -> Result<String> {
        let candidates = self
            .packages
            .iter()
            .filter(|p| p.alias.is_none())
            .collect::<Vec<_>>();
        if candidates.is_empty() && self.packages.is_empty() {
            if let Some(namespaces) = self.current_aliases().get("")
                && let [namespace] = namespaces.as_slice()
            {
                return Ok(namespace.clone());
            }
            return Err(self.error("Imported implicit returns require one unaliased metamodel"));
        }
        if candidates.len() != 1 {
            return Err(self.error("Inference requires one unaliased generated package"));
        }
        Ok(candidates[0].namespace.clone())
    }
    fn resolve<T>(
        &mut self,
        name: &Located<String>,
        create: Option<ClassifierKind>,
        declaration: &Located<T>,
    ) -> Result<TypeId> {
        let (namespaces, local) = if let Some((alias, local)) = name.value.split_once("::") {
            (
                self.current_aliases()
                    .get(alias)
                    .cloned()
                    .ok_or_else(|| self.at(name, format!("Unknown metamodel alias {alias}")))?,
                local,
            )
        } else {
            let found = self
                .types
                .keys()
                .filter(|id| {
                    id.name == name.value
                        && self
                            .current_aliases()
                            .values()
                            .flatten()
                            .any(|uri| uri == &id.namespace)
                })
                .cloned()
                .collect::<Vec<_>>();
            if found.len() > 1 {
                return Err(self.at(name, format!("Ambiguous classifier {}", name.value)));
            }
            if let Some(id) = found.first() {
                let id = id.clone();
                self.require_kind(&id, create, name)?;
                self.add_declaration(&id, name, declaration);
                return Ok(id);
            }
            (vec![self.default_namespace()?], name.value.as_str())
        };
        let candidates = namespaces
            .iter()
            .map(|namespace| TypeId {
                namespace: namespace.clone(),
                name: local.into(),
            })
            .filter(|id| self.types.contains_key(id))
            .collect::<Vec<_>>();
        let id = match candidates.as_slice() {
            [id] => id.clone(),
            [] => {
                let generated = namespaces
                    .iter()
                    .filter(|namespace| self.packages.iter().any(|p| &p.namespace == *namespace))
                    .collect::<Vec<_>>();
                let namespace = match generated.as_slice() {
                    [namespace] => (*namespace).clone(),
                    _ => {
                        return Err(self.at(name, "Unknown or ambiguous generated metamodel alias"));
                    }
                };
                TypeId {
                    namespace,
                    name: local.into(),
                }
            }
            _ => return Err(self.at(name, "Ambiguous metamodel alias classifier")),
        };
        self.bind_type(id, create, name, declaration)
    }
    fn bind_type<T>(
        &mut self,
        id: TypeId,
        create: Option<ClassifierKind>,
        name: &Located<String>,
        declaration: &Located<T>,
    ) -> Result<TypeId> {
        if self.types.contains_key(&id) {
            self.require_kind(&id, create, name)?;
            self.add_declaration(&id, name, declaration);
            return Ok(id);
        }
        let kind =
            create.ok_or_else(|| self.at(name, format!("Unknown classifier {}", id.identity())))?;
        let package = self
            .packages
            .iter()
            .position(|p| p.namespace == id.namespace)
            .ok_or_else(|| self.at(name, "Cannot add a classifier to an imported package"))?;
        if kind == ClassifierKind::Datatype {
            return Err(self.at(name, "Pinned Xtext cannot create a new terminal datatype"));
        }
        if self.types.len() >= 16_384 {
            return Err(self.at(name, "Inferred classifier budget exceeded"));
        }
        let classifier = self.packages[package].classifiers.len();
        self.packages[package].classifiers.push(Classifier {
            id: id.clone(),
            kind,
            parents: vec![],
            features: vec![],
            literals: vec![],
            origins: vec![],
            declarations: vec![],
        });
        self.types.insert(
            id.clone(),
            Info {
                kind,
                generated: Some(Location {
                    package,
                    classifier,
                }),
                imported: None,
            },
        );
        self.add_declaration(&id, name, declaration);
        Ok(id)
    }
    fn require_kind<T>(
        &self,
        id: &TypeId,
        kind: Option<ClassifierKind>,
        node: &Located<T>,
    ) -> Result<()> {
        if let Some(kind) = kind {
            let actual = self
                .types
                .get(id)
                .ok_or_else(|| self.at(node, "Missing classifier input"))?
                .kind;
            if actual != kind
                && !(kind == ClassifierKind::Datatype && actual == ClassifierKind::Enumeration)
            {
                return Err(self.at(
                    node,
                    format!("Incompatible classifier category for {}", id.identity()),
                ));
            }
        }
        Ok(())
    }
    fn add_declaration<T>(
        &mut self,
        id: &TypeId,
        name: &Located<String>,
        declaration: &Located<T>,
    ) {
        let source = self.span(declaration);
        // Implicit rule names have no upstream TypeRef adapter; keep their origin
        // in the independent declaration correspondence instead.
        let explicit = name.start != declaration.start || name.end != declaration.end;
        let origin = self.origin(name, "TypeRef");
        if let Some(record) = self.record_mut(id) {
            if !record.declarations.contains(&source) {
                record.declarations.push(source);
            }
            if explicit && !record.origins.contains(&origin) {
                record.origins.push(origin);
            }
        }
    }
    fn initialize(&mut self) -> Result<()> {
        let resources = ResourceSet::new_with_limits(
            self.bundle,
            intersect_model::definitions::ecore::ParseLimits {
                source_bytes: self.limits.source_bytes,
            },
        )
        .map_err(|e| self.error(e.to_string()))?;
        let mut reachable = BTreeSet::new();
        let mut pending = self.source.dependencies.clone();
        if let Some(parent) = self.inherited {
            pending.extend(
                parent
                    .grammars()
                    .iter()
                    .flat_map(|g| g.generated_sources.iter().map(|s| s.id.clone())),
            );
            if let Some(model) = parent.model() {
                let index = self.imports.len();
                let schema = model.schema.clone();
                for (key, declaration) in &schema.declarations {
                    let kind = if schema.classes.contains_key(key) {
                        ClassifierKind::Class
                    } else if schema.enumerations.contains_key(key) {
                        ClassifierKind::Enumeration
                    } else {
                        ClassifierKind::Datatype
                    };
                    self.types.insert(
                        TypeId {
                            namespace: declaration.namespace.clone(),
                            name: declaration.name.clone(),
                        },
                        Info {
                            kind,
                            generated: None,
                            imported: Some((index, key.clone())),
                        },
                    );
                }
                self.imports.push(schema);
            } else if let PreparedInference::Library(library) = parent {
                for (id, kind) in &library.classifiers {
                    self.types.insert(
                        id.clone(),
                        Info {
                            kind: *kind,
                            generated: None,
                            imported: None,
                        },
                    );
                }
            }
            for (name, rule) in parent.effective_rules() {
                if let Some(target) = &rule.target {
                    self.rules.insert(name.clone(), target.clone());
                    self.rules.insert(format!("super::{name}"), target.clone());
                }
            }
        }
        while let Some(id) = pending.pop() {
            if reachable.insert(id.clone()) {
                let source = self.bundle.source(&id).map_err(|e| self.error(e))?;
                pending.extend(source.dependencies.iter().cloned());
            }
        }
        if self.scoped_terminal_returns
            && let Some(parent) = self.inherited
        {
            for grammar in parent.grammars() {
                for (alias, namespaces) in &grammar.scope.aliases {
                    let selected = self.aliases.entry(alias.clone()).or_default();
                    for namespace in namespaces {
                        if !selected.contains(namespace) {
                            selected.push(namespace.clone());
                        }
                    }
                }
            }
        }
        let mut namespaces = BTreeSet::new();
        let mut generated_aliases = BTreeSet::new();
        let syntax = self.syntax;
        for entry in &syntax.metamodels {
            let uri = entry
                .value
                .uri
                .value
                .to_scalar_string()
                .map_err(|e| self.at(entry, e))?;
            let alias = entry.value.alias.as_ref().map_or("", |a| a.value.as_str());
            if !namespaces.insert(uri.clone()) {
                return Err(self.at(entry, "Duplicate metamodel namespace"));
            }
            let selected = self.aliases.entry(alias.into()).or_default();
            if !self.scoped_terminal_returns || !selected.contains(&uri) {
                selected.push(uri.clone());
            }
            if let Some(name) = &entry.value.generated_name {
                if !generated_aliases.insert(alias.to_owned()) {
                    return Err(self.at(entry, "Duplicate generated metamodel alias"));
                }
                self.packages.push(Metamodel {
                    name: name.value.clone(),
                    namespace: uri,
                    alias: entry.value.alias.as_ref().map(|a| a.value.clone()),
                    classifiers: vec![],
                });
            } else if uri == ECORE {
                let document = intersect_model::definitions::ecore::Document::parse(
                    compiler::standard_datatype_source(),
                )
                .map_err(|e| self.at(entry, e.to_string()))?;
                for node in &document.root.children {
                    if node.name.local != "eClassifiers" {
                        continue;
                    }
                    let Some(name) = node
                        .attributes
                        .iter()
                        .find(|a| a.name.namespace.is_none() && a.name.local == "name")
                    else {
                        continue;
                    };
                    let category = node
                        .attributes
                        .iter()
                        .find_map(|a| a.type_name.as_ref())
                        .map(|n| n.local.as_str());
                    let kind = match category {
                        Some("EClass") => ClassifierKind::Class,
                        Some("EEnum") => ClassifierKind::Enumeration,
                        _ => ClassifierKind::Datatype,
                    };
                    self.types.insert(
                        TypeId::builtin(&name.value),
                        Info {
                            kind,
                            generated: None,
                            imported: None,
                        },
                    );
                }
            } else {
                let binding = self.bundle.namespaces.iter().find(|b| b.namespace == uri);
                let mut candidates = vec![];
                for source in &self.bundle.sources {
                    if source.kind == SourceKind::Rcore
                        && resources
                            .packages(&source.id)
                            .map_err(|e| self.at(entry, e))?
                            .contains_key(&uri)
                        && binding.is_none_or(|b| b.source == source.id)
                    {
                        candidates.push(source.id.clone());
                    }
                }
                if candidates.len() != 1 {
                    return Err(self.at(
                        entry,
                        "Imported namespace requires one exact selected Rcore package",
                    ));
                }
                if !reachable.contains(&candidates[0]) {
                    return Err(
                        self.at(entry, "Imported package is not an exact grammar dependency")
                    );
                }
                let inherited_schema =
                    self.inherited
                        .and_then(PreparedInference::model)
                        .filter(|parent| {
                            parent
                                .grammars
                                .iter()
                                .flat_map(|g| &g.generated_sources)
                                .any(|s| s.id == candidates[0])
                        });
                let schema = if let Some(parent) = inherited_schema {
                    parent.schema.clone()
                } else {
                    compiler::compile_structure_bundle_with_limits_observed(
                        self.bundle,
                        &candidates[0],
                        &uri,
                        self.conversions,
                        &Default::default(),
                        intersect_model::definitions::ecore::ParseLimits {
                            source_bytes: self.limits.source_bytes,
                        },
                    )
                    .map_err(|e| self.at(entry, e.to_string()))?
                };
                let index = self.imports.len();
                for (key, declaration) in &schema.declarations {
                    if declaration.namespace != uri {
                        continue;
                    }
                    let kind = if schema.classes.contains_key(key) {
                        ClassifierKind::Class
                    } else if schema.enumerations.contains_key(key) {
                        ClassifierKind::Enumeration
                    } else {
                        ClassifierKind::Datatype
                    };
                    let id = TypeId {
                        namespace: uri.clone(),
                        name: declaration.name.clone(),
                    };
                    self.types.insert(
                        id,
                        Info {
                            kind,
                            generated: None,
                            imported: Some((index, key.clone())),
                        },
                    );
                }
                self.imports.push(schema);
            }
        }
        if syntax.rules.is_empty() {
            return Err(self.error("Pinned Xtext requires at least one authored rule"));
        }
        Ok(())
    }
    fn scalar_body(&self, element: &Element, scalar: &BTreeSet<String>) -> bool {
        match &element.value {
            ElementKind::Keyword(_)
            | ElementKind::Range { .. }
            | ElementKind::Wildcard
            | ElementKind::Eof => true,
            ElementKind::Call { rule, .. } => {
                self.rule_kind(&rule.value) != Some(RuleKind::TerminalFragment)
                    && scalar.contains(&rule.value)
            }
            ElementKind::Group(e)
            | ElementKind::Cardinality { element: e, .. }
            | ElementKind::Predicate { element: e, .. } => self.scalar_body(e, scalar),
            ElementKind::Unordered(items) if self.unordered_groups => {
                items.iter().all(|e| self.scalar_body(e, scalar))
            }
            ElementKind::Sequence(items)
            | ElementKind::Alternatives(items)
            | ElementKind::Guard {
                elements: items, ..
            } => items.iter().all(|e| self.scalar_body(e, scalar)),
            _ => false,
        }
    }
    fn referenced_ecore(&self) -> bool {
        self.syntax.metamodels.iter().any(|entry| {
            entry.value.generated_name.is_none()
                && entry
                    .value
                    .uri
                    .value
                    .to_scalar_string()
                    .is_ok_and(|uri| uri == ECORE)
        }) || self.inherited.is_some_and(|parent| {
            parent
                .grammars()
                .iter()
                .any(|grammar| grammar.scope.referenced_ecore)
        })
    }
    fn infer_types(&mut self) -> Result<()> {
        let syntax = self.syntax;
        let parameterized = parameter_program::requires(syntax)
            || self
                .inherited
                .is_some_and(|parent| parent.grammars().iter().any(|g| g.parameterized));
        let mut names = BTreeSet::new();
        for rule in &syntax.rules {
            if self.durable_documents
                && names
                    .iter()
                    .any(|name: &String| name.eq_ignore_ascii_case(&rule.value.name.value))
            {
                return Err(self.at(
                    rule,
                    "Grammar rule names must be distinct ignoring ASCII case",
                ));
            }
            if !names.insert(rule.value.name.value.clone()) {
                return Err(self.at(rule, "Duplicate grammar rule"));
            }
            if parameterized {
                parameters::check_rule(self, rule)?;
            }
            if rule.value.kind == RuleKind::Fragment {
                fragments::check_actions(self, &rule.value.body)?;
            }
        }
        if !self.contextual_first_rule
            && syntax
                .rules
                .first()
                .is_some_and(|r| r.value.kind == RuleKind::Fragment)
        {
            return Err(self.error("The first rule must not be a parser fragment"));
        }
        let mut scalar = syntax
            .rules
            .iter()
            .filter(|r| {
                matches!(
                    r.value.kind,
                    RuleKind::Terminal | RuleKind::TerminalFragment | RuleKind::Enum
                ) || (r.value.return_type.is_none() && !r.value.wildcard)
            })
            .map(|r| r.value.name.value.clone())
            .collect::<BTreeSet<_>>();
        for (name, target) in &self.rules {
            if !names.contains(name) && !self.class(target) {
                scalar.insert(name.clone());
            }
        }
        for rule in &syntax.rules {
            if let Some(ty) = &rule.value.return_type
                && self
                    .resolve(ty, None, ty)
                    .ok()
                    .and_then(|id| self.types.get(&id))
                    .is_some_and(|info| info.kind != ClassifierKind::Class)
            {
                scalar.insert(rule.value.name.value.clone());
            }
        }
        loop {
            let rejected = syntax
                .rules
                .iter()
                .filter(|r| {
                    matches!(r.value.kind, RuleKind::Parser | RuleKind::Fragment)
                        && !r.value.wildcard
                        && r.value.return_type.is_none()
                        && scalar.contains(&r.value.name.value)
                        && !self.scalar_body(&r.value.body, &scalar)
                })
                .map(|r| r.value.name.value.clone())
                .collect::<Vec<_>>();
            if rejected.is_empty() {
                break;
            }
            for name in rejected {
                scalar.remove(&name);
            }
        }
        for rule in &syntax.rules {
            let r = &rule.value;
            if r.wildcard {
                self.check_fragment_override(rule, None)?;
                continue;
            }
            let kind = match r.kind {
                RuleKind::Enum => ClassifierKind::Enumeration,
                RuleKind::Terminal | RuleKind::TerminalFragment => ClassifierKind::Datatype,
                _ => ClassifierKind::Class,
            };
            let id = if let Some(ty) = &r.return_type {
                // Parser rules may explicitly return imported scalar/enum types.
                let required = if matches!(r.kind, RuleKind::Parser | RuleKind::Fragment) {
                    None
                } else {
                    Some(kind)
                };
                let create = required.or(Some(ClassifierKind::Class));
                let known = self.resolve(ty, None, ty);
                match known {
                    Ok(id) => {
                        self.require_kind(&id, required, ty)?;
                        id
                    }
                    Err(_) => self.resolve(ty, create, ty)?,
                }
            } else if scalar.contains(&r.name.value) && r.kind != RuleKind::Enum {
                if self.scoped_terminal_returns && !self.referenced_ecore() {
                    return Err(self.at(rule, "Implicit scalar return requires a referenced Ecore package in the authored grammar scope"));
                }
                if !self.types.contains_key(&TypeId::builtin("EString")) {
                    return Err(self.at(rule, "Scalar inference needs the pinned Ecore import"));
                }
                TypeId::builtin("EString")
            } else {
                // An implicit local rule introduces its own classifier, even
                // when a parent has a classifier with the same short name.
                let namespace = self.default_namespace()?;
                self.bind_type(
                    TypeId {
                        namespace,
                        name: r.name.value.clone(),
                    },
                    Some(kind),
                    &r.name,
                    &r.name,
                )?
            };
            if let Some(ty) = &r.return_type {
                let origin = self.origin(ty, "TypeRef");
                if let Some(record) = self.record_mut(&id)
                    && !record.origins.contains(&origin)
                {
                    record.origins.push(origin);
                }
            }
            self.check_fragment_override(rule, Some(&id))?;
            if let Some(prior) = self
                .inherited
                .and_then(|p| p.effective_rules().get(&r.name.value))
                .and_then(|p| p.target.as_ref())
            {
                if self.class(&id) && self.class(prior) {
                    self.add_parent(&id, prior, &r.body)?;
                } else if &id != prior {
                    return Err(self.at(rule, "Override changes the inherited datatype/category"));
                }
            }
            self.rules.insert(r.name.value.clone(), id);
        }
        for rule in &syntax.rules {
            self.seed_actions(&rule.value.body)?;
        }
        if parameterized {
            for rule in &syntax.rules {
                parameters::check_authored_actions(self, rule)?;
            }
        }
        for rule in &syntax.rules {
            if rule.value.kind == RuleKind::Enum {
                self.enum_literals(&rule.value.body, &self.rule_type(&rule.value.name)?)?;
            }
        }
        Ok(())
    }
    fn rule_type(&self, name: &Located<String>) -> Result<TypeId> {
        if let Some(scope) = &self.fragment_scope {
            return scope
                .rules
                .get(&name.value)
                .and_then(|rule| rule.target.clone())
                .ok_or_else(|| self.at(name, "Unknown or wildcard value rule in fragment scope"));
        }
        self.rules
            .get(&name.value)
            .cloned()
            .ok_or_else(|| self.at(name, format!("Unknown value rule {}", name.value)))
    }
    fn rule_kind(&self, name: &str) -> Option<RuleKind> {
        self.binding(name).map(|binding| binding.kind)
    }
    fn value_rule_type(&self, name: &Located<String>) -> Result<TypeId> {
        if matches!(
            self.rule_kind(&name.value),
            Some(RuleKind::TerminalFragment | RuleKind::Fragment)
        ) {
            return Err(self.at(name, "Fragments cannot supply assigned parser values"));
        }
        self.rule_type(name)
    }
    fn seed_actions(&mut self, element: &Element) -> Result<()> {
        match &element.value {
            ElementKind::Action { ty, .. } => {
                self.resolve(ty, Some(ClassifierKind::Class), element)?;
            }
            ElementKind::Group(e)
            | ElementKind::Cardinality { element: e, .. }
            | ElementKind::Predicate { element: e, .. }
            | ElementKind::Assignment { value: e, .. } => self.seed_actions(e)?,
            ElementKind::Unordered(items) if self.unordered_groups => {
                for member in items {
                    self.seed_actions(member)?;
                }
            }
            ElementKind::Sequence(items)
            | ElementKind::Alternatives(items)
            | ElementKind::Guard {
                elements: items, ..
            } => {
                for e in items {
                    self.seed_actions(e)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn enum_literals(&mut self, element: &Element, id: &TypeId) -> Result<()> {
        match &element.value {
            ElementKind::Alternatives(items)
            | ElementKind::Sequence(items)
            | ElementKind::Guard {
                elements: items, ..
            } => {
                for e in items {
                    self.enum_literals(e, id)?;
                }
            }
            ElementKind::EnumLiteral { name, keyword } => {
                let spelling = if let Some(value) = keyword {
                    match &value.value {
                        ElementKind::Keyword(s) => {
                            s.to_scalar_string().map_err(|e| self.at(value, e))?
                        }
                        _ => return Err(self.at(value, "Enum inference requires a keyword")),
                    }
                } else {
                    name.value.clone()
                };
                let origin = self.origin(element, "EnumLiteralDeclaration");
                if let Some(record) = self.record_mut(id) {
                    if let Some(literal) = record.literals.iter_mut().find(|l| l.name == name.value)
                    {
                        literal.origins.push(origin);
                    } else {
                        let value = i32::try_from(record.literals.len()).map_err(|_| {
                            Diagnostic::new(
                                "xtext",
                                element.start,
                                element.end,
                                "Enum value overflow",
                            )
                            .with_span(origin.span.clone())
                        })?;
                        record.literals.push(Literal {
                            name: name.value.clone(),
                            literal: spelling,
                            value,
                            origins: vec![origin],
                        });
                    }
                } else {
                    let info = self
                        .types
                        .get(id)
                        .and_then(|i| i.imported.as_ref())
                        .ok_or_else(|| self.at(element, "Missing imported enum"))?;
                    if self.imports[info.0]
                        .enumerations
                        .get(&info.1)
                        .is_none_or(|e| e.by_name(&name.value).is_none())
                    {
                        return Err(self.at(name, "Unknown imported enum literal"));
                    }
                }
            }
            _ => return Err(self.at(element, "Unsupported enum inference element")),
        }
        Ok(())
    }
    fn add_parent<T>(
        &mut self,
        child: &TypeId,
        parent: &TypeId,
        element: &Located<T>,
    ) -> Result<()> {
        if child == parent {
            return Ok(());
        }
        if !self.class(child) || !self.class(parent) {
            return Err(self.at(element, "Model hierarchy requires class types"));
        }
        if let Some(record) = self.record_mut(child) {
            if !record.parents.contains(parent) {
                record.parents.push(parent.clone());
            }
            return Ok(());
        }
        if let Some((index, key)) = self.types.get(child).and_then(|i| i.imported.as_ref())
            && let Some(parent_key) = self.imports[*index].declarations.iter().find_map(|(k, d)| {
                (d.namespace == parent.namespace && d.name == parent.name).then_some(k)
            })
            && self.imports[*index].conforms(key, parent_key)
        {
            return Ok(());
        }
        Err(self.at(element, "Cannot add a supertype to an imported class"))
    }
    fn check_unordered_members(&self, element: &Element) -> Result<()> {
        let mut pending = vec![(element, false)];
        while let Some((member, assigned)) = pending.pop() {
            match &member.value {
                ElementKind::Action { .. } => {
                    return Err(self.at(member, "Actions cannot occur inside unordered groups"));
                }
                ElementKind::Call { rule, .. } if !assigned => {
                    if self
                        .binding(&rule.value)
                        .is_some_and(|binding| binding.kind == RuleKind::Fragment)
                    {
                        continue;
                    }
                    let ty = self.value_rule_type(rule)?;
                    if self.class(&ty) {
                        return Err(self.at(
                            member,
                            "Unassigned object calls cannot occur inside unordered groups",
                        ));
                    }
                }
                ElementKind::Assignment { value, .. } => pending.push((value, true)),
                ElementKind::Group(value)
                | ElementKind::Cardinality { element: value, .. }
                | ElementKind::Predicate { element: value, .. } => pending.push((value, assigned)),
                ElementKind::Unordered(items)
                | ElementKind::Sequence(items)
                | ElementKind::Alternatives(items)
                | ElementKind::Guard {
                    elements: items, ..
                } => pending.extend(items.iter().map(|item| (item, assigned))),
                _ => {}
            }
        }
        Ok(())
    }
    fn hierarchy(
        &mut self,
        element: &Element,
        current: BTreeSet<TypeId>,
        root: &TypeId,
    ) -> Result<BTreeSet<TypeId>> {
        match &element.value {
            ElementKind::Unordered(items) if self.unordered_groups => {
                self.check_unordered_members(element)?;
                let mut result = current;
                for member in items {
                    result = self.hierarchy(member, result, root)?;
                }
                Ok(result)
            }
            ElementKind::Sequence(items)
            | ElementKind::Guard {
                elements: items, ..
            } => {
                let mut result = current;
                for e in items {
                    result = self.hierarchy(e, result, root)?;
                }
                Ok(result)
            }
            ElementKind::Alternatives(items) => {
                let mut result = BTreeSet::new();
                for e in items {
                    result.extend(self.hierarchy(e, current.clone(), root)?);
                }
                Ok(result)
            }
            ElementKind::Group(e) | ElementKind::Predicate { element: e, .. } => {
                self.hierarchy(e, current, root)
            }
            ElementKind::Cardinality {
                element: e,
                operator,
            } => {
                let next = self.hierarchy(e, current.clone(), root)?;
                Ok(if matches!(operator, '?' | '*') {
                    current
                } else {
                    next
                })
            }
            ElementKind::Call { rule, .. } => {
                if let Some(binding) = self
                    .binding(&rule.value)
                    .filter(|b| b.kind == RuleKind::Fragment)
                {
                    return self.fragment_hierarchy(rule, &binding, current, root);
                }
                let id = self.value_rule_type(rule)?;
                if self.class(&id) {
                    self.add_parent(&id, root, element)?;
                    Ok(BTreeSet::from([id]))
                } else {
                    Ok(current)
                }
            }
            ElementKind::Action { ty, .. } => {
                let id = self.resolve(ty, None, ty)?;
                self.add_parent(&id, root, element)?;
                Ok(BTreeSet::from([id]))
            }
            ElementKind::Assignment { .. } | ElementKind::Keyword(_) => Ok(current),
            _ => Err(self.at(element, "Unsupported model inference flow element")),
        }
    }
}
/// Infer native definitions from exact sources. Executable grammar compilation
/// is a separate consumer; unsupported inference returns no successful report.
pub fn infer_metamodels(
    bundle: &DefinitionBundle,
    source: &SourceId,
    limits: SourceLimits,
) -> Result<InferredMetamodels> {
    sources::infer(bundle, source, limits)
}
/// Infer the same exact source world with explicitly installed native conversions.
/// Providers remain caller-owned; no generated or serialized output installs code.
pub fn infer_metamodels_with_conversions(
    bundle: &DefinitionBundle,
    source: &SourceId,
    limits: SourceLimits,
    conversions: &intersect_model::definitions::conversion::ConversionRegistry<'_>,
) -> Result<InferredMetamodels> {
    sources::infer_with_conversions(bundle, source, limits, Some(conversions))
}
// Immutable private source-validation selection. Public legacy/fixed helpers
// retain their exact defaults; no wire context or runtime policy setter.
#[derive(Clone, Copy, Default)]
struct SourceValidation {
    contextual_first_rule: bool,
    scoped_terminal_returns: bool,
    scoped_hidden_tokens: bool,
    durable_documents: bool,
    imported_assignment_compatibility: bool,
    unordered_groups: bool,
}
fn infer_local(
    bundle: &DefinitionBundle,
    source: &SourceId,
    limits: SourceLimits,
    inherited: Option<&PreparedInference>,
    conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    validation: SourceValidation,
) -> Result<PreparedInference> {
    let SourceValidation {
        contextual_first_rule,
        scoped_terminal_returns,
        scoped_hidden_tokens,
        durable_documents,
        imported_assignment_compatibility,
        unordered_groups,
    } = validation;
    bundle
        .validate()
        .map_err(|e| Diagnostic::new(&source.uri, 0, 1, e).with_context(source))?;
    let original = bundle
        .source(source)
        .map_err(|e| Diagnostic::new(&source.uri, 0, 1, e).with_context(source))?;
    for input in &bundle.sources {
        if input.kind == SourceKind::Ecore {
            return Err(Diagnostic::at_source(
                &input.id,
                0,
                input.text.len(),
                "Migrate Ecore explicitly before native inference",
            ));
        }
        if input.text.len() > limits.source_bytes {
            return Err(Diagnostic::at_source(
                &input.id,
                0,
                input.text.len(),
                "Inference source size budget exceeded",
            ));
        }
    }
    let syntax = syntax::parse(original, limits)?;
    let mut inputs = bundle
        .sources
        .iter()
        .map(|input| {
            let mut dependencies = input.dependencies.iter().collect::<Vec<_>>();
            dependencies.sort();
            (&input.id, dependencies)
        })
        .collect::<Vec<_>>();
    inputs.sort_by(|left, right| left.0.cmp(right.0));
    let mut namespaces = bundle
        .namespaces
        .iter()
        .map(|binding| (&binding.namespace, &binding.source))
        .collect::<Vec<_>>();
    namespaces.sort();
    let manifest = serde_json::to_string(&(
        1u32,
        &original.id,
        inputs,
        namespaces,
        compiler::standard_datatype_source().id,
    ))
    .map_err(|e| Diagnostic::new(&source.uri, 0, 1, e.to_string()).with_context(source))?;
    let context = SourceId::for_text("intersect:inference-context:v1", &manifest).sha256;
    let mut engine = Engine {
        syntax: &syntax,
        source: original,
        bundle,
        limits,
        context,
        packages: vec![],
        aliases: BTreeMap::new(),
        types: BTreeMap::new(),
        rules: BTreeMap::new(),
        imports: vec![],
        skeleton: None,
        inherited,
        inherited_origins: vec![],
        fragment_scope: None,
        active_fragments: BTreeSet::new(),
        conversions,
        contextual_first_rule,
        scoped_terminal_returns,
        scoped_hidden_tokens,
        durable_documents,
        imported_assignment_compatibility,
        unordered_groups,
    };
    engine.initialize()?;
    engine.infer_types()?;
    if engine.scoped_hidden_tokens {
        engine.check_hidden_policy()?;
    }
    if contextual_first_rule {
        let used = grammar::first_rule::Summary {
            contains_non_datatype_parser: inherited.is_some_and(|parent| {
                parent
                    .grammars()
                    .iter()
                    .flat_map(|g| g.rules.values())
                    .any(|r| {
                        matches!(r.kind, RuleKind::Parser | RuleKind::Fragment)
                            && !r
                                .target
                                .as_ref()
                                .and_then(|target| engine.types.get(target))
                                .is_some_and(|ty| ty.kind != ClassifierKind::Class)
                    })
            }),
        };
        grammar::first_rule::validate(&syntax, used, |rule| {
            if rule.value.wildcard {
                return Ok(false);
            }
            let target = engine.rule_type(&rule.value.name)?;
            engine
                .types
                .get(&target)
                .map(|ty| ty.kind != ClassifierKind::Class)
                .ok_or_else(|| engine.at(rule, "Missing resolved authored parser category"))
        })?;
    }
    for rule in &syntax.rules {
        if matches!(rule.value.kind, RuleKind::Parser | RuleKind::Fragment) && !rule.value.wildcard
        {
            let root = engine.rule_type(&rule.value.name)?;
            if engine.class(&root) {
                engine.hierarchy(&rule.value.body, BTreeSet::from([root.clone()]), &root)?;
            }
        }
    }
    engine.materialize_skeleton()?;
    engine.infer_features()?;
    engine.lift_features()?;
    engine.finish()
}

impl Engine<'_> {
    fn materialize_skeleton(&mut self) -> Result<()> {
        let generated = emit::sources(self)?;
        let Some((entry, namespace)) = &generated.selection else {
            return Ok(());
        };
        let schema = compiler::compile_structure_bundle_with_limits_observed(
            &generated.bundle,
            entry,
            namespace,
            self.conversions,
            &Default::default(),
            intersect_model::definitions::ecore::ParseLimits {
                source_bytes: self.limits.source_bytes,
            },
        )
        .map_err(|e| match e {
            intersect_model::definitions::ecore::ObservedImportError::Import(error) => {
                emit::diagnostic(self, &generated.origins, &error)
            }
            intersect_model::definitions::ecore::ObservedImportError::Observation(message) => {
                self.error(message)
            }
        })?;
        self.skeleton = Some(schema);
        Ok(())
    }
    fn key<'a>(&self, schema: &'a Schema, id: &TypeId) -> Option<&'a str> {
        schema.declarations.iter().find_map(|(key, d)| {
            (d.namespace == id.namespace && d.name == id.name).then_some(key.as_str())
        })
    }
    fn common_type(&self, types: &[TypeId], element: &Element) -> Result<TypeId> {
        let first = types
            .first()
            .ok_or_else(|| self.at(element, "Assignment has no value type"))?;
        if types.iter().all(|id| id == first) {
            return Ok(first.clone());
        }
        if !types.iter().all(|id| self.class(id)) {
            return Err(self.at(
                element,
                "Assignment alternatives have incompatible scalar/model types",
            ));
        }
        if let Some(schema) = &self.skeleton {
            return self.common_type_in_schema(schema, types, element);
        }
        if !self.durable_documents {
            return Err(self.at(element, "Missing inferred hierarchy"));
        }
        // Imported-only grammars have no generated skeleton. Reuse an actual
        // checked imported schema containing every exact namespace/name instead
        // of guessing an inherited relation or inventing a generated contract.
        let mut answers = BTreeSet::new();
        for schema in &self.imports {
            if types.iter().all(|id| {
                schema
                    .declarations
                    .values()
                    .filter(|d| d.namespace == id.namespace && d.name == id.name)
                    .count()
                    == 1
            }) {
                answers.insert(self.common_type_in_schema(schema, types, element)?);
            }
        }
        match answers.len() {
            1 => answers
                .into_iter()
                .next()
                .ok_or_else(|| self.at(element, "Missing checked common model type")),
            0 => Err(self.at(
                element,
                "Missing exact imported hierarchy for assignment alternatives",
            )),
            _ => Err(self.at(element, "Conflicting checked imported common model types")),
        }
    }
    fn common_type_in_schema(
        &self,
        schema: &Schema,
        types: &[TypeId],
        element: &Element,
    ) -> Result<TypeId> {
        let candidates = self
            .types
            .iter()
            .filter(|(_, info)| info.kind == ClassifierKind::Class)
            .filter_map(|(id, _)| {
                let key = self.key(schema, id)?;
                types
                    .iter()
                    .all(|ty| {
                        self.key(schema, ty)
                            .is_some_and(|actual| schema.conforms(actual, key))
                    })
                    .then_some(id.clone())
            })
            .collect::<Vec<_>>();
        let minimal = candidates
            .iter()
            .filter(|id| {
                !candidates.iter().any(|other| {
                    other != *id
                        && self
                            .key(schema, other)
                            .zip(self.key(schema, id))
                            .is_some_and(|(a, b)| schema.conforms(a, b))
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        match minimal.as_slice() {
            [id] => Ok(id.clone()),
            [] => Ok(TypeId::builtin("EObject")),
            _ => Err(self.at(element, "Ambiguous common model type")),
        }
    }
    fn value_type(&mut self, element: &Element) -> Result<(TypeId, bool, bool)> {
        match &element.value {
            ElementKind::Keyword(value) => {
                value.to_scalar_string().map_err(|e| self.at(element, e))?;
                Ok((TypeId::builtin("EString"), false, false))
            }
            ElementKind::Call { rule, .. } => {
                let id = self.value_rule_type(rule)?;
                let class = self.class(&id);
                Ok((id, class, class))
            }
            ElementKind::CrossReference { ty, .. } => {
                let id = self.resolve(ty, None, ty)?;
                if !self.class(&id) {
                    return Err(self.at(ty, "Cross-reference requires a class type"));
                }
                Ok((id, true, false))
            }
            ElementKind::Group(e)
            | ElementKind::Cardinality { element: e, .. }
            | ElementKind::Predicate { element: e, .. } => self.value_type(e),
            ElementKind::Alternatives(items) => {
                let mut types = vec![];
                let mut category = None;
                for item in items {
                    let (ty, reference, containment) = self.value_type(item)?;
                    if category.is_some_and(|c| c != (reference, containment)) {
                        return Err(
                            self.at(item, "Mixed attribute/reference/containment alternatives")
                        );
                    }
                    category = Some((reference, containment));
                    types.push(ty);
                }
                let (reference, containment) =
                    category.ok_or_else(|| self.at(element, "Empty assigned alternatives"))?;
                Ok((self.common_type(&types, element)?, reference, containment))
            }
            ElementKind::Sequence(items) if items.len() == 1 => self.value_type(&items[0]),
            _ => Err(self.at(element, "Unsupported assigned value inference element")),
        }
    }
    fn imported_feature(&self, owner: &TypeId, name: &str) -> Result<Option<Feature>> {
        let Some((index, key)) = self.types.get(owner).and_then(|i| i.imported.as_ref()) else {
            return Ok(None);
        };
        self.schema_feature(&self.imports[*index], key, name)
    }
    fn schema_feature(&self, schema: &Schema, key: &str, name: &str) -> Result<Option<Feature>> {
        let fields = schema.features(key).map_err(|e| self.error(e))?;
        let Some(field) = fields.get(name) else {
            return Ok(None);
        };
        let target = if let Some(datatype) = &field.datatype {
            schema
                .declarations
                .values()
                .find(|d| d.source == datatype.source && d.path == datatype.path)
                .map(|d| TypeId {
                    namespace: d.namespace.clone(),
                    name: d.name.clone(),
                })
        } else {
            None
        };
        let target = if target.is_none() && self.durable_documents {
            if let Some(declaration) = &field.datatype {
                let selected = schema
                    .datatypes
                    .get(&field.target)
                    .filter(|datatype| datatype.declaration() == declaration)
                    .ok_or_else(|| {
                        self.error(
                            "Feature datatype storage key does not select its exact declaration",
                        )
                    })?;
                let id = source_datatype_identity(
                    self.bundle,
                    declaration,
                    selected.name(),
                    self.limits,
                )
                .map_err(|message| self.error(message))?;
                if !self
                    .types
                    .get(&id)
                    .is_some_and(|info| info.kind == ClassifierKind::Datatype)
                {
                    return Err(self.error(
                        "Feature datatype declaration is outside the checked grammar type scope",
                    ));
                }
                Some(id)
            } else {
                target
            }
        } else {
            target
        };
        let target = target
            .or_else(|| {
                schema.declarations.get(&field.target).map(|d| TypeId {
                    namespace: d.namespace.clone(),
                    name: d.name.clone(),
                })
            })
            .unwrap_or_else(|| {
                TypeId::builtin(
                    if field.target == intersect_model::definitions::schema::EOBJECT_REFERENCE_TOP {
                        "EObject"
                    } else {
                        &field.target
                    },
                )
            });
        Ok(Some(Feature {
            name: field.name.clone(),
            target,
            reference: field.reference,
            containment: field.containment,
            many: field.upper != 1,
            unique: field.unique,
            origins: vec![],
        }))
    }
    // Imported features retain their authored uniqueness. The grammar's +=
    // creation default is not an additional constraint on that sealed feature.
    fn imported_assignment_shape(&self, expected: &Feature, actual: &Feature) -> bool {
        if expected.same_shape(actual)
            || (self.imported_assignment_compatibility
                && expected.name == actual.name
                && expected.target == actual.target
                && expected.reference == actual.reference
                && expected.containment == actual.containment
                && expected.many == actual.many)
        {
            return true;
        }
        if !self.scoped_hidden_tokens
            || expected.name != actual.name
            || expected.reference != actual.reference
            || expected.containment != actual.containment
            || expected.many != actual.many
            || (!self.imported_assignment_compatibility && expected.unique != actual.unique)
            || !self.class(&expected.target)
            || !self.class(&actual.target)
        {
            return false;
        }
        // Ordinary imported-class assignments are covariant. Reuse the exact
        // selected schema hierarchy; scalar/nominal identities stay invariant.
        self.imports
            .iter()
            .chain(self.skeleton.iter())
            .any(|schema| {
                match (
                    self.key(schema, &actual.target),
                    self.key(schema, &expected.target),
                ) {
                    (Some(actual), Some(expected)) => schema.conforms(actual, expected),
                    _ => false,
                }
            })
    }
    fn imported_assignment_is_writable(&self, owner: &TypeId, name: &str) -> Result<bool> {
        let Some((index, key)) = self
            .types
            .get(owner)
            .and_then(|info| info.imported.as_ref())
        else {
            return Ok(false);
        };
        let schema = &self.imports[*index];
        let selected = schema
            .effective_feature(key, name)
            .map_err(|error| self.error(error))?;
        Ok(selected.feature.changeable
            && !schema
                .is_container_feature(selected.feature)
                .map_err(|error| self.error(error))?)
    }
    fn add_feature(&mut self, owner: &TypeId, feature: Feature, element: &Element) -> Result<()> {
        if self.record(owner).is_none() {
            let existing = self
                .imported_feature(owner, &feature.name)?
                .ok_or_else(|| self.at(element, "Cannot add a feature to an imported class"))?;
            if (self.imported_assignment_compatibility
                && !self.imported_assignment_is_writable(owner, &feature.name)?)
                || !self.imported_assignment_shape(&existing, &feature)
            {
                return Err(self.at(
                    element,
                    "Grammar assignment conflicts with the imported feature",
                ));
            }
            return Ok(());
        }
        if let Some(schema) = &self.skeleton
            && let Some(key) = self.key(schema, owner)
            && let Ok(inherited) = schema.effective_feature(key, &feature.name)
        {
            if !self
                .schema_feature(schema, key, &feature.name)?
                .is_some_and(|f| f.same_shape(&feature))
            {
                return Err(self.at(
                    element,
                    "Assignment conflicts with the sealed inherited feature",
                ));
            }
            if let intersect_model::definitions::schema::MemberIdentity::Source { declaration } =
                inherited.identity
            {
                for origin in &feature.origins {
                    self.inherited_origins.push(InheritedOrigin {
                        declaration: declaration.clone(),
                        origin: origin.clone(),
                    });
                }
                // The shared compiler checks the inherited contract;
                // retain the assignment's correspondence without a duplicate declaration.
                return Ok(());
            }
        }
        let source = self.source.id.uri.clone();
        let provenance = self.span(element);
        let record = self.record_mut(owner).ok_or_else(|| {
            Diagnostic::new(
                &source,
                element.start,
                element.end,
                "Missing generated class",
            )
            .with_span(provenance.clone())
        })?;
        if let Some(existing) = record.features.iter_mut().find(|f| f.name == feature.name) {
            if !existing.same_shape(&feature) {
                return Err(Diagnostic::new(
                    &source,
                    element.start,
                    element.end,
                    "Conflicting inferred feature type/cardinality/containment",
                )
                .with_span(provenance.clone()));
            }
            for origin in feature.origins {
                if !existing.origins.contains(&origin) {
                    existing.origins.push(origin);
                }
            }
        } else {
            if record.features.len() >= 16_384 {
                return Err(Diagnostic::new(
                    &source,
                    element.start,
                    element.end,
                    "Inferred feature budget exceeded",
                )
                .with_span(provenance.clone()));
            }
            record.features.push(feature);
        }
        Ok(())
    }
    fn features(
        &mut self,
        element: &Element,
        current: BTreeSet<TypeId>,
        root: &TypeId,
        origin_override: Option<&Element>,
    ) -> Result<BTreeSet<TypeId>> {
        match &element.value {
            ElementKind::Unordered(items) if self.unordered_groups => {
                self.check_unordered_members(element)?;
                let mut result = current;
                for member in items {
                    result = self.features(member, result, root, None)?;
                }
                Ok(result)
            }
            ElementKind::Sequence(items)
            | ElementKind::Guard {
                elements: items, ..
            } => {
                let mut result = current;
                for e in items {
                    result = self.features(e, result, root, None)?;
                }
                Ok(result)
            }
            ElementKind::Alternatives(items) => {
                let mut result = BTreeSet::new();
                for e in items {
                    result.extend(self.features(e, current.clone(), root, None)?);
                }
                Ok(result)
            }
            ElementKind::Group(e) | ElementKind::Predicate { element: e, .. } => {
                self.features(e, current, root, None)
            }
            ElementKind::Cardinality {
                element: e,
                operator,
            } => {
                let location = matches!(e.value, ElementKind::Assignment { .. }).then_some(element);
                let next = self.features(e, current.clone(), root, location)?;
                Ok(if matches!(operator, '?' | '*') {
                    current
                } else {
                    next
                })
            }
            ElementKind::Assignment {
                feature,
                operator,
                value,
            } => {
                let (target, reference, containment) = if operator == "?=" {
                    (TypeId::builtin("EBoolean"), false, false)
                } else {
                    self.value_type(value)?
                };
                let origin = self.origin(origin_override.unwrap_or(element), "Assignment");
                for owner in &current {
                    self.add_feature(
                        owner,
                        Feature {
                            name: feature.value.clone(),
                            target: target.clone(),
                            reference,
                            containment,
                            many: operator == "+=",
                            // Pinned EClassifierInfo.createStructuralFeature:
                            // only containment references remain unique when many.
                            unique: operator != "+=" || (reference && containment),
                            origins: vec![origin.clone()],
                        },
                        element,
                    )?;
                }
                Ok(current)
            }
            ElementKind::Action {
                ty,
                feature,
                operator,
            } => {
                let id = self.resolve(ty, None, ty)?;
                if let Some(feature) = feature {
                    let origin = self.origin(element, "Action");
                    self.add_feature(
                        &id,
                        Feature {
                            name: feature.value.clone(),
                            target: root.clone(),
                            reference: true,
                            containment: true,
                            many: operator.as_deref() == Some("+="),
                            unique: true,
                            origins: vec![origin],
                        },
                        element,
                    )?;
                }
                Ok(BTreeSet::from([id]))
            }
            ElementKind::Call { rule, .. } => {
                if let Some(binding) = self
                    .binding(&rule.value)
                    .filter(|b| b.kind == RuleKind::Fragment)
                {
                    return if binding.wildcard {
                        self.fragment_features(rule, &binding, current, root)
                    } else {
                        Ok(current)
                    };
                }
                let id = self.value_rule_type(rule)?;
                Ok(if self.class(&id) {
                    BTreeSet::from([id])
                } else {
                    current
                })
            }
            ElementKind::Keyword(_) => Ok(current),
            _ => Err(self.at(element, "Unsupported model feature inference element")),
        }
    }
    fn infer_features(&mut self) -> Result<()> {
        let syntax = self.syntax;
        for rule in &syntax.rules {
            if matches!(rule.value.kind, RuleKind::Parser | RuleKind::Fragment)
                && !rule.value.wildcard
            {
                let root = self.rule_type(&rule.value.name)?;
                if self.class(&root) {
                    self.features(
                        &rule.value.body,
                        BTreeSet::from([root.clone()]),
                        &root,
                        None,
                    )?;
                }
            }
        }
        Ok(())
    }
    fn effective_feature(
        &self,
        owner: &TypeId,
        name: &str,
        active: &mut BTreeSet<TypeId>,
    ) -> Result<Option<Feature>> {
        if !active.insert(owner.clone()) {
            return Err(self.error("Cyclic inferred feature hierarchy"));
        }
        let result = if let Some(record) = self.record(owner) {
            if let Some(feature) = record.features.iter().find(|f| f.name == name) {
                Some(feature.clone())
            } else {
                let mut found = None;
                for parent in &record.parents {
                    if let Some(feature) = self.effective_feature(parent, name, active)? {
                        if let Some(prior) = &found {
                            let prior: &Feature = prior;
                            if !prior.same_shape(&feature) {
                                return Err(self.error("Incompatible inherited inferred features"));
                            }
                        } else {
                            found = Some(feature);
                        }
                    }
                }
                found
            }
        } else {
            self.imported_feature(owner, name)?
        };
        active.remove(owner);
        Ok(result)
    }
    fn lift_features(&mut self) -> Result<()> {
        let classes = self
            .packages
            .iter()
            .flat_map(|p| p.classifiers.iter())
            .filter(|c| c.kind == ClassifierKind::Class)
            .map(|c| c.id.clone())
            .collect::<Vec<_>>();
        // A lift removes at least one local feature or installs a new parent
        // declaration. The finite class/feature inventory bounds this fixed point.
        let mut lifted = BTreeSet::new();
        loop {
            let mut changed = false;
            for parent in &classes {
                let children = classes
                    .iter()
                    .filter(|id| self.record(id).is_some_and(|c| c.parents.contains(parent)))
                    .cloned()
                    .collect::<Vec<_>>();
                if children.len() < 2 {
                    continue;
                }
                let names = classes
                    .iter()
                    .filter_map(|id| self.record(id))
                    .flat_map(|c| c.features.iter().map(|f| f.name.clone()))
                    .collect::<BTreeSet<_>>();
                for name in names {
                    if lifted.contains(&(parent.clone(), name.clone())) {
                        continue;
                    }
                    let mut fields = vec![];
                    for child in &children {
                        if let Some(field) =
                            self.effective_feature(child, &name, &mut BTreeSet::new())?
                        {
                            fields.push(field);
                        } else {
                            break;
                        }
                    }
                    if fields.len() != children.len()
                        || !fields.iter().all(|f| f.same_shape(&fields[0]))
                    {
                        continue;
                    }
                    let mut field = fields[0].clone();
                    for other in fields.iter().skip(1) {
                        for origin in &other.origins {
                            if !field.origins.contains(origin) {
                                field.origins.push(origin.clone());
                            }
                        }
                    }
                    let node = Located {
                        start: self.syntax.start,
                        end: self.syntax.end,
                        value: (),
                    };
                    // Preserve the source order of first declaration on a parent.
                    let element = Located {
                        start: node.start,
                        end: node.end,
                        value: ElementKind::Keyword(syntax::SyntaxString(vec![])),
                    };
                    self.add_feature(parent, field, &element)?;
                    for child in &children {
                        if let Some(record) = self.record_mut(child) {
                            record.features.retain(|f| f.name != name);
                        }
                    }
                    lifted.insert((parent.clone(), name));
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(())
    }
    fn finish(self) -> Result<PreparedInference> {
        if self.packages.iter().any(|p| p.classifiers.is_empty()) {
            return Err(self.error("Pinned Xtext generated packages may not be empty"));
        }
        let generated = emit::sources(&self)?;
        let mut effective_rules = self
            .inherited
            .map(|p| p.effective_rules().clone())
            .unwrap_or_default();
        let mut local_rules = BTreeMap::new();
        for rule in &self.syntax.rules {
            let target = if rule.value.wildcard {
                None
            } else {
                Some(self.rule_type(&rule.value.name)?)
            };
            let binding = RuleBinding {
                parameters: rule.value.parameters.clone().unwrap_or_default(),
                grammar: self.syntax.name.value.clone(),
                declaration: self.span(rule),
                kind: rule.value.kind,
                target,
                wildcard: rule.value.wildcard,
                implicit: rule.value.return_type.is_none(),
            };
            local_rules.insert(rule.value.name.value.clone(), binding.clone());
            effective_rules.insert(rule.value.name.value.clone(), binding);
        }
        let mut grammars = self
            .inherited
            .map(|p| p.grammars().to_vec())
            .unwrap_or_default();
        grammars.push(GrammarInference {
            parameterized: parameter_program::requires(self.syntax),
            source: self.source.id.clone(),
            name: self.syntax.name.value.clone(),
            metamodels: self.packages.clone(),
            generated_sources: generated.sources.clone(),
            generated_origins: generated.origins.clone(),
            rules: local_rules,
            scope: self.local_scope(),
        });
        let mut inherited_origins = self
            .inherited
            .map(|p| p.inherited_origins().to_vec())
            .unwrap_or_default();
        inherited_origins.extend(self.inherited_origins.iter().cloned());
        let Some((entry, namespace)) = generated.selection else {
            return Ok(PreparedInference::Library(LibraryInference {
                source: self.source.id.clone(),
                bundle: generated.bundle,
                grammars,
                effective_rules,
                classifiers: self
                    .types
                    .iter()
                    .map(|(id, info)| (id.clone(), info.kind))
                    .collect(),
            }));
        };
        let resources = ResourceSet::new_with_limits(
            &generated.bundle,
            intersect_model::definitions::ecore::ParseLimits {
                source_bytes: self.limits.source_bytes,
            },
        )
        .map_err(|e| self.error(e.to_string()))?;
        let mut requested = Vec::new();
        for (id, info) in &self.types {
            if info.kind != ClassifierKind::Class || id.namespace == ECORE {
                continue;
            }
            let declaration = resources
                .resolve(&entry, &id.identity())
                .map_err(|e| self.error(e))?;
            requested.push(
                intersect_model::definitions::generic_types::RuntimeType::new(
                    intersect_model::definitions::generic_types::Type::Classifier {
                        declaration,
                        arguments: vec![],
                        raw: false,
                    },
                )
                .map_err(|e| self.error(e))?,
            );
        }
        let schema = compiler::materialize_runtime_bundle_with_limits(
            &generated.bundle,
            &entry,
            &namespace,
            &requested,
            self.conversions,
            intersect_model::definitions::ecore::ParseLimits {
                source_bytes: self.limits.source_bytes,
            },
        )
        .map_err(|e| emit::diagnostic(&self, &generated.origins, &e))?;
        let diagnostic_origins = grammars
            .iter()
            .flat_map(|g| g.generated_origins.iter().cloned())
            .collect();
        Ok(PreparedInference::Model(Box::new(InferredMetamodels {
            original: self.source.clone(),
            metamodels: self.packages,
            generated_sources: generated.sources,
            origins: generated.origins,
            bundle: generated.bundle,
            entry_source: entry,
            namespace,
            schema,
            materialized_types: requested,
            grammars,
            effective_rules,
            inherited_origins,
            diagnostic_origins,
        })))
    }
}

/// Source inputs for the language-owned inference frontend. Deserializing these
/// inputs authorizes no metadata; compile always rebuilds definitions and text.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceRequest {
    pub version: u32,
    pub bundle: DefinitionBundle,
    pub grammar: SourceId,
}
/// Validated in-process language result. Adapter replay uses the original request,
/// never the serialized generated schema or grammar. The package also retains
/// those inputs for language-owned replay. Archive/Toolkit adoption is separate.
pub struct InferredLanguage {
    request: InferenceRequest,
    definitions: InferredMetamodels,
    package: crate::package::Package,
}
impl InferenceRequest {
    pub fn compile(
        &self,
        limits: SourceLimits,
    ) -> std::result::Result<InferredLanguage, Vec<Diagnostic>> {
        self.compile_with_context(limits, &[], None, None)
    }
    /// Explicit source-bound requirements and caller-owned executable conversions.
    /// A behavior selection is part of the original replay recipe, not cached IR.
    pub fn compile_with_context(
        &self,
        limits: SourceLimits,
        settings: &[intersect_model::definitions::setting::ComputedSettingContract],
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
        behavior: Option<&SourceId>,
    ) -> std::result::Result<InferredLanguage, Vec<Diagnostic>> {
        self.compile_with_selected_profile(limits, None, settings, conversions, behavior)
    }
    /// Select the general native source profile explicitly. Historical callers
    /// and recipes retain their original inference capability selection.
    pub fn compile_with_selected_profile(
        &self,
        limits: SourceLimits,
        selected: Option<grammar::Profile>,
        settings: &[intersect_model::definitions::setting::ComputedSettingContract],
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
        behavior: Option<&SourceId>,
    ) -> std::result::Result<InferredLanguage, Vec<Diagnostic>> {
        if selected.is_some_and(|profile| profile != grammar::Profile::NativeV82) {
            return Err(vec![Diagnostic::at_source(
                &self.grammar,
                0,
                1,
                "Explicit general inference requires NativeV82",
            )]);
        }
        if self.version != 1 {
            return Err(vec![
                Diagnostic::new(
                    &self.grammar.uri,
                    0,
                    1,
                    "Unsupported inference request version",
                )
                .with_context(&self.grammar),
            ]);
        }
        if let Some(id) = behavior {
            self.bundle
                .validate()
                .map_err(|e| vec![Diagnostic::at_source(id, 0, 1, e).with_context(id)])?;
            let source = self
                .bundle
                .source(id)
                .map_err(|e| vec![Diagnostic::at_source(id, 0, 1, e)])?;
            if !matches!(source.kind, SourceKind::Rcore | SourceKind::RcoreBehavior) {
                return Err(vec![Diagnostic::at_source(
                    id,
                    0,
                    source.text.len(),
                    "Selected inference behavior must be an authored Rcore source",
                )]);
            }
            let mut reached = BTreeSet::new();
            let mut pending = vec![self.grammar.clone()];
            while let Some(next) = pending.pop() {
                if reached.insert(next.clone()) {
                    pending.extend(
                        self.bundle
                            .source(&next)
                            .map_err(|e| vec![Diagnostic::at_source(&next, 0, 1, e)])?
                            .dependencies
                            .iter()
                            .cloned(),
                    );
                }
            }
            if !reached.contains(id) {
                return Err(vec![Diagnostic::at_source(
                    id,
                    0,
                    1,
                    "Selected inference behavior is not an exact grammar dependency",
                )]);
            }
        }
        let definitions = match selected {
            Some(profile) => sources::prepare_with_profile_and_conversions(
                &self.bundle,
                &self.grammar,
                limits,
                profile,
                conversions,
            )
            .and_then(PreparedInference::into_model),
            None => {
                sources::infer_with_conversions(&self.bundle, &self.grammar, limits, conversions)
            }
        }
        .map_err(|e| vec![e])?;
        let program = parameter_program::prepare(&definitions, limits).map_err(|e| vec![e])?;
        let profile = selected.unwrap_or(if program.is_some() {
            grammar::Profile::NativeV57
        } else {
            grammar::Profile::NativeV56
        });
        let mut diagnostic_origins = definitions.diagnostic_origins.clone();
        if let Some(program) = &program {
            diagnostic_origins.extend(program.origins.iter().cloned());
        }
        let bindings = definitions
            .grammars
            .iter()
            .map(|source| grammar::GeneratedRuleTypes {
                source: source.source.clone(),
                types: source
                    .rules
                    .iter()
                    .filter_map(|(name, binding)| {
                        binding.target.as_ref().map(|target| {
                            (
                                name.clone(),
                                (target.namespace.clone(), target.name.clone()),
                            )
                        })
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        let program_bundle = program
            .as_ref()
            .map_or(&definitions.bundle, |program| &program.bundle);
        let program_entry = program
            .as_ref()
            .map_or(&definitions.original.id, |program| &program.entry);
        let program_bindings = program
            .as_ref()
            .map_or(bindings.as_slice(), |program| program.bindings.as_slice());
        let empty_producers = BTreeMap::new();
        let program_producers = program
            .as_ref()
            .map_or(&empty_producers, |program| &program.producers);
        let source_program = grammar::SourceProgram {
            bindings: program_bindings,
            producers: program_producers,
        };
        let policy = crate::package::CompilationSourcePolicy {
            structure_bytes: limits.source_bytes,
            xtext_bytes: limits.source_bytes,
            xtext_declarations: limits.declarations,
        };
        // Read semantics are checked before grammar admission through the same
        // structural compiler. Computed-write requirements are then consumed by
        // the shared grammar compiler, without bypassing missing read providers.
        let compile = if behavior.is_some() {
            compiler::compile_bundle_with_imported_semantics
        } else {
            compiler::compile_bundle_with_context
        };
        let mut package = compile(
            crate::package::BundleCompilation {
                materialized_types: definitions.materialized_types.clone(),
                bundle: definitions.bundle.clone(),
                ecore: definitions.entry_source.clone(),
                namespace: definitions.namespace.clone(),
                xtext: None,
                rcore_behavior: behavior.unwrap_or(&definitions.entry_source).clone(),
            },
            profile,
            &[],
            conversions,
            policy,
            policy,
        )
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|error| emit::project_diagnostic(error, &diagnostic_origins))
                .collect::<Vec<_>>()
        })?;
        let validate = |bundle: &DefinitionBundle, entry: &SourceId, limits: SourceLimits| {
            let original = self
                .bundle
                .source(entry)
                .map_err(|e| Diagnostic::at_source(entry, 0, 1, e))?;
            if bundle
                .source(entry)
                .map_err(|e| Diagnostic::at_source(entry, 0, 1, e))?
                != original
            {
                return Err(Diagnostic::at_source(
                    entry,
                    0,
                    1,
                    "Generated program changed its original inference source",
                ));
            }
            // Validate the original source world. The generated bundle also
            // contains derived namespace bindings, which are not authored inputs.
            sources::prepare_with_profile_and_conversions(
                &self.bundle,
                entry,
                limits,
                profile,
                conversions,
            )
            .map(|_| ())
        };
        let grammar = if selected.is_some() {
            grammar::compile_source_program_with_borrowed_authored_validation(
                program_bundle,
                program_entry,
                &mut package.schema,
                profile,
                limits,
                settings,
                &grammar::BorrowedAuthoredSourceProgramAdapter {
                    program: &source_program,
                    validate_authored: &validate,
                },
            )
        } else {
            grammar::compile_source_program(
                program_bundle,
                program_entry,
                &mut package.schema,
                profile,
                limits,
                settings,
                &source_program,
            )
        };
        package.grammar = Some(
            grammar.map_err(|error| vec![emit::project_diagnostic(error, &diagnostic_origins)])?,
        );
        if let Some(program) = program {
            let grammar = package.grammar.as_mut().ok_or_else(|| {
                vec![Diagnostic::at_source(
                    &self.grammar,
                    0,
                    1,
                    "Inference compiler returned no text grammar",
                )]
            })?;
            grammar.parameterized_rules = program.parameterized_rules;
            grammar.parameterized_rules.remove(&grammar.entry);
            for source in program.generated {
                package
                    .sources
                    .insert(format!("generated:xtext:{}", source.id.uri), source.text);
            }
        }
        package
            .sources
            .insert("xtext".into(), definitions.original.text.clone());
        package.compilation = None;
        package.xtext_inference = Some(crate::package::XtextInferenceInputs {
            profile: selected,
            version: self.version,
            bundle: self.bundle.clone(),
            grammar: self.grammar.clone(),
            behavior: behavior.cloned(),
        });
        Ok(InferredLanguage {
            request: self.clone(),
            definitions,
            package,
        })
    }
}
impl InferredLanguage {
    pub fn into_package(self) -> crate::package::Package {
        self.package
    }
    pub fn package(&self) -> &crate::package::Package {
        &self.package
    }
    pub fn definitions(&self) -> &InferredMetamodels {
        &self.definitions
    }
    pub fn request(&self) -> &InferenceRequest {
        &self.request
    }
    pub fn recompile(&self, limits: SourceLimits) -> std::result::Result<Self, Vec<Diagnostic>> {
        self.recompile_with_context(limits, &[], None)
    }
    pub fn recompile_with_context(
        &self,
        limits: SourceLimits,
        settings: &[intersect_model::definitions::setting::ComputedSettingContract],
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
    ) -> std::result::Result<Self, Vec<Diagnostic>> {
        self.request.compile_with_selected_profile(
            limits,
            self.package
                .xtext_inference
                .as_ref()
                .and_then(|inputs| inputs.profile),
            settings,
            conversions,
            self.package
                .xtext_inference
                .as_ref()
                .and_then(|inputs| inputs.behavior.as_ref()),
        )
    }
}

#[cfg(test)]
mod nominal_feature_identity_controls {
    use super::*;
    use intersect_model::definitions::resources::DeclarationId;
    #[test]
    fn exact_standard_and_same_name_source_declarations() {
        let standard = compiler::standard_datatype_source();
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![],
            namespaces: vec![],
        };
        let exact = DeclarationId {
            source: standard.id.clone(),
            path: vec![39],
        };
        assert_eq!(
            source_datatype_identity(&bundle, &exact, "EJavaObject", SourceLimits::default())
                .unwrap(),
            TypeId::builtin("EJavaObject")
        );
        let source = DefinitionSource::new(
            "memory:nominal-identity/lookalike.ecore",
            SourceKind::Ecore,
            r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="other" nsURI="urn:other" nsPrefix="o"><eClassifiers xsi:type="e:EDataType" name="EJavaObject" instanceClassName="java.lang.Object"/><eClassifiers xsi:type="e:EClass" name="ClassValue"/></e:EPackage>"#,
        );
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![source.clone()],
            namespaces: vec![],
        };
        let other = DeclarationId {
            source: source.id.clone(),
            path: vec![0],
        };
        let id = source_datatype_identity(&bundle, &other, "EJavaObject", SourceLimits::default())
            .unwrap();
        assert_eq!(id.namespace, "urn:other");
        assert_ne!(id, TypeId::builtin("EJavaObject"));
        assert!(
            source_datatype_identity(
                &bundle,
                &other,
                "__standard_EJavaObject",
                SourceLimits::default()
            )
            .is_err()
        );
        let class = DeclarationId {
            source: source.id.clone(),
            path: vec![1],
        };
        assert!(
            source_datatype_identity(&bundle, &class, "ClassValue", SourceLimits::default())
                .is_err()
        );
        let mut stale = other;
        stale.source.sha256 = "0".repeat(64);
        assert!(
            source_datatype_identity(&bundle, &stale, "EJavaObject", SourceLimits::default())
                .is_err()
        );
    }
    #[test]
    fn declaration_identity_resolution_keeps_exact_source_limit() {
        let source = compiler::standard_datatype_source();
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![],
            namespaces: vec![],
        };
        let id = DeclarationId {
            source: source.id.clone(),
            path: vec![39],
        };
        assert!(
            source_datatype_identity(
                &bundle,
                &id,
                "EJavaObject",
                SourceLimits {
                    source_bytes: source.text.len(),
                    ..SourceLimits::default()
                }
            )
            .is_ok()
        );
        assert!(
            source_datatype_identity(
                &bundle,
                &id,
                "EJavaObject",
                SourceLimits {
                    source_bytes: source.text.len() - 1,
                    ..SourceLimits::default()
                }
            )
            .is_err()
        );
    }
}
