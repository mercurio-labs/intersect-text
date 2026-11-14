//! Grammar-specific composition over the existing exact source bundle.
use super::{Grammar, Parser, Profile, analyze_grammar, definition_tokens, parse_declarations};
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceId;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::bundle::SourceSpan;
use intersect_model::definitions::schema::Diagnostic;
use intersect_model::definitions::schema::Schema;
use std::collections::{BTreeMap, BTreeSet};

/// Explicit admission policy for exact grammar source composition. This changes
/// resource admission only, never the selected syntax capability profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLimits {
    pub source_bytes: usize,
    /// Includes terminals, parser rules and retained shadowed declarations.
    pub declarations: usize,
}
impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            source_bytes: 256_000,
            declarations: 128,
        }
    }
}

fn error(source: &SourceId, message: impl Into<String>) -> Diagnostic {
    Diagnostic::at_source(source, 0, 1, message)
}
fn header(
    source: &DefinitionSource,
    schema: &Schema,
    limits: SourceLimits,
    profile: Profile,
    package_imports: &std::rc::Rc<crate::grammar::PackageImports<'_>>,
) -> Result<(super::SourceHeader, bool), Diagnostic> {
    if source.text.len() > limits.source_bytes {
        return Err(error(&source.id, "Definition size budget exceeded"));
    }
    let tokens =
        definition_tokens(&source.text, profile, limits.declarations).map_err(|mut error| {
            error.source = source.id.uri.clone();
            error.with_source(&source.id)
        })?;
    let mut parser = Parser {
        capture_origins: false,
        unordered: None,
        explicit_local_scope: matches!(
            profile,
            Profile::NativeV45
                | Profile::NativeV46
                | Profile::NativeV47
                | Profile::NativeV48
                | Profile::NativeV49
                | Profile::NativeV50
                | Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ),
        package_imports: std::rc::Rc::clone(package_imports),
        allow_first_predicates: false,
        allow_constant_guards: profile.has_constant_guards(),
        tokens,
        pos: 0,
        depth: 0,
        aliases: BTreeMap::new(),
        metamodel_imports: matches!(
            profile,
            Profile::NativeV20
                | Profile::NativeV21
                | Profile::NativeV22
                | Profile::NativeV23
                | Profile::NativeV24
                | Profile::NativeV25
                | Profile::NativeV26
                | Profile::NativeV27
                | Profile::NativeV28
                | Profile::NativeV29
                | Profile::NativeV30
                | Profile::NativeV31
                | Profile::NativeV32
                | Profile::NativeV33
                | Profile::NativeV34
                | Profile::NativeV35
                | Profile::NativeV36
                | Profile::NativeV37
                | Profile::NativeV38
                | Profile::NativeV39
                | Profile::NativeV40
                | Profile::NativeV41
                | Profile::NativeV42
                | Profile::NativeV43
                | Profile::NativeV44
                | Profile::NativeV45
                | Profile::NativeV46
                | Profile::NativeV47
                | Profile::NativeV48
                | Profile::NativeV49
                | Profile::NativeV50
                | Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ),
        hidden: None,
    };
    let result = parser.source_header(schema, true).map_err(|e| {
        let at = parser
            .tokens
            .get(parser.pos.saturating_sub(1))
            .map_or(0, |t| t.start);
        Diagnostic::at_source(&source.id, at, at + 1, e)
    })?;
    Ok((
        result,
        parser
            .aliases
            .values()
            .flatten()
            .any(|uri| uri == intersect_model::definitions::ecore::ECORE),
    ))
}

fn selected_parent<'a>(
    bundle: &'a DefinitionBundle,
    source: &'a DefinitionSource,
    source_header: &super::SourceHeader,
    schema: &Schema,
    limits: SourceLimits,
    profile: Profile,
    imports: &std::rc::Rc<super::PackageImports<'_>>,
) -> Result<Option<&'a SourceId>, Diagnostic> {
    let Some(parent_name) = &source_header.parent else {
        return Ok(None);
    };
    let parent_error = |message: String| {
        let span = source_header.parent_span.clone().unwrap_or(0..1);
        Diagnostic::at_source(&source.id, span.start, span.end, message)
    };
    let mut selected = None;
    for dependency in &source.dependencies {
        let candidate = bundle
            .source(dependency)
            .map_err(|e| error(&source.id, e))?;
        if candidate.kind != SourceKind::Xtext {
            continue;
        }
        if header(candidate, schema, limits, profile, imports)?.0.name == *parent_name
            && selected.replace(dependency).is_some()
        {
            return Err(parent_error("Ambiguous parent grammar identity".into()));
        }
    }
    selected.map(Some).ok_or_else(|| {
        parent_error(format!(
            "Missing exact parent grammar dependency {parent_name}"
        ))
    })
}

/// Inspect canonical reference requirements from exact authored headers and
/// the same parent selection as composition. No grammar body is admitted here.
pub fn source_reference_context_requirement<'a>(
    bundle: &'a DefinitionBundle,
    entry: &'a SourceId,
    schema: &Schema,
    profile: Profile,
    limits: SourceLimits,
) -> Result<Option<&'a SourceId>, Diagnostic> {
    let imports = std::rc::Rc::new(super::PackageImports::default());
    let mut names = BTreeSet::new();
    let mut current = entry;
    let mut first = None;
    loop {
        let source = bundle.source(current).map_err(|e| error(current, e))?;
        if source.kind != SourceKind::Xtext {
            return Err(error(current, "Selected grammar source must be Xtext"));
        }
        let (source_header, imports_ecore) = header(source, schema, limits, profile, &imports)?;
        if !names.insert(source_header.name.clone()) {
            return Err(error(
                current,
                "Duplicate grammar identity in inheritance chain",
            ));
        }
        if imports_ecore {
            first.get_or_insert(current);
        }
        match selected_parent(
            bundle,
            source,
            &source_header,
            schema,
            limits,
            profile,
            &imports,
        )? {
            Some(parent) => current = parent,
            None => return Ok(first),
        }
    }
}

/// Compile one explicitly selected grammar and its exact declared parent sources.
/// This does not load resources or install language-specific scope/conversion.
/// Contextual profiles admit bounded model-parser overrides. NativeV15 also
/// admits same-datatype terminal overrides; multiple parents reject.
pub fn compile_sources(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
) -> Result<Grammar, Diagnostic> {
    compile_sources_with_limits(bundle, entry, schema, profile, SourceLimits::default())
}

/// Opt-in source admission; failure restores the caller's reference context.
pub fn compile_sources_with_limits(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
    limits: SourceLimits,
) -> Result<Grammar, Diagnostic> {
    compile_sources_with_settings_and_limits(bundle, entry, schema, profile, limits, &[])
}

/// Compose exact sources with explicit computed-setting requirements.
/// Every contract must bind every grammar in the selected inheritance chain,
/// including shadowed declarations. This admits metadata, not executable providers.
/// Failure preserves the caller's reference context as in ordinary composition.
pub fn compile_sources_with_settings_and_limits(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
    limits: SourceLimits,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
) -> Result<Grammar, Diagnostic> {
    compile_sources_with_package_imports(
        bundle,
        entry,
        schema,
        profile,
        limits,
        contracts,
        crate::grammar::PackageImports::default(),
    )
}

pub(crate) fn compile_sources_with_package_imports(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
    limits: SourceLimits,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
    package_imports: crate::grammar::PackageImports,
) -> Result<Grammar, Diagnostic> {
    compile_sources_with_package_imports_observed(
        bundle,
        entry,
        schema,
        profile,
        limits,
        contracts,
        package_imports,
        None,
    )
}

#[allow(clippy::too_many_arguments)] // Keep observation separate from existing compiler context and grammar layouts.
pub(crate) fn compile_sources_with_package_imports_observed(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
    limits: SourceLimits,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
    package_imports: crate::grammar::PackageImports,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
) -> Result<Grammar, Diagnostic> {
    compile_sources_with_package_imports_prefix_inspected(
        bundle,
        entry,
        schema,
        profile,
        limits,
        contracts,
        package_imports,
        observation,
        None,
    )
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
pub fn compile_sources_with_package_imports_prefix_inspected(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
    limits: SourceLimits,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
    package_imports: crate::grammar::PackageImports,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
) -> Result<Grammar, Diagnostic> {
    compile_sources_with_package_imports_prefix_inspected_frontier_selected(
        bundle,
        entry,
        schema,
        profile,
        limits,
        contracts,
        package_imports,
        observation,
        prefix,
        super::RecursiveFrontierOrder::Historical,
    )
}
#[allow(clippy::too_many_arguments)] // Same worker plus explicit transient search order.
pub(crate) fn compile_sources_with_package_imports_prefix_inspected_frontier_selected(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
    limits: SourceLimits,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
    package_imports: crate::grammar::PackageImports,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
    order: super::RecursiveFrontierOrder,
) -> Result<Grammar, Diagnostic> {
    compile_sources_with_package_imports_prefix_inspected_frontier_selected_pool(
        bundle,
        entry,
        schema,
        profile,
        limits,
        contracts,
        package_imports,
        observation,
        prefix,
        order,
        None,
    )
}
#[allow(clippy::too_many_arguments)] // One transient caller-owned diagnostic; same worker.
pub fn compile_sources_with_package_imports_prefix_inspected_frontier_selected_pool(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
    limits: SourceLimits,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
    package_imports: crate::grammar::PackageImports,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
    order: super::RecursiveFrontierOrder,
    pool: Option<&super::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
) -> Result<Grammar, Diagnostic> {
    compile_sources_with_package_imports_prediction_policy(
        bundle,
        entry,
        schema,
        profile,
        limits,
        contracts,
        package_imports,
        observation,
        prefix,
        order,
        pool,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn compile_sources_with_package_imports_prediction_policy(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    schema: &mut impl super::StructureContext,
    profile: Profile,
    limits: SourceLimits,
    contracts: &[intersect_model::definitions::setting::ComputedSettingContract],
    package_imports: crate::grammar::PackageImports,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
    order: super::RecursiveFrontierOrder,
    pool: Option<&super::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
    runtime: Option<&mut super::RuntimeGuardedAdmission>,
) -> Result<Grammar, Diagnostic> {
    bundle
        .validate()
        .map_err(|e| error(entry, e).with_context(entry))?;
    if !profile.has_lexical_prediction() {
        return Err(error(
            entry,
            "Grammar source composition requires the predictive native profile",
        ));
    }
    if profile.has_terminal_cardinality_decisions() {
        let original = package_imports.setting_sources.get(entry).unwrap_or(entry);
        let validate = package_imports.validate_authored.ok_or_else(|| {
            error(
                original,
                "NativeV63 requires fresh language-owned authored-source validation",
            )
        })?;
        validate(
            bundle,
            original,
            package_imports.authored_source_limits.unwrap_or(limits),
        )?;
    }
    let mut context = schema.reference_context();
    let schema = &mut context;
    let result = (|| {
        let mut state = CompositionState {
            package_imports: std::rc::Rc::new(package_imports),
            contracts: contracts.to_vec(),
            ..CompositionState::default()
        };
        let mut grammar = compose(
            bundle,
            entry,
            schema,
            profile,
            &mut state,
            limits,
            observation,
        )?;
        if grammar.entry.is_empty() {
            return Err(error(entry, "Grammar requires a parser entry"));
        }
        if profile.has_explicit_terminal_calls() {
            state.retained.prepare(&mut grammar)?;
        }
        analyze_with_sources_prediction_policy(
            &mut grammar,
            schema,
            entry,
            false,
            observation,
            prefix,
            order,
            pool,
            runtime,
        )?;
        Ok(grammar)
    })();
    if result.is_ok() {
        context.commit();
    }
    result
}
fn analyze_with_sources(
    grammar: &mut Grammar,
    schema: &Schema,
    entry: &SourceId,
    library: bool,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
) -> Result<(), Diagnostic> {
    analyze_with_sources_prefix_inspected(grammar, schema, entry, library, observation, None)
}

#[allow(clippy::too_many_arguments)] // Explicit transient diagnostic; old facades pass None.
fn analyze_with_sources_prefix_inspected(
    grammar: &mut Grammar,
    schema: &Schema,
    entry: &SourceId,
    library: bool,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
) -> Result<(), Diagnostic> {
    analyze_with_sources_prefix_inspected_frontier_selected(
        grammar,
        schema,
        entry,
        library,
        observation,
        prefix,
        super::RecursiveFrontierOrder::Historical,
    )
}
#[allow(clippy::too_many_arguments)] // Same worker plus explicit transient search order.
fn analyze_with_sources_prefix_inspected_frontier_selected(
    grammar: &mut Grammar,
    schema: &Schema,
    entry: &SourceId,
    library: bool,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
    order: super::RecursiveFrontierOrder,
) -> Result<(), Diagnostic> {
    analyze_with_sources_prefix_inspected_frontier_selected_pool(
        grammar,
        schema,
        entry,
        library,
        observation,
        prefix,
        order,
        None,
    )
}
#[allow(clippy::too_many_arguments)] // One transient caller-owned diagnostic; same worker.
fn analyze_with_sources_prefix_inspected_frontier_selected_pool(
    grammar: &mut Grammar,
    schema: &Schema,
    entry: &SourceId,
    library: bool,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
    order: super::RecursiveFrontierOrder,
    pool: Option<&super::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
) -> Result<(), Diagnostic> {
    analyze_with_sources_prediction_policy(
        grammar,
        schema,
        entry,
        library,
        observation,
        prefix,
        order,
        pool,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
fn analyze_with_sources_prediction_policy(
    grammar: &mut Grammar,
    schema: &Schema,
    entry: &SourceId,
    library: bool,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
    prefix: Option<&super::SourcePrefixDiagnostic<'_>>,
    order: super::RecursiveFrontierOrder,
    pool: Option<&super::admission_observation::pool_blocks::CompilerPoolDiagnostic>,
    runtime: Option<&mut super::RuntimeGuardedAdmission>,
) -> Result<(), Diagnostic> {
    let observed = observation.map(|observation| super::AnalysisObservation {
        observation: observation.legacy.phases,
        recursive_work: observation.legacy.recursive_work,
        adjacent_work: observation.adjacent_work,
        entry,
        stage: if library {
            super::admission_observation::RunStage::Parent
        } else {
            super::admission_observation::RunStage::Executable
        },
    });
    if let Some(observation) = observation {
        observation.analysis_reached();
    }
    let result = if library
        && matches!(
            grammar.profile,
            Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ) {
        if observed.is_some() {
            super::analyze_declarations_observed(grammar, schema, false, observed)
        } else {
            super::analyze_parent(grammar, schema)
        }
    } else if matches!(
        grammar.profile,
        Profile::NativeV50
            | Profile::NativeV51
            | Profile::NativeV52
            | Profile::NativeV53
            | Profile::NativeV54
            | Profile::NativeV55
            | Profile::NativeV56
            | Profile::NativeV57
            | Profile::NativeV58
            | Profile::NativeV59
            | Profile::NativeV60
            | Profile::NativeV61
            | Profile::NativeV62
            | Profile::NativeV63
            | Profile::NativeV64
            | Profile::NativeV65
            | Profile::NativeV66
            | Profile::NativeV67
            | Profile::NativeV68
            | Profile::NativeV69
            | Profile::NativeV70
            | Profile::NativeV71
            | Profile::NativeV72
            | Profile::NativeV73
            | Profile::NativeV74
            | Profile::NativeV75
            | Profile::NativeV76
            | Profile::NativeV77
            | Profile::NativeV78
            | Profile::NativeV79
            | Profile::NativeV80
            | Profile::NativeV81
            | Profile::NativeV82
    ) && grammar.entry.is_empty()
        && grammar.rules.is_empty()
    {
        if observed.is_some() {
            super::analyze_terminal_parent_observed(
                grammar,
                observed.map(|mut context| {
                    context.stage = super::admission_observation::RunStage::TerminalParent;
                    context
                }),
            )
        } else {
            super::analyze_terminal_parent(grammar)
        }
    } else if observed.is_some()
        || prefix.is_some()
        || order != super::RecursiveFrontierOrder::Historical
        || pool.is_some()
        || runtime.is_some()
    {
        super::analyze_declarations_prediction_policy(
            grammar, schema, true, observed, prefix, order, pool, runtime,
        )
    } else {
        analyze_grammar(grammar, schema)
    };
    result.map_err(|failure| {
        let producer = failure
            .producer_declaration
            .as_ref()
            .and_then(|name| grammar.declaration_sources.get(name))
            .cloned();
        let diagnostic = if let Some(span) = failure
            .declaration
            .as_ref()
            .and_then(|name| grammar.declaration_sources.get(name))
        {
            Diagnostic::from_span(span.clone(), failure.message)
        } else {
            error(entry, failure.message).with_context(entry)
        };
        if let Some(span) = producer {
            diagnostic.with_span(span)
        } else {
            diagnostic
        }
    })
}

fn retained_count(grammar: &Grammar) -> usize {
    grammar.rules.len()
        + grammar.terminals.len()
        + grammar
            .shadowed_declarations
            .values()
            .map(Vec::len)
            .sum::<usize>()
}
#[derive(Default)]
struct CompositionState<'policy> {
    package_imports: std::rc::Rc<crate::grammar::PackageImports<'policy>>,
    contracts: Vec<intersect_model::definitions::setting::ComputedSettingContract>,
    active: BTreeSet<SourceId>,
    names: BTreeSet<String>,
    retained: super::explicit_calls::Registry,
    admission: BTreeMap<SourceId, super::first_rule::Summary>,
}
fn compose(
    bundle: &DefinitionBundle,
    id: &SourceId,
    schema: &mut super::ReferenceContextTransaction<'_>,
    profile: Profile,
    state: &mut CompositionState,
    limits: SourceLimits,
    observation: Option<super::admission_observation::AnalysisWorkObservers<'_>>,
) -> Result<Grammar, Diagnostic> {
    if state.active.len() >= 64 {
        return Err(error(id, "Grammar inheritance nesting budget exceeded"));
    }
    if !state.active.insert(id.clone()) {
        return Err(error(id, "Cyclic grammar inheritance"));
    }
    let source = bundle
        .source(id)
        .map_err(|e| error(id, e).with_context(id))?;
    if source.kind != SourceKind::Xtext {
        return Err(error(id, "Selected grammar source must be Xtext"));
    }
    if source.text.len() > limits.source_bytes {
        return Err(error(id, "Definition size budget exceeded"));
    }
    let (source_header, imports_ecore) =
        header(source, schema, limits, profile, &state.package_imports)?;
    let name = source_header.name.clone();
    if !state.names.insert(name.clone()) {
        return Err(error(id, "Duplicate grammar identity in inheritance chain"));
    }
    if imports_ecore {
        super::install_reference_top(schema).map_err(|e| error(id, e))?;
    }
    let mut parent_identity = None;
    let parent = if let Some(selected) = selected_parent(
        bundle,
        source,
        &source_header,
        schema,
        limits,
        profile,
        &state.package_imports,
    )? {
        let parent = compose(
            bundle,
            selected,
            schema,
            profile,
            state,
            limits,
            observation,
        )?;
        parent_identity = Some(selected.clone());
        if parent.name == name {
            return Err(error(id, "Duplicate grammar identity in inheritance chain"));
        }
        Some(parent)
    } else {
        None
    };
    let hidden = parent.as_ref().and_then(|g| g.hidden.clone());
    let (mut grammar, _, spans, _) = parse_declarations(
        &source.text,
        schema,
        profile,
        true,
        hidden,
        limits.declarations,
        std::rc::Rc::clone(&state.package_imports),
    )
    .map_err(|mut e| {
        e.source = id.uri.clone();
        e.with_source(id)
    })?;
    if grammar.entry.is_empty() {
        if matches!(
            profile,
            Profile::NativeV51
                | Profile::NativeV52
                | Profile::NativeV53
                | Profile::NativeV54
                | Profile::NativeV55
                | Profile::NativeV56
                | Profile::NativeV57
                | Profile::NativeV58
                | Profile::NativeV59
                | Profile::NativeV60
                | Profile::NativeV61
                | Profile::NativeV62
                | Profile::NativeV63
                | Profile::NativeV64
                | Profile::NativeV65
                | Profile::NativeV66
                | Profile::NativeV67
                | Profile::NativeV68
                | Profile::NativeV69
                | Profile::NativeV70
                | Profile::NativeV71
                | Profile::NativeV72
                | Profile::NativeV73
                | Profile::NativeV74
                | Profile::NativeV75
                | Profile::NativeV76
                | Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ) || (profile == Profile::NativeV50 && grammar.rules.is_empty())
        {
            grammar.entry = parent
                .as_ref()
                .map_or_else(String::new, |parent| parent.entry.clone());
        } else {
            grammar.entry = parent
                .as_ref()
                .ok_or_else(|| error(id, "Inherited parser entry is missing"))?
                .entry
                .clone();
        }
    }
    if !state.contracts.is_empty() {
        let producer = state
            .package_imports
            .setting_sources
            .get(&source.id)
            .map(|id| {
                bundle
                    .source(id)
                    .map_err(|message| error(&source.id, message))
            })
            .transpose()?
            .unwrap_or(source);
        super::validate_setting_contracts(producer, schema, profile, bundle, &state.contracts)?;
        grammar.computed_settings = state.contracts.clone();
    }
    for (name, range) in spans {
        grammar.declaration_sources.insert(
            name,
            SourceSpan {
                source: id.clone(),
                start: range.start,
                end: range.end,
            },
        );
    }
    if profile.has_explicit_terminal_calls() {
        state
            .retained
            .retain(&grammar, parent.as_ref().map(|g| g.name.clone()));
    }
    let mut local_terminals: Vec<_> = grammar.terminals.iter().collect();
    local_terminals.sort_by_key(|(_, terminal)| terminal.start);
    grammar.terminal_order = local_terminals
        .into_iter()
        .map(|(name, _)| name.clone())
        .collect();
    if let Some(mut parent) = parent {
        if retained_count(&grammar).saturating_add(retained_count(&parent)) > limits.declarations {
            return Err(error(id, "Grammar rule budget exceeded"));
        }
        if profile.has_parser_overrides() {
            // A shadowed body still has to be supported in its declaring context.
            let parent_source = if matches!(
                profile,
                Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) || (profile == Profile::NativeV50
                && parent.entry.is_empty()
                && parent.rules.is_empty())
            {
                parent_identity
                    .clone()
                    .ok_or_else(|| error(id, "Parent source identity is missing"))?
            } else {
                parent
                    .declaration_sources
                    .get(&parent.entry)
                    .ok_or_else(|| error(id, "Inherited entry source is missing"))?
                    .source
                    .clone()
            };
            if profile.has_explicit_terminal_calls() {
                let mut checked = parent.clone();
                state.retained.prepare(&mut checked)?;
                analyze_with_sources(&mut checked, schema, &parent_source, true, observation)?;
            } else {
                analyze_with_sources(&mut parent, schema, &parent_source, true, observation)?;
            }
        }
        if profile.has_return_inference() {
            // Resolve child declarations with inherited calls before comparing
            // override types, without discarding either source's provenance.
            let mut effective = grammar.clone();
            for (name, rule) in &parent.rules {
                if matches!(
                    profile,
                    Profile::NativeV21
                        | Profile::NativeV22
                        | Profile::NativeV23
                        | Profile::NativeV24
                        | Profile::NativeV25
                        | Profile::NativeV26
                        | Profile::NativeV27
                        | Profile::NativeV28
                        | Profile::NativeV29
                        | Profile::NativeV30
                        | Profile::NativeV31
                        | Profile::NativeV32
                        | Profile::NativeV33
                        | Profile::NativeV34
                        | Profile::NativeV35
                        | Profile::NativeV36
                        | Profile::NativeV37
                        | Profile::NativeV38
                        | Profile::NativeV39
                        | Profile::NativeV40
                        | Profile::NativeV41
                        | Profile::NativeV42
                        | Profile::NativeV43
                        | Profile::NativeV44
                        | Profile::NativeV45
                        | Profile::NativeV46
                        | Profile::NativeV47
                        | Profile::NativeV48
                        | Profile::NativeV49
                        | Profile::NativeV50
                        | Profile::NativeV51
                        | Profile::NativeV52
                        | Profile::NativeV53
                        | Profile::NativeV54
                        | Profile::NativeV55
                        | Profile::NativeV56
                        | Profile::NativeV57
                        | Profile::NativeV58
                        | Profile::NativeV59
                        | Profile::NativeV60
                        | Profile::NativeV61
                        | Profile::NativeV62
                        | Profile::NativeV63
                        | Profile::NativeV64
                        | Profile::NativeV65
                        | Profile::NativeV66
                        | Profile::NativeV67
                        | Profile::NativeV68
                        | Profile::NativeV69
                        | Profile::NativeV70
                        | Profile::NativeV71
                        | Profile::NativeV72
                        | Profile::NativeV73
                        | Profile::NativeV74
                        | Profile::NativeV75
                        | Profile::NativeV76
                        | Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                ) && effective.terminals.contains_key(name)
                {
                    continue;
                }
                effective
                    .rules
                    .entry(name.clone())
                    .or_insert_with(|| rule.clone());
            }
            for (name, terminal) in &parent.terminals {
                effective
                    .terminals
                    .entry(name.clone())
                    .or_insert_with(|| terminal.clone());
            }
            if profile.has_explicit_parser_calls() {
                for (name, span) in &parent.declaration_sources {
                    effective
                        .declaration_sources
                        .entry(name.clone())
                        .or_insert_with(|| span.clone());
                }
                state.retained.prepare(&mut effective)?;
            }
            super::datatype::infer(&mut effective, schema).map_err(|failure| {
                let span = grammar
                    .declaration_sources
                    .get(failure.declaration.as_deref().unwrap_or(""))
                    .or_else(|| {
                        parent
                            .declaration_sources
                            .get(failure.declaration.as_deref().unwrap_or(""))
                    });
                span.map_or_else(
                    || error(id, &failure.message),
                    |span| Diagnostic::from_span(span.clone(), &failure.message),
                )
            })?;
            for (name, rule) in &mut grammar.rules {
                if let Some(resolved) = effective.rules.get(name) {
                    if profile.has_explicit_parser_calls() {
                        rule.class = resolved.class.clone();
                        rule.datatype = resolved.datatype;
                    } else {
                        *rule = resolved.clone();
                    }
                }
            }
        }
        for (name, rule) in &grammar.rules {
            if rule.override_declared && !parent.rules.contains_key(name) {
                return Err(error(
                    id,
                    format!("@Override has no inherited parser declaration: {name}"),
                ));
            }
        }
        for (name, terminal) in &grammar.terminals {
            if terminal.override_declared
                && !parent.terminals.contains_key(name)
                && !(matches!(
                    profile,
                    Profile::NativeV21
                        | Profile::NativeV22
                        | Profile::NativeV23
                        | Profile::NativeV24
                        | Profile::NativeV25
                        | Profile::NativeV26
                        | Profile::NativeV27
                        | Profile::NativeV28
                        | Profile::NativeV29
                        | Profile::NativeV30
                        | Profile::NativeV31
                        | Profile::NativeV32
                        | Profile::NativeV33
                        | Profile::NativeV34
                        | Profile::NativeV35
                        | Profile::NativeV36
                        | Profile::NativeV37
                        | Profile::NativeV38
                        | Profile::NativeV39
                        | Profile::NativeV40
                        | Profile::NativeV41
                        | Profile::NativeV42
                        | Profile::NativeV43
                        | Profile::NativeV44
                        | Profile::NativeV45
                        | Profile::NativeV46
                        | Profile::NativeV47
                        | Profile::NativeV48
                        | Profile::NativeV49
                        | Profile::NativeV50
                        | Profile::NativeV51
                        | Profile::NativeV52
                        | Profile::NativeV53
                        | Profile::NativeV54
                        | Profile::NativeV55
                        | Profile::NativeV56
                        | Profile::NativeV57
                        | Profile::NativeV58
                        | Profile::NativeV59
                        | Profile::NativeV60
                        | Profile::NativeV61
                        | Profile::NativeV62
                        | Profile::NativeV63
                        | Profile::NativeV64
                        | Profile::NativeV65
                        | Profile::NativeV66
                        | Profile::NativeV67
                        | Profile::NativeV68
                        | Profile::NativeV69
                        | Profile::NativeV70
                        | Profile::NativeV71
                        | Profile::NativeV72
                        | Profile::NativeV73
                        | Profile::NativeV74
                        | Profile::NativeV75
                        | Profile::NativeV76
                        | Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                ) && parent.rules.contains_key(name))
            {
                return Err(error(
                    id,
                    format!("@Override has no inherited terminal declaration: {name}"),
                ));
            }
        }
        for name in parent.rules.keys().chain(parent.terminals.keys()) {
            if grammar.rules.contains_key(name) || grammar.terminals.contains_key(name) {
                if parent.final_rules.contains(name) {
                    let span = grammar
                        .declaration_sources
                        .get(name)
                        .ok_or_else(|| error(id, "Override declaration source is missing"))?;
                    return Err(Diagnostic::new(
                        &span.source.uri,
                        span.start,
                        span.end,
                        format!("Cannot override final rule {name}"),
                    )
                    .with_span(span.clone()));
                }
                if !profile.has_parser_overrides() {
                    return Err(error(
                        id,
                        format!("Grammar rule override is unsupported: {name}"),
                    ));
                }
                let compatible = grammar
                    .rules
                    .get(name)
                    .zip(parent.rules.get(name))
                    .is_some_and(|(child, base)| {
                        if profile.has_typed_overrides() {
                            if child.fragment != base.fragment
                                || child.wildcard != base.wildcard
                                || child.datatype != base.datatype
                            {
                                return false;
                            }
                            if child.fragment || child.datatype == Some(super::Datatype::Enum) {
                                return child.class == base.class;
                            }
                            child.datatype.is_some() || schema.conforms(&child.class, &base.class)
                        } else {
                            !child.fragment
                                && !base.fragment
                                && child.datatype.is_none()
                                && base.datatype.is_none()
                                && child.class == base.class
                        }
                    });
                let terminal_compatible = profile.has_terminal_overrides()
                    && grammar
                        .terminals
                        .get(name)
                        .zip(parent.terminals.get(name))
                        .is_some_and(|(child, base)| {
                            child.datatype == base.datatype
                                && child.enumeration == base.enumeration
                                && (!child.fragment || base.fragment)
                        });
                let datatype_to_terminal = matches!(
                    profile,
                    Profile::NativeV21
                        | Profile::NativeV22
                        | Profile::NativeV23
                        | Profile::NativeV24
                        | Profile::NativeV25
                        | Profile::NativeV26
                        | Profile::NativeV27
                        | Profile::NativeV28
                        | Profile::NativeV29
                        | Profile::NativeV30
                        | Profile::NativeV31
                        | Profile::NativeV32
                        | Profile::NativeV33
                        | Profile::NativeV34
                        | Profile::NativeV35
                        | Profile::NativeV36
                        | Profile::NativeV37
                        | Profile::NativeV38
                        | Profile::NativeV39
                        | Profile::NativeV40
                        | Profile::NativeV41
                        | Profile::NativeV42
                        | Profile::NativeV43
                        | Profile::NativeV44
                        | Profile::NativeV45
                        | Profile::NativeV46
                        | Profile::NativeV47
                        | Profile::NativeV48
                        | Profile::NativeV49
                        | Profile::NativeV50
                        | Profile::NativeV51
                        | Profile::NativeV52
                        | Profile::NativeV53
                        | Profile::NativeV54
                        | Profile::NativeV55
                        | Profile::NativeV56
                        | Profile::NativeV57
                        | Profile::NativeV58
                        | Profile::NativeV59
                        | Profile::NativeV60
                        | Profile::NativeV61
                        | Profile::NativeV62
                        | Profile::NativeV63
                        | Profile::NativeV64
                        | Profile::NativeV65
                        | Profile::NativeV66
                        | Profile::NativeV67
                        | Profile::NativeV68
                        | Profile::NativeV69
                        | Profile::NativeV70
                        | Profile::NativeV71
                        | Profile::NativeV72
                        | Profile::NativeV73
                        | Profile::NativeV74
                        | Profile::NativeV75
                        | Profile::NativeV76
                        | Profile::NativeV77
                        | Profile::NativeV78
                        | Profile::NativeV79
                        | Profile::NativeV80
                        | Profile::NativeV81
                        | Profile::NativeV82
                ) && grammar
                    .terminals
                    .get(name)
                    .zip(parent.rules.get(name))
                    .is_some_and(|(child, base)| {
                        !child.fragment
                            && !base.fragment
                            && base.datatype == Some(child.datatype)
                            && (base.datatype != Some(super::Datatype::Enum)
                                || child.enumeration.as_deref() == Some(base.class.as_str()))
                    });
                if !compatible && !terminal_compatible && !datatype_to_terminal {
                    if profile.has_terminal_overrides()
                        && (grammar.terminals.contains_key(name)
                            || parent.terminals.contains_key(name))
                    {
                        return Err(error(
                            id,
                            format!(
                                "Incompatible terminal override {name}: terminal category and datatype must match; a fragment cannot replace a non-fragment"
                            ),
                        ));
                    }
                    if profile.has_typed_overrides() {
                        return Err(error(
                            id,
                            format!(
                                "Parser override must preserve fragment/datatype identity or return a subtype: {name}"
                            ),
                        ));
                    }
                    return Err(error(
                        id,
                        format!(
                            "Override requires non-fragment parser rules with the same model return type: {name}"
                        ),
                    ));
                }
                let span = parent
                    .declaration_sources
                    .get(name)
                    .ok_or_else(|| error(id, "Inherited declaration source is missing"))?;
                grammar
                    .shadowed_declarations
                    .entry(name.clone())
                    .or_default()
                    .push(span.clone());
            }
        }
        grammar.final_rules.extend(parent.final_rules);
        for (name, rule) in parent.rules {
            if matches!(
                profile,
                Profile::NativeV21
                    | Profile::NativeV22
                    | Profile::NativeV23
                    | Profile::NativeV24
                    | Profile::NativeV25
                    | Profile::NativeV26
                    | Profile::NativeV27
                    | Profile::NativeV28
                    | Profile::NativeV29
                    | Profile::NativeV30
                    | Profile::NativeV31
                    | Profile::NativeV32
                    | Profile::NativeV33
                    | Profile::NativeV34
                    | Profile::NativeV35
                    | Profile::NativeV36
                    | Profile::NativeV37
                    | Profile::NativeV38
                    | Profile::NativeV39
                    | Profile::NativeV40
                    | Profile::NativeV41
                    | Profile::NativeV42
                    | Profile::NativeV43
                    | Profile::NativeV44
                    | Profile::NativeV45
                    | Profile::NativeV46
                    | Profile::NativeV47
                    | Profile::NativeV48
                    | Profile::NativeV49
                    | Profile::NativeV50
                    | Profile::NativeV51
                    | Profile::NativeV52
                    | Profile::NativeV53
                    | Profile::NativeV54
                    | Profile::NativeV55
                    | Profile::NativeV56
                    | Profile::NativeV57
                    | Profile::NativeV58
                    | Profile::NativeV59
                    | Profile::NativeV60
                    | Profile::NativeV61
                    | Profile::NativeV62
                    | Profile::NativeV63
                    | Profile::NativeV64
                    | Profile::NativeV65
                    | Profile::NativeV66
                    | Profile::NativeV67
                    | Profile::NativeV68
                    | Profile::NativeV69
                    | Profile::NativeV70
                    | Profile::NativeV71
                    | Profile::NativeV72
                    | Profile::NativeV73
                    | Profile::NativeV74
                    | Profile::NativeV75
                    | Profile::NativeV76
                    | Profile::NativeV77
                    | Profile::NativeV78
                    | Profile::NativeV79
                    | Profile::NativeV80
                    | Profile::NativeV81
                    | Profile::NativeV82
            ) && grammar.terminals.contains_key(&name)
            {
                continue;
            }
            grammar.rules.entry(name).or_insert(rule);
        }
        for (name, terminal) in parent.terminals {
            grammar.terminals.entry(name).or_insert(terminal);
        }
        for (name, span) in parent.declaration_sources {
            grammar.declaration_sources.entry(name).or_insert(span);
        }
        for (name, spans) in parent.shadowed_declarations {
            grammar
                .shadowed_declarations
                .entry(name)
                .or_default()
                .extend(spans);
        }
        for name in parent.terminal_order {
            if !grammar.terminal_order.contains(&name) {
                grammar.terminal_order.push(name);
            }
        }
    } else if grammar.rules.values().any(|rule| rule.override_declared)
        || grammar
            .terminals
            .values()
            .any(|rule| rule.override_declared)
    {
        return Err(error(id, "@Override has no inherited parser declaration"));
    }
    if profile.has_contextual_first_rule() {
        // Keep authored declaration order and source-local classifications; an
        // inherited entry or shadowed map must not replace the first own rule.
        let syntax = super::syntax::parse(source, limits)?;
        let used = parent_identity
            .as_ref()
            .and_then(|id| state.admission.get(id))
            .copied()
            .unwrap_or_default();
        let summary = super::first_rule::validate_compiled(&syntax, &grammar, schema, used)?;
        state.admission.insert(id.clone(), summary);
    }
    state.active.remove(id);
    state.names.remove(&name);
    Ok(grammar)
}
