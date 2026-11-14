//! Generated canonical Rcore ownership boundary. Structural validation remains
//! in the shared compiler; this writer supplies no second model implementation.
use super::*;
use intersect_model::definitions::bundle::NamespaceBinding;
use std::ops::Range;

pub(super) struct Generated {
    pub sources: Vec<DefinitionSource>,
    pub bundle: DefinitionBundle,
    pub origins: Vec<GeneratedOrigin>,
    pub selection: Option<(SourceId, String)>,
}
fn quoted(value: &str) -> String {
    serde_json::Value::String(value.into()).to_string()
}
fn identifier(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}
fn ty(id: &TypeId) -> String {
    format!("external {}", quoted(&id.identity()))
}
pub(super) fn sources(engine: &Engine<'_>) -> Result<Generated> {
    let mut sources = vec![];
    let mut origins = vec![];
    for (index, package) in engine.packages.iter().enumerate() {
        let mut text = format!(
            "// Generated Xtext metamodel from SHA-256 {}.\nrcore 1;\npackage {} version \"1\";\nmetamodel {} prefix {} {{\n",
            engine.source.id.sha256,
            identifier(&package.name),
            quoted(&package.namespace),
            identifier(&package.name)
        );
        let mut ranges: Vec<(Range<usize>, Vec<SourceSpan>)> = vec![];
        for classifier in &package.classifiers {
            let start = text.len();
            if classifier.kind == ClassifierKind::Enumeration {
                text.push_str(&format!("  enum {} {{\n", identifier(&classifier.id.name)));
                for literal in &classifier.literals {
                    let start = text.len();
                    text.push_str(&format!(
                        "    {} = {} literal {};\n",
                        identifier(&literal.name),
                        literal.value,
                        quoted(&literal.literal)
                    ));
                    ranges.push((
                        start..text.len(),
                        literal.origins.iter().map(|o| o.span.clone()).collect(),
                    ));
                }
            } else {
                text.push_str(&format!("  class {}", identifier(&classifier.id.name)));
                if !classifier.parents.is_empty() {
                    text.push_str(" extends ");
                    text.push_str(
                        &classifier
                            .parents
                            .iter()
                            .map(ty)
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                }
                text.push_str(" {\n");
                for feature in &classifier.features {
                    let start = text.len();
                    text.push_str(&format!(
                        "    {} {}: {} [0,{}]{}{};\n",
                        if feature.reference { "ref" } else { "attr" },
                        identifier(&feature.name),
                        ty(&feature.target),
                        if feature.many { "*" } else { "1" },
                        if feature.unique { "" } else { " nonunique" },
                        if feature.containment {
                            " containment"
                        } else {
                            ""
                        }
                    ));
                    ranges.push((
                        start..text.len(),
                        feature.origins.iter().map(|o| o.span.clone()).collect(),
                    ));
                }
            }
            text.push_str("  }\n");
            ranges.push((start..text.len(), classifier.declarations.clone()));
        }
        text.push_str("}\n");
        let uri = format!("intersect-inferred:{}:{index}.rcore", engine.context);
        if engine.bundle.sources.iter().any(|s| s.id.uri == uri) {
            return Err(engine.error("Generated source URI collides with an input source"));
        }
        let source = DefinitionSource::new(uri, SourceKind::Rcore, text);
        for (range, original) in ranges {
            origins.push(GeneratedOrigin {
                generated: SourceSpan {
                    source: source.id.clone(),
                    start: range.start,
                    end: range.end,
                },
                original,
            });
        }
        sources.push(source);
    }
    let ids = sources.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
    for source in &mut sources {
        source.dependencies.push(engine.source.id.clone());
        if let Some(parent) = engine.inherited {
            source.dependencies.extend(
                parent
                    .grammars()
                    .iter()
                    .flat_map(|g| g.generated_sources.iter().map(|s| s.id.clone())),
            );
        }
        source
            .dependencies
            .extend(engine.source.dependencies.iter().cloned());
        source
            .dependencies
            .extend(ids.iter().filter(|id| *id != &source.id).cloned());
        source.dependencies.sort();
        source.dependencies.dedup();
    }
    let mut bundle = engine.bundle.clone();
    for (package, source) in engine.packages.iter().zip(&sources) {
        if bundle
            .namespaces
            .iter()
            .any(|binding| binding.namespace == package.namespace)
        {
            return Err(engine.error("Generated namespace already has an input binding"));
        }
        bundle.namespaces.push(NamespaceBinding {
            namespace: package.namespace.clone(),
            source: source.id.clone(),
        });
    }
    bundle.sources.extend(sources.iter().cloned());
    bundle.validate().map_err(|e| engine.error(e))?;
    let selection = if sources.is_empty() {
        engine
            .inherited
            .and_then(PreparedInference::model)
            .map(|p| (p.entry_source.clone(), p.namespace.clone()))
    } else {
        let index = engine
            .packages
            .iter()
            .position(|p| p.alias.is_none())
            .unwrap_or(0);
        Some((
            sources[index].id.clone(),
            engine.packages[index].namespace.clone(),
        ))
    };
    Ok(Generated {
        sources,
        bundle,
        origins,
        selection,
    })
}
pub(super) fn diagnostic(
    engine: &Engine<'_>,
    origins: &[GeneratedOrigin],
    error: &intersect_model::definitions::ecore::ImportError,
) -> Diagnostic {
    let origin = origins
        .iter()
        .filter(|origin| {
            origin.generated.source == error.span.source
                && origin.generated.start <= error.span.start
                && error.span.start < origin.generated.end
        })
        .min_by_key(|origin| origin.generated.end - origin.generated.start)
        .and_then(|origin| origin.original.first());
    let diagnostic = if let Some(origin) = origin {
        Diagnostic::new(
            &origin.source.uri,
            origin.start,
            origin.end,
            format!(
                "{} (generated source {})",
                error.message, error.span.source.uri
            ),
        )
    } else {
        Diagnostic::new(
            &engine.source.id.uri,
            engine.syntax.start,
            engine.syntax.end,
            error.message.clone(),
        )
    };
    let complete = engine
        .inherited
        .into_iter()
        .flat_map(|p| p.grammars())
        .flat_map(|g| g.generated_origins.iter())
        .chain(origins.iter())
        .collect::<Vec<_>>();
    diagnostic.with_generated(error.span.clone(), originals(&complete, &error.span))
}
fn originals(origins: &[&GeneratedOrigin], generated: &SourceSpan) -> Vec<SourceSpan> {
    let matches = origins
        .iter()
        .filter(|o| {
            o.generated.source == generated.source
                && o.generated.start <= generated.start
                && generated.start < o.generated.end
                && generated.end <= o.generated.end
        })
        .collect::<Vec<_>>();
    let Some(size) = matches
        .iter()
        .map(|o| o.generated.end - o.generated.start)
        .min()
    else {
        return vec![];
    };
    let mut result = Vec::new();
    for origin in matches
        .into_iter()
        .filter(|o| o.generated.end - o.generated.start == size)
    {
        for original in &origin.original {
            if !result.contains(original) {
                result.push(original.clone());
            }
        }
    }
    result
}
pub(super) fn project_diagnostic(error: Diagnostic, origins: &[GeneratedOrigin]) -> Diagnostic {
    let Some(DiagnosticOrigin::Source { source, span }) = error.origin.as_deref() else {
        return error;
    };
    if !origins.iter().any(|o| o.generated.source == *source) {
        return error;
    }
    let generated = span.clone().unwrap_or_else(|| SourceSpan {
        source: source.clone(),
        start: error.start,
        end: error.end,
    });
    let selected = originals(&origins.iter().collect::<Vec<_>>(), &generated);
    error.with_generated(generated, selected)
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    fn span(source: &DefinitionSource, start: usize, end: usize) -> SourceSpan {
        SourceSpan {
            source: source.id.clone(),
            start,
            end,
        }
    }
    #[test]
    fn generated_mapping_preserves_all_narrowest_exact_revision_correspondences() {
        let first = DefinitionSource::new("memory:shared.xtext", SourceKind::Xtext, "first body");
        let second = DefinitionSource::new("memory:shared.xtext", SourceKind::Xtext, "second body");
        let generated = DefinitionSource::new(
            "memory:generated.rcore",
            SourceKind::Rcore,
            "generated body",
        );
        let other =
            DefinitionSource::new(&generated.id.uri, SourceKind::Rcore, "other generated body");
        let a = span(&first, 0, 4);
        let b = span(&second, 2, 6);
        let c = span(&first, 6, 9);
        let mappings = vec![
            GeneratedOrigin {
                generated: span(&generated, 0, 14),
                original: vec![c.clone()],
            },
            GeneratedOrigin {
                generated: span(&generated, 2, 8),
                original: vec![a.clone(), b.clone()],
            },
            GeneratedOrigin {
                generated: span(&generated, 2, 8),
                original: vec![b.clone(), c.clone()],
            },
            GeneratedOrigin {
                generated: span(&other, 3, 4),
                original: vec![span(&second, 0, 1)],
            },
        ];
        let legacy = Diagnostic::new(&generated.id.uri, 3, 4, "failure");
        let projected = project_diagnostic(legacy.clone().with_source(&generated.id), &mappings);
        assert_eq!(projected, legacy);
        assert_eq!(
            projected.origin.as_deref(),
            Some(&DiagnosticOrigin::Generated {
                generated: span(&generated, 3, 4),
                originals: vec![a, b, c]
            })
        );
        // A diagnostic crossing narrower scopes must retain its complete scope.
        let crossing = Diagnostic::from_span(span(&generated, 3, 11), "crossing");
        assert_eq!(
            project_diagnostic(crossing, &mappings).origin.as_deref(),
            Some(&DiagnosticOrigin::Generated {
                generated: span(&generated, 3, 11),
                originals: vec![span(&first, 6, 9)]
            })
        );
        // A legacy URI cannot be upgraded by matching an unrelated revision.
        assert_eq!(project_diagnostic(legacy.clone(), &mappings).origin, None);
        let unrecognized = DefinitionSource::new(
            &generated.id.uri,
            SourceKind::Rcore,
            "unrecognized revision",
        );
        let input = legacy.with_source(&unrecognized.id);
        assert_eq!(
            project_diagnostic(input.clone(), &mappings).origin,
            input.origin
        );
    }
}
