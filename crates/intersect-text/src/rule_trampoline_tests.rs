//! Genuine compiled-source/durable-document controls; no Pilot/full24 award.
use super::*;
use crate::package::BundleCompilation;
use intersect_model::frozen::FrozenStore;

const ECORE: &str = r##"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="frames" nsURI="urn:dispatch-frames" nsPrefix="f">
<eClassifiers xsi:type="ecore:EClass" name="Root"><eStructuralFeatures xsi:type="ecore:EReference" name="child" eType="#//Node" containment="true"/></eClassifiers>
<eClassifiers xsi:type="ecore:EClass" name="Node"><eStructuralFeatures xsi:type="ecore:EAttribute" name="name" lowerBound="1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/><eStructuralFeatures xsi:type="ecore:EReference" name="child" eType="#//Node" containment="true"/></eClassifiers>
</ecore:EPackage>"##;

fn language(chain: usize, hidden_override: bool) -> Result<DocumentLanguage, String> {
    let mut source = String::from(
        "grammar intersect.tests.DirectDispatch hidden(WS)\nimport \"urn:dispatch-frames\" as f\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nRoot returns f::Root: 'root' child=E0;\n",
    );
    for index in 0..chain {
        let hidden = if hidden_override && index == chain / 2 {
            " hidden()"
        } else {
            ""
        };
        source.push_str(&format!(
            "E{index} returns f::Node{hidden}: E{};\n",
            index + 1
        ));
    }
    source.push_str(&format!("E{chain} returns f::Node: Atom;\n"));
    source.push_str("Atom returns f::Node: 'n' name=ID ('(' child=E0 ')')?;\nterminal ID returns ecore::EString: ('a'..'z')+;\nterminal WS returns ecore::EString: (' '|'\\t'|'\\r'|'\\n')+;\n");
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    // Retain the same migration and exact fixture texts, then enter through the
    // ordinary frontend which supplies fresh language-owned source validation.
    let report = intersect_model::migration::migrate_ecore(
        DefinitionSource::new("memory:oracle.ecore", SourceKind::Ecore, ECORE),
        "memory:fixture.rcore",
    )
    .map_err(|error| format!("Actual fixture migration: {error:?}"))?;
    let converted = report
        .converted
        .ok_or_else(|| format!("Actual fixture migration issues: {:?}", report.issues))?;
    let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, &converted.text);
    let namespace = intersect_model::definitions::rcore::declared_namespace(&model)
        .map_err(|errors| format!("Actual fixture namespace: {errors:?}"))?;
    let mut authored = DefinitionSource::new("memory:language.xtext", SourceKind::Xtext, &source);
    authored.dependencies.push(model.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package dispatchFrames version \"1\";",
    );
    behavior.dependencies.push(model.id.clone());
    let package = ordinary::compile(
        BundleCompilation {
            materialized_types: vec![],
            ecore: model.id.clone(),
            namespace,
            xtext: Some(authored.id.clone()),
            rcore_behavior: behavior.id.clone(),
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![model, authored, behavior],
                namespaces: vec![],
            },
        },
        grammar::Profile::NativeV67,
    )
    .map_err(|errors| format!("Actual dispatch grammar preparation: {errors:?}"))?;
    DocumentLanguage::prepare(
        &package,
        crate::package::CompilationSourcePolicy::default(),
        AnalysisLimits::workbench(),
    )
    .map_err(|errors| format!("Actual durable dispatch preparation: {errors:?}"))
}

fn nested(depth: usize) -> Source {
    let mut text = String::from("root ");
    for _ in 0..depth {
        text.push_str("n a(");
    }
    text.push_str("n z");
    text.extend(std::iter::repeat_n(')', depth));
    Source {
        uri: "memory:dispatch-stack".into(),
        text,
    }
}

// SAME parse_resources stage used by DocumentLanguage and DraftLanguage. A test
// export is only the before/after oracle; it is never an input or an authority.
type Parsed = (
    Result<(), String>,
    Model,
    Model,
    Vec<DocumentSnapshot>,
    usize,
);

fn parse(
    language: &DocumentLanguage,
    source: &Source,
    work_limit: usize,
    association_bytes: usize,
) -> Result<Parsed, String> {
    parse_with_requested_nesting(language, source, work_limit, association_bytes, 64)
}

fn parse_with_requested_nesting(
    language: &DocumentLanguage,
    source: &Source,
    work_limit: usize,
    association_bytes: usize,
    requested_nesting: usize,
) -> Result<Parsed, String> {
    let package = language.package();
    let grammar = package.grammar.as_ref().ok_or("Missing actual grammar")?;
    let mut store = FrozenStore::for_schema(&package.schema).unwrap();
    let before = storage::export(&store)?;
    let mut snapshots = Vec::new();
    let mut limits = EvaluationLimits::workbench();
    assert_eq!(limits.nesting, 64);
    assert!(requested_nesting <= limits.nesting);
    limits.nesting = requested_nesting;
    limits.steps_per_resource = Some(work_limit);
    let mut document_limits = language.limits;
    document_limits.association_bytes = association_bytes;
    let mut work = 0;
    let mut output = StoredEvaluation::default();
    let result = storage::atomic(&mut store, |store| {
        storage::bind_schema(store, &package.schema)?;
        store.configure_schema(&package.schema)?;
        let mut invocation = Invocation {
            limits,
            entry: None,
            values: None,
            documents: Some(Request::new(
                std::sync::Arc::clone(&language.origins),
                document_limits,
                &mut snapshots,
            )),
        };
        parse_resources(
            package,
            std::slice::from_ref(source),
            store,
            limits,
            &mut invocation,
            None,
            None,
            None,
            &grammar.entry,
            &mut output,
            Some(&mut work),
        )?;
        if !output.diagnostics.is_empty() {
            return Err(format!(
                "Actual syntax diagnostics: {:?}",
                output.diagnostics
            ));
        }
        Ok(())
    });
    let after = storage::export(&store)?;
    Ok((result, before, after, snapshots, work))
}

#[test]
fn direct_dispatch_durable_nested_sources_keep_real_objects_and_document_calls()
-> Result<(), String> {
    let language = language(24, false)?;
    let source = nested(3);
    let analysis = language.analyze(&source, None, EvaluationLimits::workbench())?;
    assert!(analysis.diagnostics.is_empty());
    let model = &analysis
        .valid_model
        .as_ref()
        .ok_or("No completed actual graph")?
        .model;
    assert_eq!(
        model.elements.iter().filter(|e| e.class == "Root").count(),
        1
    );
    assert_eq!(
        model.elements.iter().filter(|e| e.class == "Node").count(),
        4
    );
    assert!(model.elements.iter().all(|e| e.resource == source.uri));
    assert!(
        model
            .elements
            .iter()
            .all(|e| e.start <= e.end && e.end <= source.text.len())
    );
    assert_eq!(
        analysis.snapshot.document().source(),
        &intersect_model::definitions::bundle::SourceId::for_text(&source.uri, &source.text)
    );
    // All original source call occurrences are retained in the document; the
    // trampoline does not erase the grammar layer from source observations.
    let calls = analysis
        .snapshot
        .origins()
        .iter()
        .filter(|o| matches!(&o.kind, OriginKind::RuleCall))
        .count();
    assert!(
        calls >= 25,
        "actual source call catalog must retain every wrapper"
    );
    let nodes = analysis.snapshot.document().nodes();
    for wrapper in 0..=24 {
        let header = format!("E{wrapper} ");
        let mut actual_nodes = 0;
        for (index, node) in nodes.iter().enumerate() {
            let Some(origin) = node
                .data
                .origin
                .and_then(|at| analysis.snapshot.origins().get(at))
            else {
                continue;
            };
            if !matches!(&origin.kind, OriginKind::RuleCall) {
                continue;
            }
            let declaration = origin
                .declaration
                .as_ref()
                .ok_or("Call origin lost actual declaration")?;
            let original = original_source(language.package(), &declaration.source)?;
            let body = original
                .get(declaration.start..declaration.end)
                .ok_or("Bad actual rule source interval")?;
            if !body.starts_with(&header) {
                continue;
            }
            actual_nodes += 1;
            assert!(node.range.start < node.range.end && node.range.end <= source.text.len());
            let parent = nodes
                .get(node.parent.ok_or("Wrapper call lost actual parent")?.0)
                .ok_or("Missing wrapper parent")?;
            assert!(parent.range.start <= node.range.start && node.range.end <= parent.range.end);
            assert!(
                node.total_range.start <= node.range.start
                    && node.range.end <= node.total_range.end
            );
            let semantic = analysis
                .snapshot
                .semantic_object(NodeId(index))?
                .ok_or("Wrapper lost enclosing actual semantic owner")?;
            let identity = analysis
                .snapshot
                .semantic_objects()
                .get(semantic)
                .ok_or("Unknown actual semantic owner")?;
            let actual = model
                .elements
                .iter()
                .find(|row| row.id == identity.id)
                .ok_or("Document semantic owner missing from same real model")?;
            assert_eq!(actual.class, identity.class);
            assert_eq!(
                actual.type_identity,
                language.package().schema.identity(&identity.class)
            );
            assert_eq!(actual.resource, source.uri);
        }
        assert_eq!(
            actual_nodes, 4,
            "every E{wrapper} actual call node must occur once per Node parse"
        );
    }
    assert!(analysis.recovered.is_none());
    Ok(())
}

#[test]
fn direct_dispatch_exact_short_zero_and_storage_refusal_roll_back_same_store() -> Result<(), String>
{
    let language = language(8, false)?;
    let source = nested(2);
    let actual = parse(&language, &source, 1_000_000, 16_000_000)?;
    actual.0?;
    assert!(actual.4 > 0 && actual.4 < 1_000_000);
    let exact = parse(&language, &source, actual.4, 16_000_000)?;
    exact.0?;
    assert_eq!(json!(exact.2), json!(actual.2));
    assert_eq!(json!(exact.3), json!(actual.3));
    for limit in [0, actual.4 - 1] {
        let denied = parse(&language, &source, limit, 16_000_000)?;
        assert!(denied.0.is_err());
        assert_eq!(json!(denied.1), json!(denied.2));
    }
    let denied = parse(&language, &source, 1_000_000, 1)?;
    assert!(denied.0.is_err());
    assert_eq!(json!(denied.1), json!(denied.2));
    Ok(())
}

#[test]
fn direct_dispatch_actual_recursive_and_continuation_limits_remain_bounded() -> Result<(), String> {
    // One dispatch wrapper keeps real document calls below128 until the
    // nontransparent recursive rule actually reaches its unchanged64 limit.
    let language = language(0, false)?;
    let recursive = parse(&language, &nested(70), 10_000_000, 16_000_000)?;
    assert!(
        recursive
            .0
            .as_ref()
            .is_err_and(|e| e.contains("Model nesting budget exceeded"))
    );
    assert_eq!(json!(recursive.1), json!(recursive.2));
    // Preserve the actual source-admission frontier separately. This longer
    // grammar is refused before runtime and cannot prove a runtime limit.
    let source_refusal = self::language(65, false)
        .err()
        .ok_or("Long original alias fixture unexpectedly passed source admission")?;
    assert!(
        source_refusal.contains("Grammar prediction budget exceeded"),
        "{source_refusal}"
    );
    // This genuinely admitted grammar first completes under the unchanged64
    // default. Only then request a smaller8 continuation bound: its12 direct
    // wrappers are explicit suspended frames while physical recursion is shallow.
    let compact = self::language(12, false)?;
    let source = nested(0);
    let admitted = parse(&compact, &source, 10_000_000, 16_000_000)?;
    admitted.0?;
    assert_eq!(admitted.2.elements.len(), 2);
    let continuation = parse_with_requested_nesting(&compact, &source, 10_000_000, 16_000_000, 8)?;
    assert!(
        continuation
            .0
            .as_ref()
            .is_err_and(|e| e.contains("Model nesting budget exceeded"))
    );
    assert_eq!(json!(continuation.1), json!(continuation.2));
    Ok(())
}

#[test]
fn direct_dispatch_explicit_hidden_override_and_syntax_recovery_stay_real() -> Result<(), String> {
    let language = language(4, true)?;
    let source = nested(0); // whitespace is real retained WS, not prefiltered
    let analysis = language.analyze(&source, None, EvaluationLimits::workbench())?;
    assert!(analysis.valid_model.is_none());
    assert!(analysis.recovered.is_some());
    assert!(!analysis.diagnostics.is_empty());
    assert_eq!(
        analysis.snapshot.document().source(),
        &intersect_model::definitions::bundle::SourceId::for_text(&source.uri, &source.text)
    );
    Ok(())
}

// Separate genuine authored language: Expr has no required scalar default;
// Atom owns its name, while suffix Capture constructs Wrap and retains Atom.
const SEQUENCE_ECORE: &str = r##"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="sequenceFrames" nsURI="urn:sequence-frames" nsPrefix="f">
<eClassifiers xsi:type="ecore:EClass" name="Root"><eStructuralFeatures xsi:type="ecore:EReference" name="value" eType="#//Expr" containment="true"/></eClassifiers>
<eClassifiers xsi:type="ecore:EClass" name="Expr" abstract="true"/>
<eClassifiers xsi:type="ecore:EClass" name="Atom" eSuperTypes="#//Expr"><eStructuralFeatures xsi:type="ecore:EAttribute" name="name" lowerBound="1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/><eStructuralFeatures xsi:type="ecore:EReference" name="child" eType="#//Expr" containment="true"/></eClassifiers>
<eClassifiers xsi:type="ecore:EClass" name="Wrap" eSuperTypes="#//Expr"><eStructuralFeatures xsi:type="ecore:EReference" name="left" lowerBound="1" eType="#//Expr" containment="true"/><eStructuralFeatures xsi:type="ecore:EAttribute" name="tail" lowerBound="1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/></eClassifiers>
</ecore:EPackage>"##;

fn sequence_language(chain: usize, hidden: bool) -> Result<DocumentLanguage, String> {
    let mut source = String::from(
        "grammar intersect.tests.SequenceDispatch hidden(WS)\nimport \"urn:sequence-frames\" as f\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nRoot returns f::Root: 'root' value=E0;\n",
    );
    if hidden {
        source.push_str("E0 returns f::Expr: Child 'end';\nChild returns f::Expr hidden(): Bracket;\nBracket returns f::Atom: '[' name=ID ']';\n");
    } else {
        for index in 0..chain {
            source.push_str(&format!(
                "E{index} returns f::Expr: E{} ('e{index}')?;\n",
                index + 1
            ));
        }
        source.push_str(&format!("E{chain} returns f::Expr: Base ('final')?;\n"));
        source.push_str("Base returns f::Expr: Atom ({f::Wrap.left=current} '+' tail=ID)?;\nAtom returns f::Atom: 'n' name=ID ('(' child=E0 ')')?;\n");
    }
    source.push_str("terminal ID returns ecore::EString: ('a'..'z')+;\nterminal WS returns ecore::EString: (' '|'\\t'|'\\r'|'\\n')+;\n");
    use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
    // Retain the same migration and exact fixture texts, then enter through the
    // ordinary frontend which supplies fresh language-owned source validation.
    let report = intersect_model::migration::migrate_ecore(
        DefinitionSource::new("memory:oracle.ecore", SourceKind::Ecore, SEQUENCE_ECORE),
        "memory:fixture.rcore",
    )
    .map_err(|error| format!("Actual fixture migration: {error:?}"))?;
    let converted = report
        .converted
        .ok_or_else(|| format!("Actual fixture migration issues: {:?}", report.issues))?;
    let model = DefinitionSource::new("memory:model.rcore", SourceKind::Rcore, &converted.text);
    let namespace = intersect_model::definitions::rcore::declared_namespace(&model)
        .map_err(|errors| format!("Actual fixture namespace: {errors:?}"))?;
    let mut authored = DefinitionSource::new("memory:language.xtext", SourceKind::Xtext, &source);
    authored.dependencies.push(model.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package dispatchFrames version \"1\";",
    );
    behavior.dependencies.push(model.id.clone());
    let package = ordinary::compile(
        BundleCompilation {
            materialized_types: vec![],
            ecore: model.id.clone(),
            namespace,
            xtext: Some(authored.id.clone()),
            rcore_behavior: behavior.id.clone(),
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![model, authored, behavior],
                namespaces: vec![],
            },
        },
        grammar::Profile::NativeV67,
    )
    .map_err(|errors| format!("Actual dispatch grammar preparation: {errors:?}"))?;
    DocumentLanguage::prepare(
        &package,
        crate::package::CompilationSourcePolicy::default(),
        AnalysisLimits::workbench(),
    )
    .map_err(|errors| format!("Actual durable dispatch preparation: {errors:?}"))
}

fn sequence_source() -> Source {
    Source {
        uri: "memory:sequence-stack".into(),
        text: "root n a(n b(n c(n d+inner)+third)+second)+outer".into(),
    }
}

#[test]
fn leading_sequence_suffix_capture_keeps_actual_owner_spans_and_document_calls()
-> Result<(), String> {
    let language = sequence_language(16, false)?;
    let source = sequence_source();
    let analysis = language.analyze(&source, None, EvaluationLimits::workbench())?;
    assert!(analysis.diagnostics.is_empty());
    assert!(analysis.recovered.is_none());
    let model = &analysis
        .valid_model
        .as_ref()
        .ok_or("No genuine Sequence graph")?
        .model;
    assert_eq!(model.elements.len(), 9);
    let root = model
        .elements
        .iter()
        .find(|row| row.class == "Root")
        .ok_or("Missing actual Root")?;
    let outer_id = root
        .values
        .get("value")
        .and_then(Value::as_str)
        .ok_or("Root lost actual final captured owner")?;
    let outer = model
        .elements
        .iter()
        .find(|row| row.id == outer_id)
        .ok_or("Missing captured root value")?;
    assert_eq!(outer.class, "Wrap");
    assert_eq!(outer.values.get("tail"), Some(&json!("outer")));
    assert_eq!(
        outer.start,
        source.text.find("n a").ok_or("Missing input interval")?
    );
    assert_eq!(outer.end, source.text.len());
    for (tail, name) in [
        ("outer", "a"),
        ("second", "b"),
        ("third", "c"),
        ("inner", "d"),
    ] {
        let wrapper = model
            .elements
            .iter()
            .find(|row| row.class == "Wrap" && row.values.get("tail") == Some(&json!(tail)))
            .ok_or("Missing actual suffix Capture")?;
        let left = wrapper
            .values
            .get("left")
            .and_then(Value::as_str)
            .ok_or("Capture lost actual previous owner")?;
        let atom = model
            .elements
            .iter()
            .find(|row| row.id == left)
            .ok_or("Captured Atom is outside same model")?;
        assert_eq!(atom.class, "Atom");
        assert_eq!(atom.values.get("name"), Some(&json!(name)));
        assert_eq!(atom.resource, source.uri);
        assert_eq!(wrapper.resource, source.uri);
        assert_eq!(atom.start, wrapper.start);
        assert!(atom.end < wrapper.end && wrapper.end <= source.text.len());
        assert_eq!(
            wrapper.type_identity,
            language.package().schema.identity("Wrap")
        );
    }
    let document = analysis.snapshot.document();
    assert_eq!(
        document.source(),
        &intersect_model::definitions::bundle::SourceId::for_text(&source.uri, &source.text)
    );
    for wrapper in 0..=16 {
        let header = format!("E{wrapper} ");
        let mut actual = 0;
        for (index, node) in document.nodes().iter().enumerate() {
            let Some(origin) = node
                .data
                .origin
                .and_then(|at| analysis.snapshot.origins().get(at))
            else {
                continue;
            };
            if !matches!(&origin.kind, OriginKind::RuleCall) {
                continue;
            }
            let declaration = origin
                .declaration
                .as_ref()
                .ok_or("Missing actual Sequence call declaration")?;
            let original = original_source(language.package(), &declaration.source)?;
            if !original
                .get(declaration.start..declaration.end)
                .ok_or("Invalid actual call range")?
                .starts_with(&header)
            {
                continue;
            }
            actual += 1;
            let parent = document
                .nodes()
                .get(node.parent.ok_or("Missing actual Sequence call parent")?.0)
                .ok_or("Unknown actual call parent")?;
            assert!(parent.range.start <= node.range.start && node.range.end <= parent.range.end);
            assert!(node.range.start < node.range.end && node.range.end <= source.text.len());
            let semantic = analysis
                .snapshot
                .semantic_object(NodeId(index))?
                .ok_or("Sequence call lost actual semantic owner")?;
            let semantic = analysis
                .snapshot
                .semantic_objects()
                .get(semantic)
                .ok_or("Unknown actual semantic object")?;
            assert!(model.elements.iter().any(|row| row.id == semantic.id
                && row.class == semantic.class
                && row.resource == source.uri));
        }
        assert_eq!(
            actual, 4,
            "E{wrapper} must retain its actual call node for every expression"
        );
    }
    Ok(())
}

#[test]
fn leading_sequence_suffix_exact_short_zero_and_document_storage_rollback() -> Result<(), String> {
    let language = sequence_language(6, false)?;
    let source = sequence_source();
    let actual = parse(&language, &source, 2_000_000, 16_000_000)?;
    actual.0?;
    assert!(actual.4 > 1 && actual.4 < 2_000_000);
    assert_eq!(actual.2.elements.len(), 9);
    let exact = parse(&language, &source, actual.4, 16_000_000)?;
    exact.0?;
    assert_eq!(json!(actual.2), json!(exact.2));
    assert_eq!(json!(actual.3), json!(exact.3));
    for limit in [0, actual.4 - 1] {
        let denied = parse(&language, &source, limit, 16_000_000)?;
        assert!(denied.0.is_err());
        assert_eq!(json!(denied.1), json!(denied.2));
    }
    let denied = parse(&language, &source, 2_000_000, 1)?;
    assert!(denied.0.is_err());
    assert_eq!(json!(denied.1), json!(denied.2));
    let invalid = Source {
        uri: source.uri.clone(),
        text: "root n a(n b+inner)+".into(),
    };
    let rejected = parse(&language, &invalid, 2_000_000, 16_000_000)?;
    assert!(rejected.0.is_err());
    assert_eq!(json!(rejected.1), json!(rejected.2));
    Ok(())
}

#[test]
fn leading_sequence_forwarding_suffix_restores_hidden_and_real_recovery() -> Result<(), String> {
    let language = sequence_language(0, true)?;
    let source = Source {
        uri: "memory:sequence-hidden".into(),
        text: "root [a] end".into(),
    };
    let valid = language.analyze(&source, None, EvaluationLimits::workbench())?;
    assert!(valid.diagnostics.is_empty());
    let model = &valid
        .valid_model
        .as_ref()
        .ok_or("Hidden restoration lost actual graph")?
        .model;
    assert_eq!(model.elements.len(), 2);
    let root = model
        .elements
        .iter()
        .find(|row| row.class == "Root")
        .ok_or("Missing genuine Root")?;
    let id = root
        .values
        .get("value")
        .and_then(Value::as_str)
        .ok_or("Forwarding Sequence returned Null instead of child owner")?;
    let child = model
        .elements
        .iter()
        .find(|row| row.id == id)
        .ok_or("Forwarded child missing")?;
    assert_eq!(child.class, "Atom");
    assert_eq!(child.values.get("name"), Some(&json!("a")));
    assert_eq!(child.start, 5);
    assert_eq!(child.end, 8); // suffix 'end' must not widen a delegated object's span
    for text in ["root [ a] end", "root [a] missing"] {
        let invalid = Source {
            uri: source.uri.clone(),
            text: text.into(),
        };
        let recovered = language.analyze(&invalid, None, EvaluationLimits::workbench())?;
        assert!(recovered.valid_model.is_none());
        assert!(recovered.recovered.is_some());
        assert!(!recovered.diagnostics.is_empty());
        assert_eq!(
            recovered.snapshot.document().source(),
            &intersect_model::definitions::bundle::SourceId::for_text(&invalid.uri, &invalid.text)
        );
    }
    Ok(())
}

#[test]
fn leading_sequence_real_recursive_and_continuation_depth_stay_bounded() -> Result<(), String> {
    let language = sequence_language(10, false)?;
    let compact = Source {
        uri: "memory:sequence-bounds".into(),
        text: "root n a+outer".into(),
    };
    let allowed = parse(&language, &compact, 2_000_000, 16_000_000)?;
    allowed.0?;
    assert_eq!(allowed.2.elements.len(), 3);
    let bounded = parse_with_requested_nesting(&language, &compact, 2_000_000, 16_000_000, 8)?;
    assert!(
        bounded
            .0
            .as_ref()
            .is_err_and(|error| error.contains("Model nesting budget exceeded"))
    );
    assert_eq!(json!(bounded.1), json!(bounded.2));
    let shallow = sequence_language(0, false)?;
    let mut text = String::from("root ");
    for _ in 0..70 {
        text.push_str("n a(");
    }
    text.push_str("n z");
    text.extend(std::iter::repeat_n(')', 70));
    let recursive = Source {
        uri: compact.uri,
        text,
    };
    // This grammar retains three real document Call envelopes per recursive
    // level. Request16 so the physical construction bound is reached before
    // the independent128 document-call bound. The retained direct control
    // above still proves genuine recursive refusal at the unchanged64 default.
    let denied = parse_with_requested_nesting(&shallow, &recursive, 10_000_000, 16_000_000, 16)?;
    assert!(
        denied
            .0
            .as_ref()
            .is_err_and(|error| error.contains("Model nesting budget exceeded"))
    );
    assert_eq!(json!(denied.1), json!(denied.2));
    Ok(())
}
