//! Handwritten finite specialization of validated parameter environments.
//! Generated Xtext programs reuse the shared compiler, prediction and model store.
use super::*;
use std::collections::VecDeque;

pub(super) struct Program {
    pub bundle: DefinitionBundle,
    pub producers: BTreeMap<SourceId, SourceId>,
    pub entry: SourceId,
    pub bindings: Vec<grammar::GeneratedRuleTypes>,
    pub origins: Vec<GeneratedOrigin>,
    pub parameterized_rules: BTreeSet<String>,
    pub generated: Vec<DefinitionSource>,
}
struct Input {
    source: DefinitionSource,
    syntax: Syntax,
    parameters: BTreeMap<String, Vec<Located<String>>>,
}
// Generated text and authored AST scopes move together through nested rewrites.
// These transient ranges are diagnostics, never serialized executable authority.
#[derive(Default)]
struct Rendered {
    text: String,
    origins: Vec<(usize, usize, SourceSpan)>,
}
impl Rendered {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            origins: vec![],
        }
    }
    fn authored(mut self, source: &SourceId, start: usize, end: usize) -> Self {
        if !self.text.is_empty() {
            self.origins.push((
                0,
                self.text.len(),
                SourceSpan {
                    source: source.clone(),
                    start,
                    end,
                },
            ));
        }
        self
    }
    fn append(&mut self, other: Self) {
        let offset = self.text.len();
        self.text.push_str(&other.text);
        self.origins.extend(
            other
                .origins
                .into_iter()
                .map(|(start, end, original)| (offset + start, offset + end, original)),
        );
    }
}
type State = (String, Vec<bool>);
struct Builder {
    inputs: Vec<Input>,
    declarations: BTreeMap<String, Vec<(usize, usize)>>,
    names: BTreeMap<State, String>,
    used: BTreeSet<String>,
    queue: VecDeque<State>,
    rendered: BTreeMap<(usize, usize, Vec<bool>), Rendered>,
    rendered_bytes: Vec<usize>,
    admitted: usize,
    limits: SourceLimits,
    entry: SourceId,
    preserve_implicit_types: bool,
}
impl Builder {
    fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(&self.entry.uri, 0, 1, message).with_context(&self.entry)
    }
    fn text<'a>(&self, source: &'a DefinitionSource, start: usize, end: usize) -> Result<&'a str> {
        source
            .text
            .get(start..end)
            .ok_or_else(|| self.error("Invalid native parameter source span"))
    }
    fn state(&mut self, name: &str, values: Vec<bool>) -> Result<String> {
        let key = (name.to_owned(), values);
        if let Some(name) = self.names.get(&key) {
            return Ok(name.clone());
        }
        let declarations = self
            .declarations
            .get(name)
            .ok_or_else(|| self.error("Missing native parameter declaration"))?;
        let count = self
            .admitted
            .checked_add(declarations.len())
            .ok_or_else(|| self.error("Parameter declaration count overflow"))?;
        if count > self.limits.declarations {
            return Err(self.error("Specialized parameter declaration budget exceeded"));
        }
        self.admitted = count;
        let selected = if key.1.iter().all(|value| !value) {
            name.to_owned()
        } else {
            let mut ordinal = self.names.len();
            loop {
                let candidate = format!("_intersect_parameter_{ordinal}");
                if self.used.insert(candidate.clone()) {
                    break candidate;
                }
                ordinal = ordinal
                    .checked_add(1)
                    .ok_or_else(|| self.error("Parameter name count overflow"))?;
            }
        };
        self.names.insert(key.clone(), selected.clone());
        self.queue.push_back(key);
        Ok(selected)
    }
    fn rewrite(
        &self,
        source: &DefinitionSource,
        start: usize,
        end: usize,
        mut changes: Vec<(usize, usize, Rendered)>,
    ) -> Result<Rendered> {
        changes.sort_by_key(|change| change.0);
        let mut result = Rendered::default();
        let mut cursor = start;
        for (a, b, value) in changes {
            if a < cursor || b > end || a > b {
                return Err(self.error("Overlapping parameter source rewrites"));
            }
            result.text.push_str(self.text(source, cursor, a)?);
            result.append(value);
            cursor = b;
            if result.text.len() > self.limits.source_bytes {
                return Err(self.error("Specialized parameter source size budget exceeded"));
            }
        }
        result.text.push_str(self.text(source, cursor, end)?);
        if result.text.len() > self.limits.source_bytes {
            return Err(self.error("Specialized parameter source size budget exceeded"));
        }
        Ok(result)
    }
    fn element(
        &mut self,
        source: &DefinitionSource,
        parameters: &[Located<String>],
        values: &[bool],
        node: &Element,
    ) -> Result<Rendered> {
        Ok(self
            .element_inner(source, parameters, values, node)?
            .authored(&source.id, node.start, node.end))
    }
    fn element_inner(
        &mut self,
        source: &DefinitionSource,
        parameters: &[Located<String>],
        values: &[bool],
        node: &Element,
    ) -> Result<Rendered> {
        match &node.value {
            ElementKind::Call {
                rule,
                arguments: supplied,
            } => {
                let local = rule.value.strip_prefix("super::").unwrap_or(&rule.value);
                let target = self
                    .inputs
                    .iter()
                    .find(|input| input.source.id == source.id)
                    .and_then(|input| input.parameters.get(&rule.value))
                    .cloned();
                if let Some(target) = target.filter(|target| !target.is_empty()) {
                    let arguments = parameters::arguments(
                        &source.id,
                        parameters,
                        values,
                        &target,
                        supplied.as_deref(),
                        node,
                    )?;
                    let selected = self.state(local, arguments)?;
                    return if selected == local {
                        Ok(Rendered::new(self.text(source, rule.start, rule.end)?))
                    } else {
                        Ok(Rendered::new(format!(
                            "{}{selected}",
                            if rule.value.starts_with("super::") {
                                "super::"
                            } else {
                                ""
                            }
                        )))
                    };
                }
            }
            ElementKind::Guard {
                condition,
                elements,
            } => {
                let selected = parameters::condition(&source.id, parameters, values, condition)?;
                let mut changes = vec![(
                    condition.start,
                    condition.end,
                    Rendered::new(selected.to_string()).authored(
                        &source.id,
                        condition.start,
                        condition.end,
                    ),
                )];
                for element in elements {
                    changes.push((
                        element.start,
                        element.end,
                        self.element(source, parameters, values, element)?,
                    ));
                }
                return self.rewrite(source, node.start, node.end, changes);
            }
            _ => {}
        }
        let mut changes = vec![];
        for child in children(node) {
            changes.push((
                child.start,
                child.end,
                self.element(source, parameters, values, child)?,
            ));
        }
        self.rewrite(source, node.start, node.end, changes)
    }
    fn render(&mut self) -> Result<()> {
        while let Some((name, values)) = self.queue.pop_front() {
            let positions = self
                .declarations
                .get(&name)
                .cloned()
                .ok_or_else(|| self.error("Missing queued parameter declaration"))?;
            for (source_index, rule_index) in positions {
                let source = self.inputs[source_index].source.clone();
                let rule = self.inputs[source_index].syntax.rules[rule_index].clone();
                let parameters = rule.value.parameters.as_deref().unwrap_or_default();
                if parameters.len() != values.len() {
                    return Err(self.error("Incompatible inherited parameter environment"));
                }
                let body = self.element(&source, parameters, &values, &rule.value.body)?;
                let selected = self
                    .names
                    .get(&(name.clone(), values.clone()))
                    .cloned()
                    .ok_or_else(|| self.error("Missing specialized rule name"))?;
                let mut changes = vec![(rule.value.body.start, rule.value.body.end, body)];
                if selected != name {
                    changes.push((
                        rule.value.name.start,
                        rule.value.name.end,
                        Rendered::new(selected).authored(
                            &source.id,
                            rule.value.name.start,
                            rule.value.name.end,
                        ),
                    ));
                    if self.preserve_implicit_types
                        && rule.value.kind == RuleKind::Parser
                        && rule.value.return_type.is_none()
                    {
                        // Renaming a specialization must not invent a classifier
                        // named after the generated rule. The original false seed
                        // still checks the authored implicit-return binding policy.
                        changes.push((
                            rule.value.name.end,
                            rule.value.name.end,
                            Rendered::new(format!(" returns {name}")).authored(
                                &source.id,
                                rule.value.name.start,
                                rule.value.name.end,
                            ),
                        ));
                    }
                }
                if rule.value.parameters.is_some() {
                    let header_text =
                        self.text(&source, rule.value.name.end, rule.value.body.start)?;
                    let tokens = grammar::lex_with_limit(header_text, None)
                        .map_err(|(_, message)| self.error(message))?;
                    let header = tokens.iter().collect::<Vec<_>>();
                    let opening = header
                        .iter()
                        .find(|token| !token.quoted && token.text == "<")
                        .ok_or_else(|| self.error("Missing parameter declaration opening"))?;
                    let closing = header
                        .iter()
                        .find(|token| {
                            !token.quoted && token.text == ">" && token.start > opening.start
                        })
                        .ok_or_else(|| self.error("Missing parameter declaration closing"))?;
                    changes.push((
                        rule.value.name.end + opening.start,
                        rule.value.name.end + closing.end,
                        Rendered::default(),
                    ));
                }
                let rendered = self
                    .rewrite(&source, rule.start, rule.end, changes)?
                    .authored(&source.id, rule.start, rule.end);
                let retained = self.rendered_bytes[source_index]
                    .checked_add(rendered.text.len())
                    .and_then(|bytes| bytes.checked_add(1))
                    .ok_or_else(|| self.error("Specialized parameter source size overflow"))?;
                if retained > self.limits.source_bytes {
                    return Err(self.error("Specialized parameter source size budget exceeded"));
                }
                self.rendered_bytes[source_index] = retained;
                self.rendered
                    .insert((source_index, rule_index, values.clone()), rendered);
            }
        }
        Ok(())
    }
}
fn children(node: &Element) -> Vec<&Element> {
    match &node.value {
        ElementKind::Sequence(items)
        | ElementKind::Alternatives(items)
        | ElementKind::Unordered(items) => items.iter().collect(),
        ElementKind::Group(item)
        | ElementKind::Cardinality { element: item, .. }
        | ElementKind::Predicate { element: item, .. }
        | ElementKind::Assignment { value: item, .. }
        | ElementKind::Negated(item)
        | ElementKind::Until(item) => vec![item],
        ElementKind::CrossReference {
            spelling: Some(item),
            ..
        } => vec![item],
        ElementKind::Range { left, right } => vec![left, right],
        ElementKind::EnumLiteral {
            keyword: Some(item),
            ..
        } => vec![item],
        ElementKind::Guard { elements, .. } => elements.iter().collect(),
        _ => vec![],
    }
}
fn needs_program(rule: &syntax::Rule) -> bool {
    fn visit(node: &Element) -> bool {
        matches!(
            node.value,
            ElementKind::Guard { .. }
                | ElementKind::Call {
                    arguments: Some(_),
                    ..
                }
        ) || children(node).into_iter().any(visit)
    }
    rule.parameters.is_some() || visit(&rule.body)
}

pub(super) fn requires(syntax: &Syntax) -> bool {
    syntax.rules.iter().any(|rule| needs_program(&rule.value))
}

fn program_header(source: &DefinitionSource) -> String {
    format!(
        "// Generated native parameter program; original digest {}\n",
        source.id.sha256
    )
}

pub(super) fn prepare(
    definitions: &InferredMetamodels,
    limits: SourceLimits,
) -> Result<Option<Program>> {
    prepare_sources(
        &definitions.bundle,
        &definitions.original.id,
        &definitions.grammars,
        limits,
        true,
    )
}

pub(super) fn prepare_imported(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    grammars: &[GrammarInference],
    limits: SourceLimits,
) -> Result<Option<Program>> {
    prepare_sources(bundle, entry, grammars, limits, false)
}

fn prepare_sources(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    grammars: &[GrammarInference],
    limits: SourceLimits,
    inferred_types: bool,
) -> Result<Option<Program>> {
    if !grammars.iter().any(|grammar| grammar.parameterized) {
        return Ok(None);
    }
    let mut inputs = vec![];
    for grammar in grammars {
        let source = bundle
            .source(&grammar.source)
            .map_err(|e| {
                Diagnostic::new(&grammar.source.uri, 0, 1, e).with_context(&grammar.source)
            })?
            .clone();
        let syntax = syntax::parse(&source, limits)?;
        let parameters = grammar
            .scope
            .rules
            .iter()
            .map(|(name, binding)| (name.clone(), binding.parameters.clone()))
            .collect();
        inputs.push(Input {
            source,
            syntax,
            parameters,
        });
    }
    if !inputs.iter().any(|input| {
        input
            .syntax
            .rules
            .iter()
            .any(|rule| needs_program(&rule.value))
    }) {
        return Ok(None);
    }
    let mut declarations: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let mut parameterized_rules = BTreeSet::new();
    for (source, input) in inputs.iter().enumerate() {
        for (rule, value) in input.syntax.rules.iter().enumerate() {
            declarations
                .entry(value.value.name.value.clone())
                .or_default()
                .push((source, rule));
            if value.value.parameters.is_some() {
                parameterized_rules.insert(value.value.name.value.clone());
            }
        }
    }
    let rendered_bytes = inputs
        .iter()
        .map(|input| {
            let first = input
                .syntax
                .rules
                .first()
                .map_or(input.source.text.len(), |rule| rule.start);
            program_header(&input.source)
                .len()
                .checked_add(first)
                .filter(|bytes| *bytes <= limits.source_bytes)
                .ok_or_else(|| {
                    Diagnostic::new(
                        &input.source.id.uri,
                        0,
                        1,
                        "Specialized parameter source size budget exceeded",
                    )
                    .with_context(&input.source.id)
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let used = declarations.keys().cloned().collect();
    let mut builder = Builder {
        inputs,
        declarations,
        names: BTreeMap::new(),
        used,
        queue: VecDeque::new(),
        rendered: BTreeMap::new(),
        rendered_bytes,
        admitted: 0,
        limits,
        entry: entry.clone(),
        preserve_implicit_types: !inferred_types,
    };
    let seeds = builder
        .declarations
        .iter()
        .map(|(name, places)| {
            let (source, rule) = places[0];
            let count = builder.inputs[source].syntax.rules[rule]
                .value
                .parameters
                .as_ref()
                .map_or(0, Vec::len);
            (name.clone(), vec![false; count])
        })
        .collect::<Vec<_>>();
    for (name, values) in seeds {
        builder.state(&name, values)?;
    }
    builder.render()?;
    for ((name, _), selected) in &builder.names {
        if name != selected {
            parameterized_rules.insert(selected.clone());
        }
    }
    let mut identity = bundle
        .sources
        .iter()
        .map(|source| source.id.clone())
        .collect::<Vec<_>>();
    identity.sort();
    let context = serde_json::to_string(&(identity, &bundle.namespaces))
        .map_err(|e| builder.error(e.to_string()))?;
    let context = SourceId::for_text("intersect:parameter-program:v1", &context).sha256;
    let mut generated = vec![];
    let mut pending_origins = vec![];
    let mut bindings = vec![];
    for (source_index, input) in builder.inputs.iter().enumerate() {
        let first = input
            .syntax
            .rules
            .first()
            .map_or(input.source.text.len(), |rule| rule.start);
        let mut text = program_header(&input.source);
        text.push_str(builder.text(&input.source, 0, first)?);
        let mut types = BTreeMap::new();
        let mut origins = vec![];
        for (rule_index, rule) in input.syntax.rules.iter().enumerate() {
            for ((index, ordinal, values), body) in &builder.rendered {
                if *index != source_index || *ordinal != rule_index {
                    continue;
                }
                let start = text.len();
                text.push_str(&body.text);
                origins.extend(
                    body.origins
                        .iter()
                        .map(|(a, b, original)| (start + a, start + b, original.clone())),
                );
                text.push('\n');
                if text.len() > limits.source_bytes {
                    return Err(builder.error("Specialized parameter source size budget exceeded"));
                }
                origins.push((
                    start,
                    text.len(),
                    SourceSpan {
                        source: input.source.id.clone(),
                        start: rule.start,
                        end: rule.end,
                    },
                ));
                let selected = builder
                    .names
                    .get(&(rule.value.name.value.clone(), values.clone()))
                    .ok_or_else(|| builder.error("Missing generated parameter binding"))?;
                if inferred_types
                    && let Some(target) = grammars[source_index]
                        .rules
                        .get(&rule.value.name.value)
                        .and_then(|binding| binding.target.as_ref())
                {
                    types.insert(
                        selected.clone(),
                        (target.namespace.clone(), target.name.clone()),
                    );
                }
            }
        }
        let source = DefinitionSource::new(
            format!(
                "intersect:parameter-program:{context}:{}",
                input.source.id.sha256
            ),
            SourceKind::Xtext,
            text,
        );
        if bundle
            .sources
            .iter()
            .any(|original| original.id == source.id)
        {
            return Err(builder.error("Generated parameter source collides with an input identity"));
        }
        origins.push((
            0,
            source.text.len(),
            SourceSpan {
                source: input.source.id.clone(),
                start: 0,
                end: input.source.text.len(),
            },
        ));
        pending_origins.push(origins);
        bindings.push(grammar::GeneratedRuleTypes {
            source: source.id.clone(),
            types,
        });
        generated.push(source);
    }
    let mapping = builder
        .inputs
        .iter()
        .zip(&generated)
        .map(|(before, after)| (before.source.id.clone(), after.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut origins = vec![];
    for ((input, source), locations) in builder
        .inputs
        .iter()
        .zip(&mut generated)
        .zip(pending_origins)
    {
        source.dependencies = input
            .source
            .dependencies
            .iter()
            .map(|dependency| mapping.get(dependency).unwrap_or(dependency).clone())
            .collect();
        source.dependencies.push(input.source.id.clone());
        for (start, end, original) in locations {
            origins.push(GeneratedOrigin {
                generated: SourceSpan {
                    source: source.id.clone(),
                    start,
                    end,
                },
                original: vec![original],
            });
        }
    }
    let entry = mapping
        .get(entry)
        .cloned()
        .ok_or_else(|| builder.error("Missing generated parameter entry source"))?;
    let mut bundle = bundle.clone();
    bundle.sources.extend(generated.iter().cloned());
    bundle.validate().map_err(|e| builder.error(e))?;
    let producers = mapping
        .into_iter()
        .map(|(original, program)| (program, original))
        .collect();
    Ok(Some(Program {
        bundle,
        entry,
        bindings,
        origins,
        parameterized_rules,
        generated,
        producers,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> InferenceRequest {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/xtext/inference-parameters/cases.json"
        ))
        .unwrap();
        let source = DefinitionSource::new(
            "memory:parameter-test.xtext",
            SourceKind::Xtext,
            cases["lowercase_counterparts"][0]["sources"][0]["source"]
                .as_str()
                .unwrap(),
        );
        InferenceRequest {
            version: 1,
            grammar: source.id.clone(),
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![source],
                namespaces: vec![],
            },
        }
    }
    fn prepared(request: &InferenceRequest) -> (InferredMetamodels, Program) {
        let definitions =
            infer_metamodels(&request.bundle, &request.grammar, SourceLimits::default()).unwrap();
        let program = prepare(&definitions, SourceLimits::default())
            .unwrap()
            .unwrap();
        assert_eq!(
            program.bundle.sources[..definitions.bundle.sources.len()],
            definitions.bundle.sources
        );
        (definitions, program)
    }
    fn compile(program: &Program, definitions: &InferredMetamodels) -> Result<grammar::Grammar> {
        grammar::compile_generated_source_programs(
            &program.bundle,
            &program.entry,
            &mut definitions.schema.clone(),
            grammar::Profile::NativeV57,
            SourceLimits::default(),
            &program.bindings,
            &program.producers,
        )
    }
    #[test]
    fn nested_specialization_scopes_keep_authored_calls_conditions_and_whole_rules() {
        let request = request();
        let (_, program) = prepared(&request);
        let mut call = false;
        let mut condition = false;
        for origin in &program.origins {
            let source = program.bundle.source(&origin.generated.source).unwrap();
            let generated = source
                .text
                .get(origin.generated.start..origin.generated.end)
                .unwrap();
            for original in &origin.original {
                let source = request.bundle.source(&original.source).unwrap();
                let authored = source.text.get(original.start..original.end).unwrap();
                call |= authored == "Node<true>" && generated.starts_with("_intersect_parameter_");
                condition |= authored == "Flag" && ["true", "false"].contains(&generated);
            }
        }
        assert!(call, "specialized names retain the complete authored call");
        assert!(
            condition,
            "constant conditions retain the authored boolean expression"
        );
        let source = &program.generated[0];
        let syntax = syntax::parse(source, SourceLimits::default()).unwrap();
        for rule in syntax.rules {
            let generated = SourceSpan {
                source: source.id.clone(),
                start: rule.start,
                end: rule.end,
            };
            let diagnostic = emit::project_diagnostic(
                Diagnostic::from_span(generated.clone(), "whole declaration"),
                &program.origins,
            );
            let Some(DiagnosticOrigin::Generated {
                generated: actual,
                originals,
            }) = diagnostic.origin.as_deref()
            else {
                panic!("missing generated ownership");
            };
            assert_eq!(*actual, generated);
            assert_eq!(originals.len(), 1);
            let original = &originals[0];
            let original_syntax = syntax::parse(
                request.bundle.source(&original.source).unwrap(),
                SourceLimits::default(),
            )
            .unwrap();
            assert!(
                original_syntax
                    .rules
                    .iter()
                    .any(|rule| rule.start == original.start && rule.end == original.end)
            );
        }
    }
    #[test]
    fn derived_programs_require_exact_producer_dependencies_and_matching_headers() {
        let request = request();
        let (definitions, mut program) = prepared(&request);
        assert!(compile(&program, &definitions).is_ok());
        let original = program.producers[&program.entry].clone();
        let source = program
            .bundle
            .sources
            .iter_mut()
            .find(|s| s.id == program.entry)
            .unwrap();
        source
            .dependencies
            .retain(|dependency| dependency != &original);
        assert!(
            compile(&program, &definitions)
                .unwrap_err()
                .message
                .contains("exact authored producer dependency")
        );
        let (definitions, mut program) = prepared(&request);
        let previous = program.entry.clone();
        let source = program
            .bundle
            .sources
            .iter_mut()
            .find(|s| s.id == previous)
            .unwrap();
        source.text = source.text.replace(
            "oracle.parameters.matrix.truecall.Language",
            "oracle.parameters.matrix.changed.Language",
        );
        source.id = SourceId::for_text(&source.id.uri, &source.text);
        program.entry = source.id.clone();
        let producer = program.producers.remove(&previous).unwrap();
        program.producers.insert(program.entry.clone(), producer);
        program
            .bindings
            .iter_mut()
            .find(|b| b.source == previous)
            .unwrap()
            .source = program.entry.clone();
        assert!(
            compile(&program, &definitions)
                .unwrap_err()
                .message
                .contains("changed its authored producer header context")
        );
        let (definitions, mut program) = prepared(&request);
        let producer = program.producers.get_mut(&program.entry).unwrap();
        producer.sha256 = "0".repeat(64);
        assert!(
            compile(&program, &definitions)
                .unwrap_err()
                .message
                .contains("exact authored producer dependency")
        );
    }
    #[test]
    fn specialization_limits_apply_after_valid_original_inference_and_recover() {
        let request = request();
        let limits = SourceLimits {
            declarations: 5,
            ..SourceLimits::default()
        };
        assert!(infer_metamodels(&request.bundle, &request.grammar, limits).is_ok());
        let errors = request.compile(limits).err().unwrap();
        assert!(errors.iter().any(|e| {
            e.message
                .contains("Specialized parameter declaration budget")
        }));
        assert!(request.compile(SourceLimits::default()).is_ok());
        let (definitions, program) = prepared(&request);
        let exact = program
            .generated
            .iter()
            .map(|source| source.text.len())
            .max()
            .unwrap();
        assert!(
            prepare(
                &definitions,
                SourceLimits {
                    source_bytes: exact,
                    ..SourceLimits::default()
                }
            )
            .is_ok()
        );
        assert!(
            prepare(
                &definitions,
                SourceLimits {
                    source_bytes: exact - 1,
                    ..SourceLimits::default()
                }
            )
            .err()
            .unwrap()
            .message
            .contains("Specialized parameter source size budget")
        );
        assert!(request.compile(SourceLimits::default()).is_ok());
    }
    #[test]
    fn opaque_source_uris_retain_identity_and_reject_controls() {
        let mut request = request();
        let source = &mut request.bundle.sources[0];
        source.id.uri = "memory:opaque name?value=/*not syntax*/".into();
        request.grammar = source.id.clone();
        let language = request.compile(SourceLimits::default()).unwrap();
        assert_eq!(language.request().grammar, request.grammar);
        let result = crate::evaluate(
            language.package(),
            &[crate::Source {
                uri: "memory:model".into(),
                text: "alpha".into(),
            }],
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let mut invalid = request.clone();
        invalid.bundle.sources[0].id.uri.push('\n');
        invalid.grammar = invalid.bundle.sources[0].id.clone();
        let errors = invalid.compile(SourceLimits::default()).err().unwrap();
        assert!(
            errors
                .iter()
                .any(|error| error.message.contains("contain no controls"))
        );
    }
    #[test]
    fn source_record_order_preserves_program_identity_without_rewriting_original_requests() {
        let mut first = request();
        first.bundle.sources.push(DefinitionSource::new(
            &first.grammar.uri,
            SourceKind::Xtext,
            "grammar unused.Revision",
        ));
        let mut second = first.clone();
        second.bundle.sources.reverse();
        let a = first.compile(SourceLimits::default()).unwrap();
        let b = second.compile(SourceLimits::default()).unwrap();
        assert_eq!(
            serde_json::to_value(&a.package().grammar).unwrap(),
            serde_json::to_value(&b.package().grammar).unwrap()
        );
        assert_eq!(a.package().sources, b.package().sources);
        assert_eq!(
            a.package().xtext_inference.as_ref().unwrap().bundle.sources,
            first.bundle.sources
        );
        assert_eq!(
            b.package().xtext_inference.as_ref().unwrap().bundle.sources,
            second.bundle.sources
        );
    }
    #[test]
    fn parameter_archives_derive_profiles_from_sources_and_reject_mismatched_metadata() {
        let request = request();
        let distribution = crate::distribution::build_inferred(
            request,
            crate::distribution::PackageMetadata {
                package_id: "parameter.archive".into(),
                release: "1".into(),
                display_name: "Parameter archive".into(),
                description: String::new(),
            },
        )
        .unwrap();
        assert_eq!(
            distribution.manifest.runtime.profile,
            "intersect.xtext.native.v57"
        );
        let mut files = distribution.files.clone();
        files.insert(
            "manifest.json".into(),
            serde_json::to_string(&distribution.manifest).unwrap(),
        );
        let loaded = crate::distribution::import_files(files.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(loaded.package).unwrap(),
            serde_json::to_value(distribution.package).unwrap()
        );
        let mut manifest: serde_json::Value =
            serde_json::from_str(&files["manifest.json"]).unwrap();
        manifest["runtime"]["profile"] = serde_json::json!("intersect.xtext.native.v56");
        files.insert(
            "manifest.json".into(),
            serde_json::to_string(&manifest).unwrap(),
        );
        assert!(
            crate::distribution::import_files(files)
                .err()
                .unwrap()
                .contains("runtime requirements differ")
        );
    }
}
