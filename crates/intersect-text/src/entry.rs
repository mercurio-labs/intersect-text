//! Handwritten source-authoritative named entries over the shared interpreter.
//! Entry selection is invocation state, never serialized executable authority.
use super::*;
use crate::grammar::SourceLimits;
use crate::grammar::syntax;
use crate::grammar::syntax::RuleKind;
use crate::package::CompilationSourcePolicy;
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum EntryValue {
    Object(String),
    /// The generated datatype parser's raw token text, before AST conversion.
    Datatype(String),
    Empty,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntryEvaluation {
    pub evaluation: Evaluation,
    pub values: BTreeMap<String, EntryValue>,
}
#[derive(Clone, Debug)]
pub struct EntryRule {
    pub declaration: SourceSpan,
    pub datatype: bool,
}
/// A rebuilt package and its actual public entry catalog. Existing Package DTOs,
/// compiler profiles and archive payloads retain their original contracts.
pub struct EntryLanguage {
    package: Package,
    entries: BTreeMap<String, EntryRule>,
}
impl EntryLanguage {
    pub fn prepare(
        package: &Package,
        host: CompilationSourcePolicy,
    ) -> Result<Self, Vec<Diagnostic>> {
        Self::prepare_with_context(package, &[], None, host)
    }
    pub fn prepare_with_context(
        package: &Package,
        settings: &[intersect_model::definitions::setting::ComputedSettingContract],
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
        host: CompilationSourcePolicy,
    ) -> Result<Self, Vec<Diagnostic>> {
        let package = crate::recompile_with_context(package, settings, conversions, host)?;
        let failure = |message: &str| vec![Diagnostic::new("entry", 0, 0, message)];
        let (bundle, source) = if let Some(inputs) = &package.xtext_inference {
            (&inputs.bundle, &inputs.grammar)
        } else if let Some(inputs) = &package.compilation {
            (
                &inputs.bundle,
                inputs
                    .xtext
                    .as_ref()
                    .ok_or_else(|| failure("Named entries require authoritative grammar inputs"))?,
            )
        } else {
            return Err(failure("Named entries require original compilation inputs"));
        };
        let policy = package.source_policy.unwrap_or_default();
        let declarations = declarations(
            bundle,
            source,
            SourceLimits {
                source_bytes: policy.xtext_bytes,
                declarations: policy.xtext_declarations,
            },
        )?;
        let grammar = package
            .grammar
            .as_ref()
            .ok_or_else(|| failure("Package has no text grammar"))?;
        // The existing compiler already specialized argument environments and
        // erased false guarded bodies. Its real call graph excludes validation
        // seeds and dead calls without a second parameter implementation.
        let mut reached = BTreeSet::new();
        let mut pending = vec![grammar.entry.as_str()];
        while let Some(name) = pending.pop() {
            if !reached.insert(name) {
                continue;
            }
            let Some(rule) = grammar.rules.get(name) else {
                continue;
            };
            let mut expressions = vec![&rule.body];
            while let Some(expr) = expressions.pop() {
                match &expr.kind {
                    Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => {
                        pending.push(name)
                    }
                    Kind::Assign { value, .. }
                    | Kind::BooleanAssign { value, .. }
                    | Kind::SyntacticPredicate(value) => expressions.push(value),
                    Kind::Sequence(items) | Kind::Alternative(items) => expressions.extend(items),
                    Kind::Action(_)
                    | Kind::Capture { .. }
                    | Kind::Literal(_)
                    | Kind::Reference(_) => {}
                }
            }
        }
        let entries = declarations
            .into_iter()
            .filter_map(|(name, (kind, span))| {
                let rule = grammar.rules.get(&name)?;
                (kind == RuleKind::Parser && !rule.fragment && reached.contains(name.as_str()))
                    .then_some((
                        name,
                        EntryRule {
                            declaration: span,
                            datatype: rule.datatype.is_some(),
                        },
                    ))
            })
            .collect();
        Ok(Self { package, entries })
    }
    pub fn package(&self) -> &Package {
        &self.package
    }
    pub fn entries(&self) -> &BTreeMap<String, EntryRule> {
        &self.entries
    }
    pub fn evaluate(
        &self,
        entry: &str,
        sources: &[Source],
        limits: EvaluationLimits,
    ) -> Result<EntryEvaluation, String> {
        self.evaluate_with_context(
            entry,
            sources,
            limits,
            None,
            None,
            &query::PackageDerived(&self.package),
        )
    }
    pub fn evaluate_with_context(
        &self,
        entry: &str,
        sources: &[Source],
        limits: EvaluationLimits,
        conversions: Option<&intersect_model::definitions::conversion::ConversionRegistry<'_>>,
        settings: Option<&setting::TextSettings<'_>>,
        derived: &dyn query::DerivedValues,
    ) -> Result<EntryEvaluation, String> {
        if !self.entries.contains_key(entry) {
            if sources.is_empty() {
                return Err(format!("No emitted no-argument parser entry for {entry}"));
            }
            return Ok(EntryEvaluation {
                evaluation: Evaluation {
                    model: Model {
                        schema_revision: self.package.schema.revision.clone(),
                        ..Model::default()
                    },
                    diagnostics: sources
                        .iter()
                        .map(|s| {
                            Diagnostic::new(
                                &s.uri,
                                0,
                                0,
                                format!("No emitted no-argument parser entry for {entry}"),
                            )
                        })
                        .collect(),
                    references: vec![],
                    derived: BTreeMap::new(),
                },
                values: BTreeMap::new(),
            });
        }
        let mut store = new_evaluation_store(&self.package)?;
        let mut values = BTreeMap::new();
        let result = storage::atomic(&mut store, |store| {
            evaluate_staged(
                &self.package,
                sources,
                Invocation {
                    limits,
                    entry: Some(entry),
                    values: Some(&mut values),
                    documents: None,
                },
                store,
                conversions,
                settings,
                derived,
            )
        })?;
        Ok(EntryEvaluation {
            evaluation: Evaluation {
                model: store.into_model()?,
                diagnostics: result.diagnostics,
                references: result.references,
                derived: result.derived,
            },
            values,
        })
    }
}

fn declarations(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    limits: SourceLimits,
) -> Result<BTreeMap<String, (RuleKind, SourceSpan)>, Vec<Diagnostic>> {
    let mut pending = vec![(entry.clone(), false)];
    let mut seen = BTreeSet::new();
    let mut parsed = BTreeMap::new();
    let mut declarations = BTreeMap::new();
    while let Some((id, complete)) = pending.pop() {
        let fail = |message: String| vec![Diagnostic::at_source(&id, 0, 1, message)];
        if complete {
            let syntax = parsed
                .remove(&id)
                .ok_or_else(|| fail("Missing prepared entry grammar".into()))?;
            let syntax: syntax::Syntax = syntax;
            for rule in syntax.rules {
                declarations.insert(
                    rule.value.name.value.clone(),
                    (
                        rule.value.kind,
                        SourceSpan {
                            source: id.clone(),
                            start: rule.start,
                            end: rule.end,
                        },
                    ),
                );
            }
            continue;
        }
        if !seen.insert(id.clone()) {
            continue;
        }
        let source = bundle.source(&id).map_err(fail)?;
        let syntax = syntax::parse(source, limits).map_err(|e| vec![e])?;
        pending.push((id.clone(), true));
        for parent in &syntax.parents {
            let mut selected = None;
            for dependency in &source.dependencies {
                let candidate = bundle.source(dependency).map_err(fail)?;
                if candidate.kind == SourceKind::Xtext
                    && syntax::grammar_name(candidate, limits)
                        .map_err(|e| vec![e])?
                        .value
                        == parent.value
                    && selected.replace(candidate.id.clone()).is_some()
                {
                    return Err(fail("Ambiguous exact parent grammar".into()));
                }
            }
            match selected {
                Some(id) => pending.push((id, false)),
                None if parent.value == "org.eclipse.xtext.common.Terminals" => {}
                None => return Err(fail("Missing exact parent grammar".into())),
            }
        }
        parsed.insert(id, syntax);
    }
    Ok(declarations)
}
