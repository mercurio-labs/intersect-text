//! Handwritten durable document state over the shared interpreter. Serialized
//! nodes and recovered objects are observations, never executable authority.
use super::*;
use intersect_model::{
    core::text_document::{
        DocumentBuilder, DocumentLimits, DocumentMark, DocumentResourceAssociation, DocumentUsage,
        NodeData, NodeId, TextDocument, TextRange,
    },
    definitions::bundle::{SourceId, SourceSpan},
};
use std::sync::Arc;
#[path = "document_values.rs"]
mod values;
pub use values::ParsedValues;
#[path = "document_association_index.rs"]
mod association_index;

// Portable V65 document logical bytes are fixed per record, independent of
// target pointer widths. Payload units, operation order and admission precedence
// are unchanged. These guards bound inline record headers; allocator capacity
// and uncharged compiler/transient storage remain outside this accounting claim.
mod logical_debit {
    use super::*;
    macro_rules! header {
        ($name:ident, $ty:ty, $bytes:expr) => {
            pub(super) const $name: usize = $bytes;
            const _: () = assert!(std::mem::size_of::<$ty>() <= $name);
        };
    }
    header!(ORIGIN, GrammarOrigin, 224);
    header!(ORIGIN_INDEX, (String, usize), 32);
    header!(FRAME, Frame, 24);
    header!(FEATURE_UNDO, (usize, NodeId), 16);
    header!(FRAME_UNDO, (usize, Frame), 32);
    header!(STRING, String, 24);
    header!(FEATURE, FeatureNode, 160);
    header!(DEPENDENCY, PredictionDependency, 80);
    header!(LEXICAL_ISSUE, grammar::LexicalIssue, 64);
    header!(TOKEN_RANGE, grammar::DocumentTokenRange, 40);
    header!(SYNTAX_ISSUE, SyntaxIssue, 136);
    header!(UNIT_MESSAGE, (String, Vec<u16>), 48);
    header!(PARSED_OBJECT, ParsedObject, 136);
    header!(PARSED_REFERENCE, ParsedReference, 184);
    header!(ENTRY_VALUE, entry::EntryValue, 32);
    header!(WORD, usize, 8);
    header!(OBJECT_IDENTITY, SemanticObjectIdentity, 192);
    header!(SNAPSHOT, DocumentSnapshot, 424);
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginKind {
    ParserRule,
    /// Native scaffolding generated from the indicated authored declaration.
    GeneratedParserRule,
    /// Native transition generated from the indicated authored expression.
    GeneratedRuleCall,
    TerminalRule,
    Keyword,
    RuleCall,
    CrossReference,
    Action,
}
#[derive(Clone, Debug, Serialize)]
pub struct GrammarOrigin {
    pub kind: OriginKind,
    pub declaration: Option<SourceSpan>,
    pub expression: Option<SourceSpan>,
    pub target: Option<SourceSpan>,
    pub builtin: Option<String>,
}
#[derive(Debug, Serialize)]
pub(crate) struct Origins {
    entries: Vec<GrammarOrigin>,
    #[serde(skip)]
    by_key: BTreeMap<String, usize>,
}
#[derive(Clone, Copy, Debug)]
pub struct AnalysisLimits {
    pub resources: usize,
    pub total_source_bytes: usize,
    pub tree: DocumentLimits,
    /// Logical language association records/payloads; not allocator capacity.
    pub association_bytes: usize,
    pub association_records: usize,
    pub association_operations: usize,
}

/// Explicit per-invocation native language services. Executable providers are
/// borrowed capabilities; document/package JSON cannot install them. The same
/// forward linker and conversion implementations serve parsing and generation.
#[derive(Default)]
pub struct DocumentServices<'a, 'providers> {
    pub entry: Option<&'a str>,
    pub conversions:
        Option<&'a intersect_model::definitions::conversion::ConversionRegistry<'providers>>,
    pub settings: Option<&'a setting::TextSettings<'providers>>,
    pub derived: Option<&'a dyn query::DerivedValues>,
    pub linking: Option<&'a mut linking::LinkingContext<'providers>>,
    pub formatter: Option<&'a dyn crate::serialization::TextFormatter>,
    pub unassigned: Option<&'a dyn crate::serialization::UnassignedText>,
    pub transient: Option<&'a dyn crate::serialization::TransientValues>,
}
impl AnalysisLimits {
    pub const fn workbench() -> Self {
        Self {
            resources: 256,
            total_source_bytes: 4_000_000,
            tree: DocumentLimits {
                source_bytes: 256_000,
                nodes: 100_000,
                logical_storage_bytes: 32_000_000,
                operations: 2_000_000,
            },
            association_bytes: 16_000_000,
            association_records: 100_000,
            association_operations: 2_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize)]
struct AssociationUsage {
    bytes: usize,
    records: usize,
    operations: usize,
}
impl AssociationUsage {
    fn work(&mut self, limits: AnalysisLimits, units: usize) -> Result<(), String> {
        let operations = self
            .operations
            .checked_add(units)
            .ok_or("Document association operation overflow")?;
        if operations > limits.association_operations {
            return Err(format!(
                "Document association operation budget exceeded; requested_operations={units}; prospective_operations={operations}"
            ));
        }
        self.operations = operations;
        Ok(())
    }
    fn admit(
        &mut self,
        limits: AnalysisLimits,
        bytes: usize,
        records: usize,
    ) -> Result<(), String> {
        let operations = self
            .operations
            .checked_add(1)
            .ok_or("Document association operation overflow")?;
        if operations > limits.association_operations {
            return Err("Document association operation budget exceeded".into());
        }
        let bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or("Document association storage overflow")?;
        let records = self
            .records
            .checked_add(records)
            .ok_or("Document association record overflow")?;
        if bytes > limits.association_bytes || records > limits.association_records {
            return Err(format!(
                "Document association budget exceeded; axis={}; requested_bytes={}; requested_records={}; prospective_bytes={bytes}; prospective_records={records}",
                if bytes > limits.association_bytes {
                    "bytes"
                } else {
                    "records"
                },
                bytes - self.bytes,
                records - self.records
            ));
        }
        *self = Self {
            bytes,
            records,
            operations,
        };
        Ok(())
    }
}
fn kind(expr: &Expr) -> Option<OriginKind> {
    match expr.kind {
        Kind::Literal(_) => Some(OriginKind::Keyword),
        Kind::Call(ref name) if crate::grammar::is_generated_unordered_rule(name) => {
            Some(OriginKind::GeneratedRuleCall)
        }
        Kind::Call(_) => Some(OriginKind::RuleCall),
        Kind::Reference(_) | Kind::ScalarReference { .. } => Some(OriginKind::CrossReference),
        Kind::Action(_) | Kind::Capture { .. } => Some(OriginKind::Action),
        _ => None,
    }
}
fn original_source<'a>(p: &'a Package, id: &SourceId) -> Result<&'a str, String> {
    if let Some(s) = p
        .compilation
        .as_ref()
        .and_then(|c| c.bundle.sources.iter().find(|s| s.id == *id))
    {
        return Ok(&s.text);
    }
    if let Some(s) = p
        .xtext_inference
        .as_ref()
        .and_then(|c| c.bundle.sources.iter().find(|s| s.id == *id))
    {
        return Ok(&s.text);
    }
    if let Some(s) = p
        .sources
        .get(&id.uri)
        .filter(|s| SourceId::for_text(&id.uri, s) == *id)
    {
        return Ok(s);
    }
    // The ordinary parameter frontend retains exact generated definitions under
    // an explicit generated key. This is the freshly rebuilt package, and the
    // original generated SourceId remains visible; no authored origin is guessed.
    if let Some(s) = p
        .sources
        .get(&format!("generated:xtext:{}", id.uri))
        .filter(|text| SourceId::for_text(&id.uri, text) == *id)
    {
        return Ok(s);
    }
    Err("Missing exact grammar source for document origin".into())
}
impl Origins {
    fn prepare(p: &Package, limits: AnalysisLimits) -> Result<Self, String> {
        let g = p
            .grammar
            .as_ref()
            .ok_or("Document language needs a text grammar")?;
        Self::prepare_parts(g, |id| original_source(p, id), limits)
    }
    pub(crate) fn prepare_parts<'s>(
        g: &grammar::Grammar,
        mut source: impl FnMut(&SourceId) -> Result<&'s str, String>,
        limits: AnalysisLimits,
    ) -> Result<Self, String> {
        let mut out = Self {
            entries: Vec::new(),
            by_key: BTreeMap::new(),
        };
        let mut budget = AssociationUsage::default();
        let mut add = |key: String, origin: GrammarOrigin| -> Result<(), String> {
            let payload = origin
                .declaration
                .iter()
                .chain(origin.expression.iter())
                .chain(origin.target.iter())
                .map(|s| s.source.uri.len() + s.source.sha256.len())
                .sum::<usize>()
                + origin.builtin.as_ref().map_or(0, String::len);
            budget.admit(
                limits,
                logical_debit::ORIGIN + logical_debit::ORIGIN_INDEX + key.len() + payload,
                1,
            )?;
            if out.by_key.insert(key, out.entries.len()).is_some() {
                return Err("Duplicate document origin occurrence".into());
            }
            out.entries.push(origin);
            Ok(())
        };
        for (name, rule) in &g.rules {
            let declaration = g
                .declaration_sources
                .get(name)
                .cloned()
                .ok_or("Missing exact parser declaration origin")?;
            let text = source(&declaration.source)?;
            if declaration.start > declaration.end || declaration.end > text.len() {
                return Err("Parser declaration escapes exact source".into());
            }
            add(
                format!("rule/{name}/declaration"),
                GrammarOrigin {
                    kind: if crate::grammar::is_generated_unordered_rule(name) {
                        OriginKind::GeneratedParserRule
                    } else {
                        OriginKind::ParserRule
                    },
                    declaration: Some(declaration.clone()),
                    expression: None,
                    target: None,
                    builtin: None,
                },
            )?;
            let mut pending = vec![(&rule.body, 0usize)];
            while let Some((expr, depth)) = pending.pop() {
                if depth > 64 {
                    return Err("Document origin depth exceeded".into());
                }
                if let Some(kind) = kind(expr) {
                    let range = expr
                        .original_range
                        .as_ref()
                        .ok_or("Missing same-parser expression origin")?;
                    if range.start < declaration.start
                        || range.end > declaration.end
                        || text.get(range.clone()).is_none()
                    {
                        return Err("Expression origin escapes exact declaration".into());
                    }
                    let path = expr
                        .prediction_path
                        .as_ref()
                        .ok_or("Missing exact compiled occurrence path")?;
                    let target = match &expr.kind {
                        Kind::Call(name) if crate::grammar::is_generated_unordered_rule(name) => {
                            None
                        }
                        Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => {
                            g.declaration_sources.get(name).cloned()
                        }
                        _ => None,
                    };
                    add(
                        path.clone(),
                        GrammarOrigin {
                            kind,
                            declaration: Some(declaration.clone()),
                            expression: Some(SourceSpan {
                                source: declaration.source.clone(),
                                start: range.start,
                                end: range.end,
                            }),
                            target,
                            builtin: None,
                        },
                    )?;
                }
                match &expr.kind {
                    Kind::Sequence(items) | Kind::Alternative(items) => {
                        for item in items.iter().rev() {
                            pending.push((item, depth + 1));
                        }
                    }
                    Kind::Assign { value, .. }
                    | Kind::BooleanAssign { value, .. }
                    | Kind::SyntacticPredicate(value) => pending.push((value, depth + 1)),
                    _ => {}
                }
            }
        }
        for name in g
            .terminals
            .keys()
            .map(String::as_str)
            .chain(["ID", "INT", "STRING"])
            .collect::<BTreeSet<_>>()
        {
            let key = format!("terminal/{name}");
            let declaration = g.declaration_sources.get(name).cloned();
            add(
                key,
                GrammarOrigin {
                    kind: OriginKind::TerminalRule,
                    declaration,
                    expression: None,
                    target: None,
                    builtin: (!g.declaration_sources.contains_key(name)).then(|| name.to_owned()),
                },
            )?;
        }
        Ok(out)
    }
    pub(crate) fn origin(&self, key: &str) -> Option<&GrammarOrigin> {
        self.by_key
            .get(key)
            .and_then(|index| self.entries.get(*index))
    }
    pub(crate) fn indexed_origin(&self, index: usize) -> Option<&GrammarOrigin> {
        self.entries.get(index)
    }
    fn get(&self, key: &str) -> Result<usize, String> {
        self.by_key
            .get(key)
            .copied()
            .ok_or_else(|| format!("Missing exact document grammar occurrence {key}"))
    }
}
/// Source extent actually read by the shared native syntax predictor. This is
/// not ANTLR's internal composite-node getLookAhead bookkeeping.
#[derive(Debug, Serialize)]
pub struct PredictionDependency {
    pub node: NodeId,
    pub occurrence: String,
    pub range: TextRange,
    pub reached_end: bool,
    pub hidden_policy: Vec<String>,
}
#[derive(Clone, Copy, Debug)]
pub struct InvalidationLimits {
    pub operations: usize,
    pub nodes: usize,
}
#[derive(Debug, Serialize)]
pub struct SyntaxIssue {
    pub issue_code: &'static str,
    pub issue_data: Option<Vec<String>>,
    pub message: String,
    pub message_utf16: Option<Vec<u16>>,
    pub range: TextRange,
    pub expected: Option<String>,
    pub recovery: Recovery,
}
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Recovery {
    Delete,
    Insert,
    Resynchronize,
    Trailing,
    Lexical,
    Conversion,
}
#[derive(Debug, Serialize)]
pub struct FeatureNode {
    pub owner: String,
    pub owner_object: usize,
    pub feature: String,
    pub node: NodeId,
    pub identity: intersect_model::definitions::schema::MemberIdentity,
}
#[derive(Debug)]
struct CapturedFeature {
    owner: String,
    feature: String,
    node: NodeId,
}
#[derive(Debug, Serialize)]
pub struct SemanticObjectIdentity {
    pub id: String,
    pub class: String,
    pub classifier: intersect_model::definitions::schema::ClassifierDeclaration,
    pub runtime_type: Option<intersect_model::definitions::generic_types::RuntimeType>,
}
#[derive(Clone, Copy)]
pub(crate) enum ConversionNode {
    MatchedTerminal,
    CalledRule,
}

#[derive(Clone, Copy)]
struct Frame {
    node: NodeId,
    anchor: NodeId,
    fallback: usize,
}
pub(crate) struct Capture {
    pub(crate) builder: DocumentBuilder,
    origins: Arc<Origins>,
    frames: Vec<Frame>,
    frame_undo: Vec<(usize, Frame)>,
    pub(crate) emitted: usize,
    pub(crate) issues: Vec<SyntaxIssue>,
    features: Vec<CapturedFeature>,
    dependencies: Vec<PredictionDependency>,
    feature_undo: Vec<(usize, NodeId)>,
    pub(crate) calls: Vec<String>,
    lexical_issues: Vec<grammar::LexicalIssue>,
    token_ranges: Vec<grammar::DocumentTokenRange>,
    pending_error: Option<usize>,
    unit_messages: Vec<(String, Vec<u16>)>,
    failure_message: Option<usize>,
    pub(crate) recovering: bool,
    pub(crate) recovered_match: bool,
    pub(crate) last_error_index: Option<usize>,
    limits: AnalysisLimits,
    usage: AssociationUsage,
}
pub(crate) struct Mark {
    tree: DocumentMark,
    emitted: usize,
    issues: usize,
    features: usize,
    dependencies: usize,
    feature_undo: usize,
    frames: usize,
    frame_undo: usize,
    pending_error: Option<usize>,
    recovering: bool,
    recovered_match: bool,
    last_error_index: Option<usize>,
    unit_messages: usize,
    failure_message: Option<usize>,
}
#[derive(Debug, Serialize)]
pub struct DocumentSnapshot {
    pub(crate) document: TextDocument,
    #[serde(serialize_with = "serialize_origins")]
    origins: Arc<Origins>,
    pub(crate) syntax_issues: Vec<SyntaxIssue>,
    pub(crate) lexical_issues: Vec<grammar::LexicalIssue>,
    pub(crate) feature_nodes: Vec<FeatureNode>,
    pub(crate) semantic_objects: Vec<SemanticObjectIdentity>,
    pub(crate) schema_revision: String,
    pub(crate) parse_stage: ParseStage,
    pub(crate) prediction_dependencies: Vec<PredictionDependency>,
    #[serde(skip)]
    captured_features: Vec<CapturedFeature>,
    pub(crate) tree_usage: DocumentUsage,
    association_usage: AssociationUsage,
}
fn serialize_origins<S: serde::Serializer>(
    origins: &Arc<Origins>,
    s: S,
) -> Result<S::Ok, S::Error> {
    origins.entries.serialize(s)
}
#[derive(Serialize)]
pub struct NodeDiagnostic<'a> {
    pub source: &'a SourceId,
    pub node: NodeId,
    pub utf16: TextRange,
    pub utf8: Option<TextRange>,
    pub issue: &'a SyntaxIssue,
}
impl DocumentSnapshot {
    pub fn document(&self) -> &TextDocument {
        &self.document
    }
    pub fn syntax_issues(&self) -> &[SyntaxIssue] {
        &self.syntax_issues
    }
    pub fn lexical_issues(&self) -> &[grammar::LexicalIssue] {
        &self.lexical_issues
    }
    pub fn feature_nodes(&self) -> &[FeatureNode] {
        &self.feature_nodes
    }
    pub fn semantic_objects(&self) -> &[SemanticObjectIdentity] {
        &self.semantic_objects
    }
    pub fn schema_revision(&self) -> &str {
        &self.schema_revision
    }
    pub fn parse_stage(&self) -> &ParseStage {
        &self.parse_stage
    }
    pub fn prediction_dependencies(&self) -> &[PredictionDependency] {
        &self.prediction_dependencies
    }
    pub fn tree_usage(&self) -> DocumentUsage {
        self.tree_usage
    }
    /// Resolve an owned node through the actual unchanged resource/schema identity.
    /// This never imports an observed arena or publishes a recovered object graph.
    pub fn linked_object<'m>(
        &self,
        handle: &intersect_model::core::text_document::NodeHandle,
        model: &'m Model,
        schema: &intersect_model::definitions::schema::Schema,
    ) -> Result<Option<&'m intersect_model::core::model::Element>, String> {
        self.linked_object_with_association(handle, model, schema, None)
    }
    pub fn linked_object_in_resource<'m>(
        &self,
        handle: &intersect_model::core::text_document::NodeHandle,
        model: &'m Model,
        schema: &intersect_model::definitions::schema::Schema,
        association: &DocumentResourceAssociation,
    ) -> Result<Option<&'m intersect_model::core::model::Element>, String> {
        self.linked_object_with_association(handle, model, schema, Some(association))
    }
    fn linked_object_with_association<'m>(
        &self,
        handle: &intersect_model::core::text_document::NodeHandle,
        model: &'m Model,
        schema: &intersect_model::definitions::schema::Schema,
        association: Option<&DocumentResourceAssociation>,
    ) -> Result<Option<&'m intersect_model::core::model::Element>, String> {
        self.document.node(handle)?;
        if schema.revision != self.schema_revision || model.schema_revision != self.schema_revision
        {
            return Err("Document model links require the exact schema revision".into());
        }
        let resource_uri = association.map_or(
            self.document.source().uri.as_str(),
            DocumentResourceAssociation::resource_uri,
        );
        let resource = model
            .resources
            .get(resource_uri)
            .ok_or("Document model resource is absent")?;
        match association {
            Some(row) => self.document.check_associated_resource(row, resource)?,
            None => self.document.check_resource(resource)?,
        }
        let Some(index) = self.semantic_object(handle.node)? else {
            return Ok(None);
        };
        let expected = self
            .semantic_objects
            .get(index)
            .ok_or("Unknown document semantic identity")?;
        if schema.declarations.get(&expected.class) != Some(&expected.classifier)
            || schema.instantiations.get(&expected.class) != expected.runtime_type.as_ref()
        {
            return Err("Document classifier source identity changed".into());
        }
        let mut found = None;
        for element in &model.elements {
            if element.id == expected.id {
                if found.is_some() {
                    return Err("Ambiguous document model object identity".into());
                }
                found = Some(element);
            }
        }
        let element = found.ok_or("Document model object is absent")?;
        if element.resource != resource_uri
            || element.class != expected.class
            || element.type_identity != schema.identity(&expected.class)
        {
            return Err("Document model object source/class identity changed".into());
        }
        Ok(Some(element))
    }
    pub fn syntax_diagnostics(&self) -> impl Iterator<Item = Result<NodeDiagnostic<'_>, String>> {
        self.document
            .nodes()
            .iter()
            .enumerate()
            .filter_map(|(index, node)| node.data.syntax_error.map(|issue| (index, node, issue)))
            .map(|(index, node, issue)| {
                Ok(NodeDiagnostic {
                    source: self.document.source(),
                    node: NodeId(index),
                    utf16: node.range,
                    utf8: self.document.utf8_range(node.range)?,
                    issue: self
                        .syntax_issues
                        .get(issue)
                        .ok_or("Unknown source-bound syntax diagnostic")?,
                })
            })
    }
    pub fn origins(&self) -> &[GrammarOrigin] {
        &self.origins.entries
    }
    pub fn semantic_object(&self, node: NodeId) -> Result<Option<usize>, String> {
        let mut next = Some(node);
        let mut remaining = self.document.nodes().len();
        while let Some(id) = next {
            if remaining == 0 {
                return Err("Cyclic document parent links".into());
            }
            remaining -= 1;
            let n = self
                .document
                .nodes()
                .get(id.0)
                .ok_or("Unknown document node")?;
            if n.data.semantic_object.is_some() {
                return Ok(n.data.semantic_object);
            }
            next = n.parent;
        }
        Ok(None)
    }
}
impl DocumentSnapshot {
    /// Conservative source/node invalidation. No model mutation or incremental
    /// reparse is authorized by this query; reparsing uses the same shared Machine.
    pub fn invalidated_nodes(
        &self,
        resource: &intersect_model::core::resources::ResourceRecord,
        edit: TextRange,
        limits: InvalidationLimits,
    ) -> Result<Vec<NodeId>, String> {
        self.invalidated_nodes_with_association(resource, edit, limits, None)
    }
    pub fn invalidated_nodes_in_resource(
        &self,
        resource: &intersect_model::core::resources::ResourceRecord,
        edit: TextRange,
        limits: InvalidationLimits,
        association: &DocumentResourceAssociation,
    ) -> Result<Vec<NodeId>, String> {
        self.invalidated_nodes_with_association(resource, edit, limits, Some(association))
    }
    fn invalidated_nodes_with_association(
        &self,
        resource: &intersect_model::core::resources::ResourceRecord,
        edit: TextRange,
        limits: InvalidationLimits,
        association: Option<&DocumentResourceAssociation>,
    ) -> Result<Vec<NodeId>, String> {
        match association {
            Some(row) => self.document.check_associated_resource(row, resource)?,
            None => self.document.check_resource(resource)?,
        }
        let _checked = self.document.units_in(edit)?;
        let mut remaining = limits.operations;
        let mut selected = BTreeSet::new();
        let mut visit = |at: Option<NodeId>| -> Result<(), String> {
            if remaining == 0 {
                return Err("Document invalidation operation budget exceeded".into());
            }
            remaining -= 1;
            let mut next = at;
            while let Some(node) = next {
                if remaining == 0 {
                    return Err("Document invalidation operation budget exceeded".into());
                }
                remaining -= 1;
                let row = self
                    .document
                    .nodes()
                    .get(node.0)
                    .ok_or("Invalid prediction dependency node")?;
                if !selected.contains(&node.0) {
                    if selected.len() >= limits.nodes {
                        return Err("Document invalidation node budget exceeded".into());
                    }
                    selected.insert(node.0);
                }
                next = row.parent;
            }
            Ok(())
        };
        for (index, node) in self.document.nodes().iter().enumerate() {
            visit(
                (node.total_range.start <= edit.end && edit.start <= node.total_range.end)
                    .then_some(NodeId(index)),
            )?;
        }
        for dependency in &self.prediction_dependencies {
            visit(
                (dependency.range.start <= edit.end && edit.start <= dependency.range.end)
                    .then_some(dependency.node),
            )?;
        }
        Ok(selected.into_iter().map(NodeId).collect())
    }
}
impl Capture {
    fn new(
        source: &Source,
        revision: u64,
        origins: Arc<Origins>,
        limits: AnalysisLimits,
    ) -> Result<Self, String> {
        // Source admission precedes its only owned copy and the builder's digest.
        if source.text.len() > limits.tree.source_bytes {
            return Err("Document source byte budget exceeded".into());
        }
        let builder =
            DocumentBuilder::from_source(&source.uri, &source.text, revision, limits.tree)?;
        Ok(Self::from_builder(builder, origins, limits))
    }
    fn from_associated_source(
        source: &Source,
        association: &DocumentResourceAssociation,
        revision: u64,
        origins: Arc<Origins>,
        limits: AnalysisLimits,
    ) -> Result<Self, String> {
        if source.uri != association.source().uri {
            return Err("Capture association belongs to another raw source".into());
        }
        let builder = DocumentBuilder::from_associated_source(
            association,
            &source.text,
            revision,
            limits.tree,
        )?;
        Ok(Self::from_builder(builder, origins, limits))
    }
    fn from_builder(
        builder: DocumentBuilder,
        origins: Arc<Origins>,
        limits: AnalysisLimits,
    ) -> Self {
        Self {
            builder,
            origins,
            frames: Vec::new(),
            frame_undo: Vec::new(),
            emitted: 0,
            issues: Vec::new(),
            features: Vec::new(),
            dependencies: Vec::new(),
            feature_undo: Vec::new(),
            calls: Vec::new(),
            lexical_issues: Vec::new(),
            token_ranges: Vec::new(),
            unit_messages: Vec::new(),
            failure_message: None,
            pending_error: None,
            recovering: false,
            recovered_match: false,
            last_error_index: None,
            limits,
            usage: AssociationUsage::default(),
        }
    }
    // Portable logical admission for the bounded interpreter continuation.
    // It creates no document node/source observation and refunds no prior work.
    pub(crate) fn admit_rule_continuation(&mut self, bytes: usize) -> Result<(), String> {
        self.usage.admit(self.limits, bytes, 1)
    }
    pub(crate) fn mark(&mut self) -> Result<Mark, String> {
        Ok(Mark {
            tree: self.builder.mark()?,
            emitted: self.emitted,
            issues: self.issues.len(),
            features: self.features.len(),
            dependencies: self.dependencies.len(),
            feature_undo: self.feature_undo.len(),
            frames: self.frames.len(),
            frame_undo: self.frame_undo.len(),
            pending_error: self.pending_error,
            recovering: self.recovering,
            recovered_match: self.recovered_match,
            last_error_index: self.last_error_index,
            unit_messages: self.unit_messages.len(),
            failure_message: self.failure_message,
        })
    }
    pub(crate) fn check_mark(&self, mark: &Mark) -> Result<(), String> {
        self.builder.check_mark(&mark.tree)
    }
    pub(crate) fn commit(&mut self, mark: Mark) -> Result<(), String> {
        self.builder.commit(mark.tree)
    }
    pub(crate) fn rollback(&mut self, mark: Mark) -> Result<(), String> {
        self.builder.rollback(mark.tree)?;
        self.emitted = mark.emitted;
        self.issues.truncate(mark.issues);
        self.features.truncate(mark.features);
        self.dependencies.truncate(mark.dependencies);
        while self.feature_undo.len() > mark.feature_undo {
            let (index, node) = self
                .feature_undo
                .pop()
                .ok_or("Missing document feature undo")?;
            if index < mark.features {
                self.features
                    .get_mut(index)
                    .ok_or("Missing original document feature")?
                    .node = node;
            }
        }
        while self.frame_undo.len() > mark.frame_undo {
            let (index, frame) = self.frame_undo.pop().ok_or("Missing document frame undo")?;
            if index < mark.frames {
                *self
                    .frames
                    .get_mut(index)
                    .ok_or("Missing original document frame")? = frame;
            }
        }
        self.frames.truncate(mark.frames);
        self.pending_error = mark.pending_error;
        self.recovering = mark.recovering;
        self.recovered_match = mark.recovered_match;
        self.last_error_index = mark.last_error_index;
        self.unit_messages.truncate(mark.unit_messages);
        self.failure_message = mark.failure_message;
        Ok(())
    }
    /// Actual active capture frame, used only by explicit draft rejection.
    pub(crate) fn active_origin(&self) -> Result<Option<usize>, String> {
        self.frames.last().map_or(Ok(None), |frame| {
            Ok(self.builder.node_data(frame.node)?.origin)
        })
    }
    pub(crate) fn begin(&mut self, key: &str, at: usize) -> Result<NodeId, String> {
        self.usage.admit(self.limits, logical_debit::FRAME, 1)?;
        let node = self.builder.begin(
            at,
            NodeData {
                origin: Some(self.origins.get(key)?),
                ..NodeData::default()
            },
        )?;
        self.frames.push(Frame {
            node,
            anchor: node,
            fallback: at,
        });
        Ok(node)
    }
    pub(crate) fn close(&mut self, node: NodeId) -> Result<(), String> {
        let frame = self.frames.last().ok_or("Missing document frame")?;
        if frame.anchor != node {
            return Err("Document parser frame mismatch".into());
        }
        self.builder
            .close_from_children(frame.node, frame.fallback)?;
        self.frames.pop();
        Ok(())
    }
    pub(crate) fn action(&mut self, key: &str) -> Result<NodeId, String> {
        let index = self
            .frames
            .len()
            .checked_sub(1)
            .ok_or("Missing action document frame")?;
        let before = self.frames[index];
        let mut links = 0usize;
        for feature in &self.features {
            self.usage.admit(self.limits, 0, 0)?;
            self.usage.admit(self.limits, 0, 0)?;
            if feature.node == before.node {
                links = links
                    .checked_add(1)
                    .ok_or("Document action feature count overflow")?;
            }
        }
        let bytes = links
            .checked_mul(logical_debit::FEATURE_UNDO)
            .and_then(|n| n.checked_add(logical_debit::FRAME_UNDO))
            .ok_or("Document action association storage overflow")?;
        self.usage.admit(
            self.limits,
            bytes,
            links
                .checked_add(1)
                .ok_or("Document action association record overflow")?,
        )?;
        let node = self.builder.wrap_open(
            before.node,
            NodeData {
                origin: Some(self.origins.get(key)?),
                ..NodeData::default()
            },
        )?;
        for (index, feature) in self.features.iter_mut().enumerate() {
            if feature.node == before.node {
                self.feature_undo.push((index, feature.node));
                feature.node = node;
            }
        }
        self.frame_undo.push((index, before));
        self.frames[index].node = node;
        Ok(before.node)
    }
    pub(crate) fn push_call(&mut self, path: &str) -> Result<(), String> {
        if self.calls.len() >= 128 {
            return Err("Document recovery call depth exceeded".into());
        }
        self.usage
            .admit(self.limits, logical_debit::STRING + path.len(), 1)?;
        self.calls.push(path.to_owned());
        Ok(())
    }
    pub(crate) fn associate(&mut self, object: usize) -> Result<(), String> {
        let frame = self
            .frames
            .last()
            .ok_or("Missing semantic document frame")?;
        let mut data = self.builder.node_data(frame.node)?;
        data.semantic_object = Some(object);
        data.direct_semantic_association = true;
        self.builder.set_data(frame.node, data)
    }
    pub(crate) fn feature(
        &mut self,
        owner: &str,
        feature: &str,
        node: NodeId,
    ) -> Result<(), String> {
        self.usage.admit(
            self.limits,
            logical_debit::FEATURE + owner.len() + feature.len(),
            1,
        )?;
        self.features.push(CapturedFeature {
            owner: owner.into(),
            feature: feature.into(),
            node,
        });
        Ok(())
    }
    pub(crate) fn prediction(
        &mut self,
        path: &str,
        start: usize,
        after: usize,
        hidden: &[String],
    ) -> Result<(), String> {
        if start == after {
            return Ok(());
        }
        let node = self
            .frames
            .last()
            .ok_or("Missing actual prediction document frame")?
            .node;
        let eof = self.token_ranges.last().map_or(0, |r| r.utf16_end);
        let reached_end = after > self.token_ranges.len();
        let range = TextRange {
            start: self.token_ranges.get(start).map_or(eof, |r| r.utf16_start),
            end: if reached_end {
                eof
            } else {
                self.token_ranges
                    .get(after.checked_sub(1).ok_or("Invalid prediction cursor")?)
                    .ok_or("Prediction cursor escapes retained token stream")?
                    .utf16_end
            },
        };
        let payload = hidden.iter().try_fold(path.len(), |n, value| {
            n.checked_add(logical_debit::STRING)
                .and_then(|n| n.checked_add(value.len()))
                .ok_or("Prediction dependency storage overflow")
        })?;
        self.usage.admit(
            self.limits,
            payload
                .checked_add(logical_debit::DEPENDENCY)
                .ok_or("Prediction dependency storage overflow")?,
            1,
        )?;
        self.dependencies.push(PredictionDependency {
            node,
            occurrence: path.into(),
            range,
            reached_end,
            hidden_policy: hidden.to_vec(),
        });
        Ok(())
    }
    pub(crate) fn lexical_issues(
        &mut self,
        issues: Vec<grammar::LexicalIssue>,
        ranges: Vec<grammar::DocumentTokenRange>,
    ) -> Result<(), String> {
        self.usage.admit(
            self.limits,
            issues
                .len()
                .checked_mul(logical_debit::LEXICAL_ISSUE)
                .ok_or("Document lexical size overflow")?,
            issues.len(),
        )?;
        self.usage.admit(
            self.limits,
            ranges
                .len()
                .checked_mul(logical_debit::TOKEN_RANGE)
                .ok_or("Document token range storage overflow")?,
            ranges.len(),
        )?;
        self.lexical_issues = issues;
        self.token_ranges = ranges;
        Ok(())
    }
    pub(crate) fn token_range(&self, index: usize) -> Option<TextRange> {
        self.token_ranges.get(index).map(|r| TextRange {
            start: r.utf16_start,
            end: r.utf16_end,
        })
    }
    pub(crate) fn token_positions(&self) -> Vec<usize> {
        self.token_ranges.iter().map(|r| r.utf16_start).collect()
    }
    pub(crate) fn scalar_token(&self, index: usize) -> bool {
        self.token_ranges
            .get(index)
            .is_some_and(|r| r.utf8.is_some())
    }
    pub(crate) fn lexical_leaf(
        &mut self,
        index: usize,
        range: TextRange,
    ) -> Result<Option<NodeId>, String> {
        let Some(issue) = self.lexical_issues.iter().find(|e| e.token == index) else {
            return Ok(None);
        };
        let prefix = if issue.early_exit {
            "required (...)+ loop did not match anything at character '"
        } else if issue.expected.is_some() {
            "mismatched character '"
        } else {
            "no viable alternative at character '"
        };
        let mut units: Vec<u16> = prefix.encode_utf16().collect();
        append_error_unit(&mut units, issue.unit);
        if let Some(expected) = issue.expected {
            units.extend("' expecting '".encode_utf16());
            append_error_unit(&mut units, Some(expected));
        }
        units.push(39);
        let (message, message_utf16) = unit_message(units);
        self.usage.admit(
            self.limits,
            logical_debit::SYNTAX_ISSUE
                + message.len()
                + message_utf16.as_ref().map_or(0, |v| v.len() * 2),
            1,
        )?;
        let error = self.issues.len();
        self.issues.push(SyntaxIssue {
            issue_code: "org.eclipse.xtext.diagnostics.Diagnostic.Syntax",
            issue_data: None,
            message,
            message_utf16,
            range,
            expected: None,
            recovery: Recovery::Lexical,
        });
        Ok(Some(self.builder.leaf(
            range,
            NodeData {
                origin: None,
                hidden: true,
                syntax_error: Some(error),
                ..NodeData::default()
            },
        )?))
    }
    pub(crate) fn token_message(
        &mut self,
        index: usize,
        prefix: &str,
        suffix: &str,
    ) -> Result<String, String> {
        let range = self.token_range(index);
        let source_units = range.map_or(5, |r| r.end - r.start);
        let units = prefix
            .encode_utf16()
            .count()
            .checked_add(
                source_units
                    .checked_mul(2)
                    .ok_or("Diagnostic unit count overflow")?,
            )
            .and_then(|v| v.checked_add(suffix.encode_utf16().count()))
            .ok_or("Diagnostic unit count overflow")?;
        self.usage.admit(
            self.limits,
            units
                .checked_mul(10)
                .and_then(|v| v.checked_add(logical_debit::UNIT_MESSAGE))
                .ok_or("Diagnostic unit storage overflow")?,
            1,
        )?;
        let mut message: Vec<u16> = prefix.encode_utf16().collect();
        if let Some(range) = range {
            for unit in self.builder.source_units(range)? {
                append_error_unit(&mut message, Some(unit));
            }
        } else {
            message.extend("<EOF>".encode_utf16())
        }
        message.extend(suffix.encode_utf16());
        let (display, raw) = unit_message(message);
        self.failure_message = None;
        if let Some(raw) = raw {
            self.failure_message = Some(self.unit_messages.len());
            self.unit_messages.push((display.clone(), raw));
        }
        Ok(display)
    }
    pub(crate) fn matched(&mut self) {
        if self.recovered_match {
            self.recovered_match = false
        } else {
            self.recovering = false;
        }
    }
    pub(crate) fn report(
        &mut self,
        message: String,
        range: TextRange,
        expected: Option<String>,
        recovery: Recovery,
        force: bool,
    ) -> Result<(), String> {
        if self.pending_error.is_some() || (self.recovering && !force) {
            return Ok(());
        }
        let was_recovering = self.recovering;
        self.issue(message, range, expected, recovery)?;
        // Generated rule catches invoke recover directly, not reportError. The
        // pinned recover helper attaches currentError without entering report suppression.
        self.recovering = if force { was_recovering } else { true };
        Ok(())
    }
    pub(crate) fn conversion_issue(
        &mut self,
        message: String,
        range: TextRange,
        target: ConversionNode,
    ) -> Result<(), String> {
        // Conversion follows a committed successful match. Terminal conversion
        // belongs to its leaf; datatype-rule conversion belongs to the retained
        // caller RuleCall composite. Neither path rewinds or synthesizes tokens.
        let error = self.issue(message, range, None, Recovery::Conversion)?;
        match target {
            ConversionNode::MatchedTerminal => self.builder.attach_final_error(error)?,
            ConversionNode::CalledRule => {
                let frame = self
                    .frames
                    .last()
                    .ok_or("Missing datatype conversion rule frame")?;
                let mut data = self.builder.node_data(frame.node)?;
                data.syntax_error = Some(error);
                self.builder.set_data(frame.node, data)?;
            }
        }
        self.pending_error = None;
        Ok(())
    }
    pub(crate) fn issue(
        &mut self,
        message: String,
        range: TextRange,
        expected: Option<String>,
        recovery: Recovery,
    ) -> Result<usize, String> {
        self.usage.admit(
            self.limits,
            logical_debit::SYNTAX_ISSUE + message.len() + expected.as_ref().map_or(0, String::len),
            1,
        )?;
        let id = self.issues.len();
        let message_utf16 = if let Some(index) = self.failure_message.take() {
            let (display, raw) = self
                .unit_messages
                .get(index)
                .ok_or("Unknown source-unit diagnostic message")?;
            if *display == message {
                self.usage.admit(
                    self.limits,
                    raw.len()
                        .checked_mul(2)
                        .ok_or("Diagnostic unit storage overflow")?,
                    1,
                )?;
                Some(raw.clone())
            } else {
                None
            }
        } else {
            None
        };
        self.issues.push(SyntaxIssue {
            issue_code: "org.eclipse.xtext.diagnostics.Diagnostic.Syntax",
            issue_data: None,
            message,
            message_utf16,
            range,
            expected,
            recovery,
        });
        self.pending_error = Some(id);
        Ok(id)
    }
    pub(crate) fn leaf(
        &mut self,
        key: Option<&str>,
        range: TextRange,
        hidden: bool,
    ) -> Result<NodeId, String> {
        let error = if hidden {
            None
        } else {
            self.pending_error.take()
        };
        self.builder.leaf(
            range,
            NodeData {
                origin: key.map(|key| self.origins.get(key)).transpose()?,
                hidden,
                syntax_error: error,
                ..NodeData::default()
            },
        )
    }
    pub(crate) fn finish(mut self) -> Result<DocumentSnapshot, String> {
        if !self.frames.is_empty() {
            return Err("Unclosed parser document frame".into());
        }
        if let Some(error) = self.pending_error.take() {
            self.builder.attach_final_error(error)?;
        }
        let (document, tree_usage, remapping) = self.builder.finish_with_remapping()?;
        if let Some(remapping) = remapping {
            for feature in &mut self.features {
                feature.node = remapping.node(feature.node)?;
            }
            for dependency in &mut self.dependencies {
                dependency.node = remapping.node(dependency.node)?;
            }
        }
        Ok(DocumentSnapshot {
            document,
            origins: self.origins,
            syntax_issues: self.issues,
            lexical_issues: self.lexical_issues,
            feature_nodes: Vec::new(),
            semantic_objects: Vec::new(),
            schema_revision: String::new(),
            parse_stage: ParseStage::default(),
            prediction_dependencies: self.dependencies,
            captured_features: self.features,
            tree_usage,
            association_usage: self.usage,
        })
    }
}
/// Actual owned parser-stage observations, captured before reference linking,
/// model validation and semantic execution. No Deserialize/ModelStore facade.
#[derive(Debug, Serialize, Default)]
pub struct ParseStage {
    pub objects: Vec<ParsedObject>,
    pub references: Vec<ParsedReference>,
    pub returned: Option<entry::EntryValue>,
}
#[derive(Debug, Serialize)]
pub struct ParsedObject {
    pub id: String,
    pub class: String,
    pub type_identity: String,
    pub resource: String,
    pub start: usize,
    pub end: usize,
    pub values: ParsedValues,
}
#[derive(Debug, Serialize)]
pub struct ParsedReference {
    pub owner: String,
    pub field: String,
    pub spelling: String,
    pub target: String,
    pub source: SourceId,
    pub start: usize,
    pub end: usize,
    pub index: Option<usize>,
    pub active: bool,
}
impl DocumentSnapshot {
    pub(crate) fn capture_parse_stage<S: ModelRead>(
        &mut self,
        store: &S,
        count: usize,
        pending: &[Pending],
        returned: Option<&Value>,
        datatype: bool,
        limits: AnalysisLimits,
    ) -> Result<(), String> {
        self.capture_parse_stage_inner(store, count, pending, returned, datatype, limits)
            .map_err(|error| self.association_error("capture_parse_stage", limits, error))
    }
    fn capture_parse_stage_inner<S: ModelRead>(
        &mut self,
        store: &S,
        count: usize,
        pending: &[Pending],
        returned: Option<&Value>,
        datatype: bool,
        limits: AnalysisLimits,
    ) -> Result<(), String> {
        let mut pool = values::ValuePool::new(&mut self.association_usage, limits)?;
        let uri = self.document.source().uri.as_str();
        for index in 0..count {
            self.association_usage.admit(
                limits,
                uri.len()
                    .checked_add(32)
                    .ok_or("Parse-stage identity storage overflow")?,
                0,
            )?;
            let id = format!("{uri}#{index}");
            let found = store.inspect(&id, &mut |view| {
                let header = [
                    view.id().len(),
                    view.class().len(),
                    view.type_identity().len(),
                    view.resource().len(),
                    logical_debit::PARSED_OBJECT,
                ]
                .into_iter()
                .try_fold(0usize, |n, v| {
                    n.checked_add(v)
                        .ok_or("Parse-stage object storage overflow")
                })?;
                self.association_usage.admit(limits, header, 1)?;
                let mut values = ParsedValues::new();
                for (name, value) in view.fields() {
                    values.capture(name, value, &mut pool, &mut self.association_usage, limits)?;
                }
                self.parse_stage.objects.push(ParsedObject {
                    id: view.id().into(),
                    class: view.class().into(),
                    type_identity: view.type_identity().into(),
                    resource: view.resource().into(),
                    start: view.start(),
                    end: view.end(),
                    values,
                });
                Ok(())
            })?;
            if !found {
                return Err("Parse-stage object disappeared before linking".into());
            }
        }
        for reference in pending {
            if *reference.uri != *self.document.source().uri
                || reference.start > reference.end
                || self
                    .document
                    .text()
                    .get(reference.start..reference.end)
                    .is_none()
            {
                return Err("Parse-stage reference is outside exact source".into());
            }
            let bytes = [
                reference.owner.len(),
                reference.field.len(),
                reference.spelling.len(),
                reference.target.len(),
                self.document.source().uri.len(),
                self.document.source().sha256.len(),
                logical_debit::PARSED_REFERENCE,
            ]
            .into_iter()
            .try_fold(0usize, |n, v| {
                n.checked_add(v)
                    .ok_or("Parse-stage reference storage overflow")
            })?;
            self.association_usage.admit(limits, bytes, 1)?;
            self.parse_stage.references.push(ParsedReference {
                owner: reference.owner.to_string(),
                field: reference.field.to_string(),
                spelling: reference.spelling.clone(),
                target: reference.target.to_string(),
                source: self.document.source().clone(),
                start: reference.start,
                end: reference.end,
                index: reference.index,
                active: reference.active,
            });
        }
        let payload = returned.and_then(Value::as_str).map_or(0, str::len);
        self.association_usage.admit(
            limits,
            payload
                .checked_add(logical_debit::ENTRY_VALUE)
                .ok_or("Parse entry storage overflow")?,
            1,
        )?;
        self.parse_stage.returned = match returned {
            Some(Value::String(value)) if datatype => {
                Some(entry::EntryValue::Datatype(value.clone()))
            }
            Some(Value::String(id)) => Some(entry::EntryValue::Object(id.clone())),
            Some(Value::Null) => Some(entry::EntryValue::Empty),
            _ => None,
        };
        Ok(())
    }
}
fn declaration_payload(source: &SourceId, path: &[usize]) -> Result<usize, String> {
    source
        .uri
        .len()
        .checked_add(source.sha256.len())
        .and_then(|v| v.checked_add(path.len().checked_mul(logical_debit::WORD)?))
        .ok_or_else(|| "Document declaration storage overflow".into())
}
fn member_payload(
    member: &intersect_model::definitions::schema::MemberIdentity,
) -> Result<usize, String> {
    use intersect_model::definitions::schema::MemberIdentity;
    match member {
        MemberIdentity::Source { declaration } => {
            declaration_payload(&declaration.source, &declaration.path)
        }
        MemberIdentity::Programmatic {
            revision,
            namespace,
            owner,
            name,
        } => [revision.len(), namespace.len(), owner.len(), name.len()]
            .into_iter()
            .try_fold(0usize, |n, v| {
                n.checked_add(v)
                    .ok_or_else(|| "Document member storage overflow".into())
            }),
    }
}
impl DocumentSnapshot {
    fn bind_model(
        &mut self,
        model: &Model,
        schema: &intersect_model::definitions::schema::Schema,
        limits: AnalysisLimits,
    ) -> Result<(), String> {
        self.bind_model_with_association(model, schema, limits, None)
    }
    fn bind_model_with_association(
        &mut self,
        model: &Model,
        schema: &intersect_model::definitions::schema::Schema,
        limits: AnalysisLimits,
        association: Option<&DocumentResourceAssociation>,
    ) -> Result<(), String> {
        self.bind_model_with_association_inner(model, schema, limits, association)
            .map_err(|error| self.association_error("bind_model", limits, error))
    }
    fn association_error(&self, stage: &str, limits: AnalysisLimits, error: String) -> String {
        format!(
            "{error}; actual_document_uri={:?}; actual_document_sha256={}; stage={stage}; association_usage=bytes:{},records:{},operations:{}; association_limits=bytes:{},records:{},operations:{}",
            self.document.source().uri,
            self.document.source().sha256,
            self.association_usage.bytes,
            self.association_usage.records,
            self.association_usage.operations,
            limits.association_bytes,
            limits.association_records,
            limits.association_operations
        )
    }
    fn bind_model_with_association_inner(
        &mut self,
        model: &Model,
        schema: &intersect_model::definitions::schema::Schema,
        limits: AnalysisLimits,
        association: Option<&DocumentResourceAssociation>,
    ) -> Result<(), String> {
        let resource = association.map_or(
            self.document.source().uri.as_str(),
            DocumentResourceAssociation::resource_uri,
        );
        self.association_usage
            .admit(limits, schema.revision.len(), 0)?;
        self.schema_revision = schema.revision.clone();
        let model_index = association_index::IdentityIndex::prepare(
            model
                .elements
                .iter()
                .enumerate()
                .map(|(index, element)| (element.id.as_str(), index)),
            &mut self.association_usage,
            limits,
        )?;
        for parsed in &self.parse_stage.objects {
            let index = model_index
                .lookup(
                    &parsed.id,
                    "Ambiguous actual document model identity",
                    &mut self.association_usage,
                    limits,
                )?
                .ok_or("Parsed document object disappeared during later model stages")?;
            let element = model
                .elements
                .get(index)
                .ok_or("Document model index escapes its original rows")?;
            if element.resource != resource {
                return Err("Document object moved outside its exact source resource".into());
            }
            let classifier = schema
                .declarations
                .get(&element.class)
                .ok_or("Document model class lacks a source declaration")?;
            let payload = declaration_payload(&classifier.source, &classifier.path)?
                .checked_add(classifier.namespace.len())
                .and_then(|v| v.checked_add(classifier.name.len()))
                .and_then(|v| v.checked_add(element.id.len()))
                .and_then(|v| v.checked_add(element.class.len()))
                .and_then(|v| v.checked_add(logical_debit::OBJECT_IDENTITY))
                .ok_or("Document object association storage overflow")?;
            self.association_usage.admit(limits, payload, 1)?;
            self.semantic_objects.push(SemanticObjectIdentity {
                id: element.id.clone(),
                class: element.class.clone(),
                classifier: classifier.clone(),
                runtime_type: schema.instantiations.get(&element.class).cloned(),
            });
        }
        for node in self.document.nodes() {
            if let Some(object) = node.data.semantic_object
                && object >= self.semantic_objects.len()
            {
                return Err("Document semantic locator escapes actual analyzed objects".into());
            }
        }
        let owner_index = association_index::IdentityIndex::prepare(
            self.semantic_objects
                .iter()
                .enumerate()
                .map(|(index, element)| (element.id.as_str(), index)),
            &mut self.association_usage,
            limits,
        )?;
        for feature in std::mem::take(&mut self.captured_features) {
            let owner_object = owner_index
                .lookup(
                    &feature.owner,
                    "Ambiguous document feature owner",
                    &mut self.association_usage,
                    limits,
                )?
                .ok_or("Document feature owner is not in analyzed model")?;
            let element = self
                .semantic_objects
                .get(owner_object)
                .ok_or("Document owner index escapes its original rows")?;
            let member = schema.effective_feature(&element.class, &feature.feature)?;
            self.association_usage.admit(
                limits,
                logical_debit::FEATURE
                    .checked_add(member_payload(&member.identity)?)
                    .ok_or("Document feature identity storage overflow")?,
                1,
            )?;
            self.feature_nodes.push(FeatureNode {
                owner: feature.owner,
                owner_object,
                feature: feature.feature,
                node: feature.node,
                identity: member.identity,
            });
        }
        Ok(())
    }
}
/// Source-rebuilt grammar and immutable original/source-generated origin catalog.
/// Old profiles expose none of this new document/recovery contract.
pub struct DocumentLanguage {
    entry: entry::EntryLanguage,
    origins: Arc<Origins>,
    limits: AnalysisLimits,
}
impl DocumentLanguage {
    pub(crate) fn serialization_hidden<'a>(
        &'a self,
        snapshot: &DocumentSnapshot,
        mut node: NodeId,
        work: &mut usize,
    ) -> Result<Option<&'a [String]>, String> {
        let grammar = self
            .package()
            .grammar
            .as_ref()
            .ok_or("Missing serialization grammar")?;
        loop {
            *work = work
                .checked_sub(1)
                .ok_or("Hidden-context association budget exceeded")?;
            let current = snapshot
                .document()
                .nodes()
                .get(node.0)
                .ok_or("Missing hidden-context node")?;
            if let Some(origin) = current
                .data
                .origin
                .and_then(|index| self.origins.indexed_origin(index))
                && matches!(origin.kind, OriginKind::ParserRule | OriginKind::RuleCall)
            {
                let declaration = match origin.kind {
                    OriginKind::RuleCall => origin.target.as_ref(),
                    _ => origin.declaration.as_ref(),
                };
                for (name, rule) in &grammar.rules {
                    *work = work
                        .checked_sub(1)
                        .ok_or("Hidden-context association budget exceeded")?;
                    if grammar.declaration_sources.get(name) == declaration
                        && let Some(hidden) = rule.hidden.as_deref()
                    {
                        return Ok(Some(hidden));
                    }
                }
            }
            let Some(parent) = current.parent else {
                return Ok(grammar.hidden.as_deref());
            };
            node = parent;
        }
    }
    pub(crate) fn serialization_hints(
        &self,
        snapshots: &[DocumentSnapshot],
        model: &Model,
        work: &mut usize,
    ) -> Result<crate::serialization::SerializationHints, String> {
        let grammar = self
            .package()
            .grammar
            .as_ref()
            .ok_or("Missing serialization grammar")?;
        let mut unassigned = std::collections::BTreeSet::new();
        let mut pending: Vec<_> = grammar.rules.values().map(|rule| &rule.body).collect();
        while let Some(expression) = pending.pop() {
            *work = work
                .checked_sub(1)
                .ok_or("Serialization hint work budget exceeded")?;
            match &expression.kind {
                Kind::Call(name)
                    if grammar
                        .rules
                        .get(name)
                        .is_none_or(|rule| rule.datatype.is_some()) =>
                {
                    if let Some(path) = &expression.prediction_path {
                        unassigned.insert(path.as_str());
                    }
                }
                Kind::Sequence(items) | Kind::Alternative(items) => pending.extend(items.iter()),
                Kind::SyntacticPredicate(value) => pending.push(value),
                _ => {}
            }
        }
        let paths: BTreeMap<_, _> = self
            .origins
            .by_key
            .iter()
            .map(|(path, index)| (*index, path))
            .collect();
        let mut hints = crate::serialization::SerializationHints::default();
        for snapshot in snapshots {
            for (index, node) in snapshot.document().nodes().iter().enumerate() {
                *work = work
                    .checked_sub(1)
                    .ok_or("Serialization hint work budget exceeded")?;
                if node.data.hidden {
                    continue;
                }
                let Some(path) = node.data.origin.and_then(|i| paths.get(&i)) else {
                    continue;
                };
                let owner = snapshot
                    .serialization_owner(model, &self.package().schema, NodeId(index), work)?
                    .map_or_else(
                        || {
                            crate::serialization::HintOwner::Resource(
                                snapshot.document().source().uri.clone(),
                            )
                        },
                        crate::serialization::HintOwner::Object,
                    );
                let mut keys = vec![(owner.clone(), (*path).clone())];
                if let crate::serialization::HintOwner::Object(id) = &owner
                    && model
                        .resources
                        .get(&snapshot.document().source().uri)
                        .is_some_and(|record| record.roots.contains(id))
                {
                    keys.push((
                        crate::serialization::HintOwner::Resource(
                            snapshot.document().source().uri.clone(),
                        ),
                        (*path).clone(),
                    ));
                }
                for key in &keys {
                    *hints.counts.entry(key.clone()).or_insert(0) += 1;
                }
                if unassigned.contains(path.as_str()) {
                    let range = snapshot
                        .document()
                        .utf8_range(node.range)?
                        .ok_or("Unassigned source hint splits Unicode")?;
                    for key in keys {
                        *work = work
                            .checked_sub(range.end - range.start + 1)
                            .ok_or("Unassigned source hint work budget exceeded")?;
                        hints
                            .text
                            .entry(key)
                            .or_default()
                            .push(snapshot.document().text()[range.start..range.end].into());
                    }
                }
            }
        }
        Ok(hints)
    }
    pub fn prepare(
        package: &Package,
        host: crate::package::CompilationSourcePolicy,
        limits: AnalysisLimits,
    ) -> Result<Self, Vec<Diagnostic>> {
        Self::prepare_with_context(package, &[], None, host, limits)
    }
    pub fn prepare_with_context(
        package: &Package,
        settings: &[intersect_model::definitions::setting::ComputedSettingContract],
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
        host: crate::package::CompilationSourcePolicy,
        limits: AnalysisLimits,
    ) -> Result<Self, Vec<Diagnostic>> {
        let entry =
            entry::EntryLanguage::prepare_with_context(package, settings, conversions, host)?;
        let p = entry.package();
        if !p
            .grammar
            .as_ref()
            .is_some_and(|g| g.profile.has_durable_documents())
        {
            return Err(vec![Diagnostic::new(
                "document",
                0,
                0,
                "Durable document analysis requires explicit NativeV65",
            )]);
        }
        let origins =
            Origins::prepare(p, limits).map_err(|e| vec![Diagnostic::new("document", 0, 0, e)])?;
        Ok(Self {
            entry,
            origins: Arc::new(origins),
            limits,
        })
    }
    pub fn package(&self) -> &Package {
        self.entry.package()
    }

    /// Reconcile checked semantic associations after a source edit. Parse-stage
    /// observations deliberately retain the actual parser IDs as source evidence.
    /// Growth in retained association payload is charged before allocating it.
    pub(crate) fn reconcile_snapshot_ids(
        &self,
        snapshot: &mut DocumentSnapshot,
        parsed_to_model: &BTreeMap<String, String>,
    ) -> Result<(), String> {
        for object in &mut snapshot.semantic_objects {
            let id = parsed_to_model
                .get(&object.id)
                .ok_or("Missing reconciled document object ID")?;
            if object.id != *id {
                snapshot.association_usage.admit(self.limits, id.len(), 0)?;
                object.id = id.clone();
            }
        }
        for feature in &mut snapshot.feature_nodes {
            let id = parsed_to_model
                .get(&feature.owner)
                .ok_or("Missing reconciled feature owner ID")?;
            if feature.owner != *id {
                snapshot.association_usage.admit(self.limits, id.len(), 0)?;
                feature.owner = id.clone();
            }
        }
        Ok(())
    }
}

impl DocumentSnapshot {
    /// Rendering hints can also use explicit feature spans and accepted parser
    /// spans when construction occurred in a fragment. This does not change the
    /// durable node model's direct/inherited association contract.
    pub(crate) fn serialization_owner(
        &self,
        model: &Model,
        schema: &intersect_model::definitions::schema::Schema,
        node: NodeId,
        work: &mut usize,
    ) -> Result<Option<String>, String> {
        let debit = |work: &mut usize| -> Result<(), String> {
            *work = work
                .checked_sub(1)
                .ok_or("Serialization association work budget exceeded")?;
            Ok(())
        };
        debit(work)?;
        if let Some(index) = self.semantic_object(node)? {
            return Ok(Some(
                self.semantic_objects()
                    .get(index)
                    .ok_or("Missing semantic syntax owner")?
                    .id
                    .clone(),
            ));
        }
        let range = self
            .document()
            .nodes()
            .get(node.0)
            .ok_or("Missing syntax owner node")?
            .range;
        let mut feature_owner: Option<(usize, &str)> = None;
        for feature in self.feature_nodes() {
            debit(work)?;
            let owner = self
                .semantic_objects()
                .iter()
                .find(|e| e.id == feature.owner)
                .ok_or("Missing feature owner")?;
            if schema
                .effective_feature(&owner.class, &feature.feature)?
                .feature
                .containment
            {
                continue;
            }
            let feature_range = self
                .document()
                .nodes()
                .get(feature.node.0)
                .ok_or("Missing feature node")?
                .range;
            if feature_range.start <= range.start && range.end <= feature_range.end {
                let length = feature_range.end - feature_range.start;
                if feature_owner.is_none_or(|(old, _)| length < old) {
                    feature_owner = Some((length, &feature.owner));
                }
            }
        }
        if let Some((_, owner)) = feature_owner {
            return Ok(Some(owner.into()));
        }
        let range = self
            .document()
            .utf8_range(range)?
            .ok_or("Syntax owner splits Unicode")?;
        let mut span_owner: Option<(usize, Option<&str>)> = None;
        // Only objects associated with this accepted parse may lend spans.
        // Runtime-only/transient objects can carry older source metadata.
        let associated: std::collections::BTreeSet<_> = self
            .semantic_objects()
            .iter()
            .map(|object| object.id.as_str())
            .collect();
        for object in &model.elements {
            debit(work)?;
            if !associated.contains(object.id.as_str())
                || object.resource != self.document().source().uri
                || object.start > range.start
                || range.end > object.end
            {
                continue;
            }
            let length = object.end - object.start;
            match span_owner {
                None => span_owner = Some((length, Some(&object.id))),
                Some((old, _)) if length < old => span_owner = Some((length, Some(&object.id))),
                Some((old, _)) if length == old => span_owner = Some((old, None)),
                _ => {}
            }
        }
        Ok(span_owner.and_then(|(_, owner)| owner).map(str::to_owned))
    }
}

fn append_error_unit(units: &mut Vec<u16>, unit: Option<u16>) {
    match unit {
        None => units.extend("<EOF>".encode_utf16()),
        Some(10) => units.extend("\\n".encode_utf16()),
        Some(13) => units.extend("\\r".encode_utf16()),
        Some(9) => units.extend("\\t".encode_utf16()),
        Some(unit) => units.push(unit),
    }
}
fn unit_message(units: Vec<u16>) -> (String, Option<Vec<u16>>) {
    match String::from_utf16(&units) {
        Ok(text) => (text, None),
        Err(_) => {
            let display = std::char::decode_utf16(units.iter().copied())
                .map(|c| match c {
                    Ok(c) => c.to_string(),
                    Err(e) => format!("\\u{:04X}", e.unpaired_surrogate()),
                })
                .collect();
            (display, Some(units))
        }
    }
}
/// Recovered objects deliberately expose no ModelStore, mutation, Deserialize or
/// automatic publication operation. They remain owned analysis observations.
#[derive(Debug, Serialize)]
pub struct RecoveredObject {
    #[serde(flatten)]
    element: intersect_model::core::model::Element,
}
impl RecoveredObject {
    pub fn id(&self) -> &str {
        &self.element.id
    }
    pub fn class(&self) -> &str {
        &self.element.class
    }
    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.element.values
    }
    pub fn start(&self) -> usize {
        self.element.start
    }
    pub fn end(&self) -> usize {
        self.element.end
    }
    pub fn resource(&self) -> &str {
        &self.element.resource
    }
    pub fn type_identity(&self) -> &str {
        &self.element.type_identity
    }
}
#[derive(Debug, Serialize)]
pub struct RecoveredGraph {
    pub schema_revision: String,
    pub objects: Vec<RecoveredObject>,
    pub resources: BTreeMap<String, intersect_model::core::resources::ResourceRecord>,
}
#[derive(Debug, Serialize)]
pub struct DocumentAnalysis {
    pub snapshot: DocumentSnapshot,
    pub diagnostics: Vec<Diagnostic>,
    /// Some only after the actual full shared parse/link/validation succeeded.
    pub valid_model: Option<Evaluation>,
    pub recovered: Option<RecoveredGraph>,
}
/// A workspace analysis owns all source snapshots and one atomic publication
/// decision. Cross-resource linking remains in the same shared interpreter.
#[derive(Debug, Serialize)]
pub struct DocumentAnalysisSet {
    pub snapshots: Vec<DocumentSnapshot>,
    pub diagnostics: Vec<Diagnostic>,
    pub valid_model: Option<Evaluation>,
    pub recovered: Option<RecoveredGraph>,
}
pub(crate) struct Request<'a> {
    origins: Arc<Origins>,
    limits: AnalysisLimits,
    output: &'a mut Vec<DocumentSnapshot>,
}
impl<'a> Request<'a> {
    #[cfg(test)]
    pub(crate) fn new(
        origins: Arc<Origins>,
        limits: AnalysisLimits,
        output: &'a mut Vec<DocumentSnapshot>,
    ) -> Self {
        Self {
            origins,
            limits,
            output,
        }
    }
    pub(crate) fn limits(&self) -> AnalysisLimits {
        self.limits
    }
    pub(crate) fn capture(&self, source: &Source) -> Result<Capture, String> {
        Capture::new(source, 0, Arc::clone(&self.origins), self.limits)
    }
    pub(crate) fn capture_associated(
        &self,
        source: &Source,
        association: &DocumentResourceAssociation,
    ) -> Result<Capture, String> {
        Capture::from_associated_source(
            source,
            association,
            0,
            Arc::clone(&self.origins),
            self.limits,
        )
    }
    pub(crate) fn push(&mut self, snapshot: DocumentSnapshot) {
        self.output.push(snapshot)
    }
}
impl DocumentLanguage {
    pub fn analyze(
        &self,
        source: &Source,
        selected_entry: Option<&str>,
        limits: EvaluationLimits,
    ) -> Result<DocumentAnalysis, String> {
        self.analyze_with_context(
            source,
            selected_entry,
            limits,
            None,
            None,
            &query::PackageDerived(self.package()),
        )
    }
    pub fn analyze_with_context(
        &self,
        source: &Source,
        selected_entry: Option<&str>,
        limits: EvaluationLimits,
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
        settings: Option<&setting::TextSettings<'_>>,
        derived: &dyn query::DerivedValues,
    ) -> Result<DocumentAnalysis, String> {
        let mut all = self.analyze_sources_with_context(
            std::slice::from_ref(source),
            selected_entry,
            limits,
            conversions,
            settings,
            derived,
        )?;
        if all.snapshots.len() != 1 {
            return Err("Single document analysis did not return exactly one snapshot".into());
        }
        let snapshot = all.snapshots.pop().ok_or("Missing document snapshot")?;
        Ok(DocumentAnalysis {
            snapshot,
            diagnostics: all.diagnostics,
            valid_model: all.valid_model,
            recovered: all.recovered,
        })
    }
    pub fn analyze_sources(
        &self,
        sources: &[Source],
        selected_entry: Option<&str>,
        limits: EvaluationLimits,
    ) -> Result<DocumentAnalysisSet, String> {
        self.analyze_sources_with_context(
            sources,
            selected_entry,
            limits,
            None,
            None,
            &query::PackageDerived(self.package()),
        )
    }
    pub fn analyze_sources_with_context(
        &self,
        sources: &[Source],
        selected_entry: Option<&str>,
        limits: EvaluationLimits,
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
        settings: Option<&setting::TextSettings<'_>>,
        derived: &dyn query::DerivedValues,
    ) -> Result<DocumentAnalysisSet, String> {
        self.analyze_sources_with_services(
            sources,
            limits,
            &mut DocumentServices {
                entry: selected_entry,
                conversions,
                settings,
                derived: Some(derived),
                linking: None,
                formatter: None,
                unassigned: None,
                transient: None,
            },
        )
    }

    pub fn analyze_sources_with_services(
        &self,
        sources: &[Source],
        limits: EvaluationLimits,
        services: &mut DocumentServices<'_, '_>,
    ) -> Result<DocumentAnalysisSet, String> {
        let selected_entry = services.entry;
        if let Some(entry) = selected_entry
            && !self.entry.entries().contains_key(entry)
        {
            return Err(format!("No emitted no-argument parser entry for {entry}"));
        }
        if sources.len() > self.limits.resources {
            return Err("Document source resource budget exceeded".into());
        }
        let mut identities = BTreeSet::new();
        let mut bytes = 0usize;
        for source in sources {
            intersect_model::core::resources::validate_resource_uri(&source.uri)?;
            if !identities.insert(source.uri.as_str()) {
                return Err("Duplicate document source URI".into());
            }
            bytes = bytes
                .checked_add(source.uri.len())
                .and_then(|v| v.checked_add(source.text.len()))
                .and_then(|v| v.checked_add(logical_debit::SNAPSHOT))
                .ok_or("Document source aggregate storage overflow")?;
            if source.text.len() > self.limits.tree.source_bytes
                || bytes > self.limits.total_source_bytes
            {
                return Err("Document source byte budget exceeded".into());
            }
        }
        let package = self.package();
        let mut store = new_evaluation_store(package)?;
        let mut documents = Vec::new();
        let result = storage::atomic(&mut store, |store| {
            evaluate_staged_with_linking(
                package,
                sources,
                Invocation {
                    limits,
                    entry: selected_entry,
                    values: None,
                    documents: Some(Request {
                        origins: Arc::clone(&self.origins),
                        limits: self.limits,
                        output: &mut documents,
                    }),
                },
                store,
                services.conversions,
                services.settings,
                services.derived.unwrap_or(&query::PackageDerived(package)),
                services.linking.as_deref_mut(),
            )
        })?;
        let model = store.into_model()?;
        let bound = bind_snapshots(documents, sources, &model, &package.schema, self.limits)?;
        let diagnostics = result.diagnostics;
        if diagnostics.is_empty()
            && bound.iter().all(|snapshot| {
                snapshot.syntax_issues.is_empty() && snapshot.lexical_issues.is_empty()
            })
        {
            Ok(DocumentAnalysisSet {
                snapshots: bound,
                diagnostics: Vec::new(),
                valid_model: Some(Evaluation {
                    model,
                    diagnostics,
                    references: result.references,
                    derived: result.derived,
                }),
                recovered: None,
            })
        } else {
            let recovered = recovered_graph(model);
            Ok(DocumentAnalysisSet {
                snapshots: bound,
                diagnostics,
                valid_model: None,
                recovered: Some(recovered),
            })
        }
    }
}

pub(crate) fn recovered_graph(model: Model) -> RecoveredGraph {
    RecoveredGraph {
        schema_revision: model.schema_revision,
        objects: model
            .elements
            .into_iter()
            .map(|element| RecoveredObject { element })
            .collect(),
        resources: model.resources,
    }
}

/// Same source/model identity association for Package and explicit raw drafts.
pub(crate) fn bind_snapshots(
    documents: Vec<DocumentSnapshot>,
    sources: &[Source],
    model: &Model,
    schema: &intersect_model::definitions::schema::Schema,
    limits: AnalysisLimits,
) -> Result<Vec<DocumentSnapshot>, String> {
    bind_snapshots_with_resources(documents, sources, model, schema, limits, |_| Ok(None))
}
pub(crate) fn bind_snapshots_with_resources<'a>(
    mut documents: Vec<DocumentSnapshot>,
    sources: &[Source],
    model: &Model,
    schema: &intersect_model::definitions::schema::Schema,
    limits: AnalysisLimits,
    mut association: impl FnMut(&SourceId) -> Result<Option<&'a DocumentResourceAssociation>, String>,
) -> Result<Vec<DocumentSnapshot>, String> {
    if documents.len() != sources.len() {
        return Err("Document analysis did not produce one snapshot per exact source".into());
    }
    for (snapshot, source) in documents.iter_mut().zip(sources) {
        if snapshot.document.source() != &SourceId::for_text(&source.uri, &source.text) {
            return Err("Document snapshot source order/identity mismatch".into());
        }
        let associated = association(snapshot.document.source())?;
        let resource_uri = associated.map_or(
            source.uri.as_str(),
            DocumentResourceAssociation::resource_uri,
        );
        let resource = model
            .resources
            .get(resource_uri)
            .ok_or("Missing analyzed resource record")?;
        // bind_resource consumes the immutable arena, so the completed source-owned
        // snapshot is replaced in the vector only by its returned validated value.
        match associated {
            Some(row) => snapshot.bind_model_with_association(model, schema, limits, Some(row))?,
            None => snapshot.bind_model(model, schema, limits)?,
        }
        if resource.uri != resource_uri
            || resource.source.as_ref().is_none_or(|s| {
                s.uri != snapshot.document.source().uri
                    || s.revision != snapshot.document.source().sha256
            })
        {
            return Err("Document resource lacks exact source correspondence".into());
        }
    }
    // Resource revisions are assigned by actual shared model lifecycle, never a
    // caller-supplied serialized node arena. Move each arena through its binder.
    let mut bound = Vec::new();
    for mut snapshot in documents {
        let associated = association(snapshot.document.source())?;
        let resource_uri = associated.map_or(
            snapshot.document.source().uri.as_str(),
            DocumentResourceAssociation::resource_uri,
        );
        let resource = model
            .resources
            .get(resource_uri)
            .ok_or("Missing document resource")?;
        snapshot.document = match associated {
            Some(row) => snapshot.document.bind_associated_resource(row, resource)?,
            None => snapshot.document.bind_resource(resource)?,
        };
        bound.push(snapshot);
    }
    Ok(bound)
}

#[cfg(test)]
mod draft_origin_controls {
    use super::*;

    #[test]
    fn draft_capture_origin_is_real_bounded_and_optional() -> Result<(), String> {
        let source = Source {
            uri: "memory:origin-input".into(),
            text: "x".into(),
        };
        let grammar_source = SourceId::for_text("memory:origin-grammar", "Root: 'x';");
        let declaration = SourceSpan {
            source: grammar_source,
            start: 0,
            end: 10,
        };
        let origins = Arc::new(Origins {
            entries: vec![GrammarOrigin {
                kind: OriginKind::ParserRule,
                declaration: Some(declaration.clone()),
                expression: None,
                target: None,
                builtin: None,
            }],
            by_key: [("rule/Root/declaration".into(), 0)].into(),
        });
        let mut capture = Capture::new(
            &source,
            0,
            Arc::clone(&origins),
            AnalysisLimits::workbench(),
        )?;
        assert_eq!(capture.active_origin()?, None);
        let node = capture.begin("rule/Root/declaration", 0)?;
        let index = capture
            .active_origin()?
            .ok_or("Missing actual capture origin")?;
        assert_eq!(index, 0);
        assert_eq!(
            origins
                .indexed_origin(index)
                .and_then(|origin| origin.declaration.as_ref()),
            Some(&declaration)
        );
        assert!(origins.indexed_origin(usize::MAX).is_none());
        let mut data = capture.builder.node_data(node)?;
        data.origin = None;
        capture.builder.set_data(node, data)?;
        assert_eq!(capture.active_origin()?, None);
        data.origin = Some(usize::MAX);
        capture.builder.set_data(node, data)?;
        assert!(
            capture
                .active_origin()?
                .and_then(|index| origins.indexed_origin(index))
                .is_none()
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "rule_trampoline_tests.rs"]
mod rule_trampoline_tests;
