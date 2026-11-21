//! Ported language-neutral frozen loading. See PROVENANCE.json. No I/O.
use intersect_model::core::frozen::{
    FeatureId, FrozenError, FrozenStore, LinkView, Model as FrozenModel, ObjId, Severity,
    Validators, Violation,
};
use intersect_model::frozen_scopes::RcoreScopes;
use intersect_text::{Source, StoredEvaluation};
use serde::Serialize;
use std::{cell::RefCell, collections::BTreeMap};
/// What a load reports besides the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadPhase {
    /// Parsing and construction of one source.
    Parse,
    /// Name resolution of one cross reference, or a link violation.
    Link,
    /// Completion of the implicit elements.
    Implicit,
    /// Completion of the construction defaults.
    Defaults,
    /// Structural validation of the completed model.
    Validate,
    /// The language's validation rules (`bind validate`) over the objects
    /// this load added.
    Check,
}

/// One problem found by a load. `resource`, `start` and `end` locate it in a
/// source when it is known; `object` is the element identity it concerns.
///
/// `severity` is `Error` for every parse, link, completion and structural
/// problem; a validation rule (`LoadPhase::Check`) may also report warnings
/// and information, which do not make a load unclean (`is_error`). `code` is
/// the rule's issue code (for rules ported from the Pilot, the Pilot's code
/// verbatim), the `Violation` variant name for a structural problem
/// (`Multiplicity`, `TargetType`, ...), and none for parse and link problems.
/// `rule` identifies the rule where several share a code; `feature` names the
/// feature the problem concerns, when the rule names one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoadDiagnostic {
    pub phase: LoadPhase,
    pub severity: Severity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<usize>,
}

impl LoadDiagnostic {
    /// An error without a code or a location: a parse, link or completion
    /// problem, before the caller locates it.
    pub fn error(phase: LoadPhase, message: String) -> Self {
        Self {
            phase,
            severity: Severity::Error,
            code: None,
            rule: None,
            feature: None,
            message,
            resource: None,
            object: None,
            start: None,
            end: None,
        }
    }
    /// Whether this diagnostic makes a load unclean: every severity `Error`.
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// The result of [`load_frozen`]: the validated frozen model when every
/// phase passed, the diagnostics of every phase that ran, and the reference
/// counts of the link phase.
pub struct FrozenLoad {
    pub model: Option<FrozenModel>,
    pub diagnostics: Vec<LoadDiagnostic>,
    /// Cross references the linker asked the Rcore scopes to resolve.
    pub references: usize,
    /// Of those, the ones that resolved to exactly one element.
    pub resolved: usize,
    /// Exact authored occurrences and uniquely linked targets. Never inferred
    /// from an element's enclosing span or from a global name search.
    pub navigation: Vec<intersect_text::Reference>,
    pub issues: Vec<intersect_model::core::frozen::ObjectIssue>,
    pub proxies: usize,
    pub unresolved_occurrences: usize,
}

impl FrozenLoad {
    /// A model, no diagnostic of severity error and every reference
    /// resolved uniquely. Warnings and information do not count.
    pub fn is_clean(&self) -> bool {
        self.model.is_some()
            && !self.diagnostics.iter().any(LoadDiagnostic::is_error)
            && self.resolved == self.references
    }
}

/// Parse `sources` into `store` with `parse` (the host picks the bound
/// language of each source), then link the new cross references with
/// `resolvers`, complete implicit elements and defaults, and validate: the
/// whole-library load of `frozen_library_load` and the load of user files on
/// top of a prepared library (`store` from `FrozenStore::from_model`, whose
/// objects count as already linked and completed) alike. Parse diagnostics
/// do not stop the load; a failing link, completion or validation does.
///
/// After structural validation, the language's validation rules
/// (`bind validate` in `resolvers`) run over the objects this load added
/// (`check`); their issues are located at their objects, and only issues of
/// severity error make the load unclean (`FrozenLoad::is_clean`). The model
/// is returned whenever structural validation passed.
///
/// `observe` receives `parse_end`, `link_end`, `implicit_end`,
/// `defaults_end`, `validate_end` and `check_end` as each phase finishes, so
/// a host can time the phases (this library reads no clock). Unlike
/// `load_kerml`, structural diagnostics name the element they concern but are
/// not located in the source: locating them needs an export of the whole
/// store, which a library-sized load should not pay for.
pub fn load_frozen(
    mut store: FrozenStore,
    sources: &[Source],
    mut parse: impl FnMut(&Source, &mut FrozenStore) -> Result<(StoredEvaluation, usize, usize), String>,
    resolvers: &RcoreScopes<'_>,
    observe: &dyn Fn(&'static str),
) -> Result<FrozenLoad, String> {
    let metamodel = store.metamodel().clone();
    let mut diagnostics = Vec::new();
    let mut proxies = 0;
    let mut unresolved_occurrences = 0;
    let mut occurrences =
        BTreeMap::<(String, String, String), Vec<intersect_text::UnresolvedReference>>::new();
    for source in sources {
        match parse(source, &mut store) {
            Ok((evaluation, written, _)) => {
                proxies += written;
                unresolved_occurrences += evaluation.unresolved_references.len();
                for reference in evaluation.unresolved_references {
                    occurrences
                        .entry((
                            reference.owner.clone(),
                            reference.feature.clone(),
                            reference.spelling.clone(),
                        ))
                        .or_default()
                        .push(reference);
                }
                diagnostics.extend(evaluation.diagnostics.into_iter().map(|d| LoadDiagnostic {
                    resource: Some(source.uri.clone()),
                    start: Some(d.start),
                    end: Some(d.end),
                    ..LoadDiagnostic::error(LoadPhase::Parse, d.message)
                }))
            }
            Err(error) => diagnostics.push(LoadDiagnostic {
                resource: Some(source.uri.clone()),
                ..LoadDiagnostic::error(LoadPhase::Parse, error)
            }),
        }
    }
    observe("parse_end");
    let unlocated = |phase: LoadPhase, object: Option<&str>, message: String| LoadDiagnostic {
        object: object.map(str::to_owned),
        ..LoadDiagnostic::error(phase, message)
    };
    let violations = |phase: LoadPhase, found: &[Violation]| -> Vec<LoadDiagnostic> {
        found
            .iter()
            .map(|violation| LoadDiagnostic {
                code: Some(violation_code(violation).to_owned()),
                ..unlocated(phase, violation_object(violation), violation.to_string())
            })
            .collect()
    };
    let outcomes = RefCell::new(Vec::<LoadDiagnostic>::new());
    let counts = RefCell::new((0usize, 0usize));
    let navigation = RefCell::new(Vec::new());
    let resolve = |view: &LinkView<'_>,
                   owner: ObjId,
                   feature: FeatureId,
                   spelling: &str|
     -> Result<Option<ObjId>, FrozenError> {
        let found = resolvers.candidates(view, owner, feature, spelling);
        let mut counts = counts.borrow_mut();
        counts.0 += 1;
        let unique = match &found {
            Ok(candidates) if candidates.len() == 1 => candidates.first().copied(),
            _ => None,
        };
        if unique.is_some() {
            counts.1 += 1;
            if let Some(target) = unique.and_then(|id| view.identity(id)) {
                let key = (
                    view.identity(owner).unwrap_or("").to_owned(),
                    metamodel.feature_name(feature).to_owned(),
                    spelling.to_owned(),
                );
                for reference in occurrences.get(&key).into_iter().flatten() {
                    navigation.borrow_mut().push(intersect_text::Reference {
                        uri: reference.uri.clone(),
                        start: reference.start,
                        end: reference.end,
                        target: target.to_owned(),
                    });
                }
            }
        } else {
            let owner_id = owner;
            let owner = view.identity(owner).unwrap_or("");
            let feature = metamodel.feature_name(feature);
            let message = match &found {
                Ok(candidates) if candidates.is_empty() => {
                    format!("{owner}.{feature}: no element named '{spelling}'")
                }
                Ok(candidates) => format!(
                    "{owner}.{feature}: '{spelling}' is ambiguous ({} candidates)",
                    candidates.len()
                ),
                Err(error) => format!("{owner}.{feature}: '{spelling}': {error}"),
            };
            // Located at the referencing element: its resource and span.
            let mut diagnostic = unlocated(LoadPhase::Link, Some(owner), message);
            diagnostic.resource = view.resource(owner_id).map(str::to_owned);
            if let Some((start, end)) = view.span(owner_id).filter(|(s, e)| s < e) {
                diagnostic.start = Some(start);
                diagnostic.end = Some(end);
            }
            outcomes.borrow_mut().push(diagnostic);
        }
        Ok(unique)
    };
    let builder = store.into_builder().map_err(|error| error.to_string())?;
    let linked = builder.link(&resolve);
    observe("link_end");
    diagnostics.extend(outcomes.into_inner());
    let (references, resolved) = counts.into_inner();
    let mut navigation = navigation.into_inner();
    navigation.sort_by(|a, b| {
        (&a.uri, a.start, a.end, &a.target).cmp(&(&b.uri, b.start, b.end, &b.target))
    });
    navigation.dedup_by(|a, b| {
        a.uri == b.uri && a.start == b.start && a.end == b.end && a.target == b.target
    });
    let result = |model: Option<FrozenModel>, diagnostics: Vec<LoadDiagnostic>| FrozenLoad {
        model,
        diagnostics,
        references,
        resolved,
        navigation,
        issues: vec![],
        proxies,
        unresolved_occurrences,
    };
    let linked = match linked {
        Ok(linked) => linked,
        Err(found) => {
            diagnostics.extend(violations(LoadPhase::Link, &found.0));
            return Ok(result(None, diagnostics));
        }
    };
    let linked = linked.complete_implicit(resolvers);
    observe("implicit_end");
    let linked = match linked {
        Ok(linked) => linked,
        Err(found) => {
            diagnostics.extend(violations(LoadPhase::Implicit, &found.0));
            return Ok(result(None, diagnostics));
        }
    };
    let linked = linked.complete_defaults(resolvers);
    observe("defaults_end");
    let linked = match linked {
        Ok(linked) => linked,
        Err(found) => {
            diagnostics.extend(violations(LoadPhase::Defaults, &found.0));
            return Ok(result(None, diagnostics));
        }
    };
    let validated = linked.validate();
    observe("validate_end");
    let model = match validated {
        Ok(model) => model,
        Err(found) => {
            diagnostics.extend(violations(LoadPhase::Validate, &found.0));
            return Ok(result(None, diagnostics));
        }
    };
    let issues = model.check(resolvers);
    diagnostics.extend(issue_diagnostics(&model, &issues));
    observe("check_end");
    let mut loaded = result(Some(model), diagnostics);
    loaded.issues = issues;
    Ok(loaded)
}

/// Run the language's validation rules (`bind validate`) over the objects of
/// `model` that its load added (`Model::check`) and locate each issue at its
/// object: resource and source span. The model is not changed.
pub fn check(model: &FrozenModel, validators: &dyn Validators) -> Vec<LoadDiagnostic> {
    issue_diagnostics(model, &model.check(validators))
}
fn issue_diagnostics(
    model: &FrozenModel,
    issues: &[intersect_model::core::frozen::ObjectIssue],
) -> Vec<LoadDiagnostic> {
    issues
        .iter()
        .map(|found| {
            let object = model.obj(found.object);
            let span = object.map(|object| object.span()).filter(|(s, e)| s < e);
            LoadDiagnostic {
                phase: LoadPhase::Check,
                severity: found.issue.severity,
                code: Some(found.issue.code.clone()),
                rule: found.issue.rule.clone(),
                feature: found.issue.feature.clone(),
                message: found.issue.message.clone(),
                resource: object.map(|object| object.resource().to_owned()),
                object: object.map(|object| object.identity().to_owned()),
                start: span.map(|(start, _)| start),
                end: span.map(|(_, end)| end),
            }
        })
        .collect()
}

/// The stable code of a structural violation: its variant name.
pub fn violation_code(violation: &Violation) -> &'static str {
    match violation {
        Violation::UnknownClass(_) => "UnknownClass",
        Violation::MultipleContainers(_) => "MultipleContainers",
        Violation::ContainmentCycle(_) => "ContainmentCycle",
        Violation::Abstract { .. } => "Abstract",
        Violation::Multiplicity { .. } => "Multiplicity",
        Violation::Kind { .. } => "Kind",
        Violation::Attribute { .. } => "Attribute",
        Violation::TargetType { .. } => "TargetType",
        Violation::Duplicate { .. } => "Duplicate",
        Violation::Unresolved { .. } => "Unresolved",
        Violation::ReferenceCycle { .. } => "ReferenceCycle",
        Violation::Opposite { .. } => "Opposite",
        Violation::Resolver(_) => "Resolver",
    }
}

/// The element a violation concerns.
pub fn violation_object(violation: &Violation) -> Option<&str> {
    match violation {
        Violation::UnknownClass(object)
        | Violation::MultipleContainers(object)
        | Violation::ContainmentCycle(object)
        | Violation::Abstract { object, .. }
        | Violation::Multiplicity { object, .. }
        | Violation::Kind { object, .. }
        | Violation::Attribute { object, .. }
        | Violation::TargetType { object, .. }
        | Violation::Duplicate { object, .. }
        | Violation::Unresolved { object, .. }
        | Violation::ReferenceCycle { object, .. }
        | Violation::Opposite { object, .. } => Some(object),
        Violation::Resolver(_) => None,
    }
}

/// Identity of the shared loader implementation and its checked dependency context.
pub const SOURCE_IDENTITY: &str = env!("INTERSECT_LOAD_SOURCE_IDENTITY");
