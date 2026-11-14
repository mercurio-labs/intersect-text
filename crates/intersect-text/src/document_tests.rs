//! Private V65 controls. All authored source graphs are frozen originals or the
//! independently qualified namespace-only grouped companion; old inputs stay exact.
use super::*;
use crate::package::BundleCompilation;
use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
fn cases() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!("document_tests/cases.json")).unwrap()["cases"]
        .as_array()
        .unwrap()
        .clone()
}
fn inputs(c: &Value) -> BundleCompilation {
    serde_json::from_value(c["compilation"].clone()).unwrap()
}
fn walk(expr: &Expr, source: &str, count: &mut usize) {
    let range = expr
        .original_range
        .as_ref()
        .expect("same-parser V65 origin");
    assert!(
        range.start <= range.end && range.end <= source.len(),
        "{range:?}"
    );
    assert!(source.is_char_boundary(range.start) && source.is_char_boundary(range.end));
    *count += 1;
    match &expr.kind {
        Kind::Sequence(v) | Kind::Alternative(v) => {
            for e in v {
                walk(e, source, count)
            }
        }
        Kind::Assign { value, .. } | Kind::BooleanAssign { value, .. } => {
            walk(value, source, count)
        }
        _ => {}
    }
}
#[test]
fn exact_authored_origins_and_legacy_absence() {
    for c in cases()
        .into_iter()
        .filter(|c| c["id"] != "grouped_objects_original")
    {
        let p = ordinary::compile(inputs(&c), grammar::Profile::NativeV65)
            .unwrap_or_else(|e| panic!("{} {e:?}", c["id"]));
        let g = p.grammar.as_ref().unwrap();
        let b = &p.compilation.as_ref().unwrap().bundle;
        let mut count = 0;
        for (name, rule) in &g.rules {
            let declaration = g.declaration_sources.get(name).unwrap();
            let source = b
                .sources
                .iter()
                .find(|s| s.id == declaration.source)
                .unwrap();
            walk(&rule.body, &source.text, &mut count);
        }
        assert!(count > 0);
        assert_eq!(json!(crate::recompile(&p).unwrap()), json!(p));
        if c["id"] != "grouped_objects_namespace_companion" {
            let old = ordinary::compile(inputs(&c), grammar::Profile::NativeV64).unwrap();
            assert!(!json!(old).to_string().contains("original_range"));
        }
    }
}
#[test]
fn invalid_original_is_not_repaired_by_imported_hierarchy() {
    let cases = cases();
    let original = cases
        .iter()
        .find(|c| c["id"] == "grouped_objects_original")
        .unwrap();
    let e = ordinary::compile(inputs(original), grammar::Profile::NativeV65).unwrap_err();
    assert!(e.iter().any(|e| e.message.contains("upper case")), "{e:?}");
    let companion = cases
        .iter()
        .find(|c| c["id"] == "grouped_objects_namespace_companion")
        .unwrap();
    assert!(
        ordinary::compile(inputs(companion), grammar::Profile::NativeV64)
            .unwrap_err()
            .iter()
            .any(|e| e.message.contains("Missing inferred hierarchy"))
    );
    ordinary::compile(inputs(companion), grammar::Profile::NativeV65).unwrap();
}
#[test]
fn recovery_follows_only_actual_live_return_site() {
    let c = cases().remove(0);
    let mut input = inputs(&c);
    let old = input.xtext.clone().unwrap();
    let at = input
        .bundle
        .sources
        .iter()
        .position(|s| s.id == old)
        .unwrap();
    let mut src = DefinitionSource::new(
        "memory:document-mechanics/ActualReturn.xtext",
        SourceKind::Xtext,
        "grammar document.mechanics.ActualReturn\nimport \"urn:intersect:xtext:mini-kerml:terminal-spellings:1\" as k\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nModel returns k::Model: name=Inner 'm';\nOther returns k::Model: name=Inner 'o';\nInner returns ecore::EString: 'a';\n",
    );
    src.dependencies = input.bundle.sources[at].dependencies.clone();
    input.xtext = Some(src.id.clone());
    input.bundle.sources[at] = src;
    let p = ordinary::compile(input, grammar::Profile::NativeV65).unwrap();
    let program = p
        .grammar
        .as_ref()
        .unwrap()
        .parser_prediction
        .as_ref()
        .unwrap();
    let mut work = Some(1000);
    let m = program
        .recovery_symbols_with_work("rule/Inner", &["rule/Model/s0/value"], &mut work)
        .unwrap();
    assert_eq!(
        m,
        BTreeSet::from([grammar::PredictionSymbol::Keyword("m".into())])
    );
    let o = program
        .recovery_symbols_with_work("rule/Inner", &["rule/Other/s0/value"], &mut work)
        .unwrap();
    assert_eq!(
        o,
        BTreeSet::from([grammar::PredictionSymbol::Keyword("o".into())])
    );
    assert_eq!(
        program
            .recovery_symbols_with_work("rule/Inner", &[], &mut work)
            .unwrap(),
        BTreeSet::from([grammar::PredictionSymbol::End])
    );
    assert!(
        program
            .recovery_symbols_with_work("rule/Inner", &["rule/Model/s0/value"], &mut Some(0))
            .unwrap_err()
            .contains("budget")
    );
}

#[test]
fn same_machine_durable_positive_nodes_and_complete_models() {
    let cases = cases();
    let data: Value =
        serde_json::from_str(include_str!("document_tests/positive_nodes.json")).unwrap();
    for c in cases.into_iter().filter(|c| {
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == c["id"])
    }) {
        let package = ordinary::compile(inputs(&c), grammar::Profile::NativeV65).unwrap();
        let language = document::DocumentLanguage::prepare(
            &package,
            Default::default(),
            document::AnalysisLimits::workbench(),
        )
        .unwrap();
        for row in data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == c["id"])
        {
            let source = Source {
                uri: "memory:node-analysis/input".into(),
                text: row["text"].as_str().unwrap().into(),
            };
            let analysis = language
                .analyze(&source, None, EvaluationLimits::native())
                .unwrap_or_else(|e| panic!("{} {} {e}", c["id"], row["text"]));
            assert!(
                analysis.recovered.is_none(),
                "{} {} {:?}",
                c["id"],
                row["text"],
                analysis.diagnostics
            );
            let model = analysis.valid_model.as_ref().unwrap();
            assert_eq!(
                json!(model),
                json!(evaluate(&package, std::slice::from_ref(&source))),
                "{} {}",
                c["id"],
                row["text"]
            );
            let snapshot = &analysis.snapshot;
            let leaves:Vec<_>=snapshot.document.nodes().iter().filter(|n|n.kind==intersect_model::core::text_document::NodeKind::Leaf).map(|n|json!({"text":document_text(&snapshot.document,n.total_range),"hidden":n.data.hidden,"utf16_start":n.range.start,"utf16_end":n.range.end,"syntax_error":null})).collect();
            assert_eq!(
                json!(leaves),
                row["resource"]["all_root_leaves"],
                "{} {}",
                c["id"],
                row["text"]
            );
            snapshot
                .document
                .check_resource(&model.model.resources[&source.uri])
                .unwrap();
            let mut stale = model.model.resources[&source.uri].clone();
            stale.content_revision += 1;
            assert!(snapshot.document.check_resource(&stale).is_err());
            let expected = row["nodes"]["nodes"].as_array().unwrap();
            assert_eq!(
                snapshot.document.nodes().len(),
                expected.len(),
                "{} {} native {} vs upstream {}",
                c["id"],
                row["text"],
                json!(snapshot),
                row["nodes"]
            );
            for (i, (node, expected)) in snapshot.document.nodes().iter().zip(expected).enumerate()
            {
                assert_eq!(
                    json!(node.parent),
                    expected["parent"],
                    "{} {} node{i}",
                    c["id"],
                    row["text"]
                );
                assert_eq!(
                    node.range.start,
                    expected["utf16_start"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    node.range.end,
                    expected["utf16_end"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    node.total_range.start,
                    expected["total_utf16_start"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    node.total_range.end,
                    expected["total_utf16_end"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    json!(semantic_ordinal(
                        language.package(),
                        &analysis,
                        intersect_model::core::text_document::NodeId(i)
                    )),
                    expected["semantic_object"]
                );
            }
        }
    }
}

#[test]
fn original_nested_error_nodes_and_quarantined_partial_models() {
    let c = cases()
        .into_iter()
        .find(|c| c["id"] == "namespace_nested_restore")
        .unwrap();
    let package = ordinary::compile(inputs(&c), grammar::Profile::NativeV65).unwrap();
    let language = document::DocumentLanguage::prepare(
        &package,
        Default::default(),
        document::AnalysisLimits::workbench(),
    )
    .unwrap();
    let data: Value =
        serde_json::from_str(include_str!("document_tests/nested_recovery.json")).unwrap();
    for row in data["rows"].as_array().unwrap() {
        let source = Source {
            uri: "memory:node-analysis/recovery".into(),
            text: row["text"].as_str().unwrap().into(),
        };
        let analysis = language
            .analyze(&source, None, EvaluationLimits::workbench())
            .unwrap_or_else(|e| panic!("{} {e}", row["text"]));
        assert!(analysis.valid_model.is_none());
        let recovered = analysis.recovered.as_ref().unwrap();
        assert_eq!(
            recovered.objects.len(),
            row["resource"]["model"].as_array().unwrap().len(),
            "{} {}",
            row["text"],
            json!(analysis)
        );
        for (object, expected) in recovered
            .objects
            .iter()
            .zip(row["resource"]["model"].as_array().unwrap())
        {
            assert_eq!(
                json!(object.values()),
                expected["values"],
                "{} {}",
                row["text"],
                json!(analysis)
            );
        }
        let snapshot = &analysis.snapshot;
        let errors: Vec<_> = snapshot.syntax_issues.iter().map(|e| &e.message).collect();
        let expected: Vec<_> = row["resource"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["message"].as_str().unwrap())
            .collect();
        assert_eq!(errors, expected, "{} {}", row["text"], json!(analysis));
        let leaves:Vec<_>=snapshot.document.nodes().iter().filter(|n|n.kind==intersect_model::core::text_document::NodeKind::Leaf).map(|n|json!({"text":document_text(&snapshot.document,n.total_range),"hidden":n.data.hidden,"utf16_start":n.range.start,"utf16_end":n.range.end,"syntax_error":n.data.syntax_error.map(|i|issue_observation(&snapshot.syntax_issues[i]))})).collect();
        assert_eq!(
            json!(leaves),
            row["resource"]["all_root_leaves"],
            "{} {}",
            row["text"],
            json!(analysis)
        );
        let expected = row["nodes"]["nodes"].as_array().unwrap();
        assert_eq!(
            snapshot.document.nodes().len(),
            expected.len(),
            "{} {}",
            row["text"],
            json!(analysis)
        );
        for (i, (n, e)) in snapshot.document.nodes().iter().zip(expected).enumerate() {
            assert_eq!(json!(n.parent), e["parent"], "{} node{i}", row["text"]);
            assert_eq!(json!(n.range.start), e["utf16_start"]);
            assert_eq!(json!(n.range.end), e["utf16_end"]);
            assert_eq!(json!(n.total_range.start), e["total_utf16_start"]);
            assert_eq!(json!(n.total_range.end), e["total_utf16_end"]);
            assert_eq!(
                json!(semantic_ordinal(
                    language.package(),
                    &analysis,
                    intersect_model::core::text_document::NodeId(i)
                )),
                e["semantic_object"]
            );
        }
    }
}

#[test]
fn all_original_resource_node_outcomes() {
    let cases = cases();
    let data: Value =
        serde_json::from_str(include_str!("document_tests/all_resource_nodes.json")).unwrap();
    let mut failures = Vec::new();
    for c in cases.into_iter().filter(|c| {
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == c["id"])
    }) {
        let package = ordinary::compile(inputs(&c), grammar::Profile::NativeV65).unwrap();
        let language = document::DocumentLanguage::prepare(
            &package,
            Default::default(),
            document::AnalysisLimits::workbench(),
        )
        .unwrap();
        for row in data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == c["id"])
        {
            let source = Source {
                uri: "memory:node-analysis/all-resource".into(),
                text: row["text"].as_str().unwrap().into(),
            };
            let result = language.analyze(&source, None, EvaluationLimits::workbench());
            let analysis = match result {
                Ok(a) => a,
                Err(e) => {
                    failures.push(format!("{} {} unavailable: {e}", c["id"], row["text"]));
                    continue;
                }
            };
            let snapshot = &analysis.snapshot;
            let accepted = analysis.valid_model.is_some();
            if accepted != row["resource"]["accepted"].as_bool().unwrap() {
                failures.push(format!(
                    "{} {} publication: {}",
                    c["id"],
                    row["text"],
                    json!(analysis)
                ));
                continue;
            }
            compare_complete_objects(language.package(), &analysis, row, &mut failures);
            let leaves:Vec<_>=snapshot.document.nodes().iter().filter(|n|n.kind==intersect_model::core::text_document::NodeKind::Leaf).map(|n|json!({"text":document_text(&snapshot.document,n.total_range),"hidden":n.data.hidden,"utf16_start":n.range.start,"utf16_end":n.range.end,"syntax_error":n.data.syntax_error.map(|i|issue_observation(&snapshot.syntax_issues[i]))})).collect();
            if json!(leaves) != row["resource"]["all_root_leaves"] {
                failures.push(format!(
                    "{} {} leaves native={} expected={}",
                    c["id"],
                    row["text"],
                    json!(leaves),
                    row["resource"]["all_root_leaves"]
                ));
                continue;
            }
            let expected = row["nodes"]["nodes"].as_array().unwrap();
            let actual:Vec<_>=snapshot.document.nodes().iter().enumerate().map(|(i,n)|json!({"parent":n.parent,"kind":n.kind,"start":n.range.start,"end":n.range.end,"total_start":n.total_range.start,"total_end":n.total_range.end,"semantic":semantic_ordinal(language.package(),&analysis,intersect_model::core::text_document::NodeId(i)),"direct":n.data.direct_semantic_association,"error":n.data.syntax_error.map(|i|issue_observation(&snapshot.syntax_issues[i]))})).collect();
            let expected:Vec<_>=expected.iter().map(|n|json!({"parent":n["parent"],"kind":n["kind"],"start":n["utf16_start"],"end":n["utf16_end"],"total_start":n["total_utf16_start"],"total_end":n["total_utf16_end"],"semantic":n["semantic_object"],"direct":n["direct_semantic_association"],"error":n["syntax_error"]["message"]})).collect();
            if actual != expected {
                failures.push(format!(
                    "{} {} tree native={} expected={}",
                    c["id"],
                    row["text"],
                    json!(actual),
                    json!(expected)
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} original mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn objects(analysis: &document::DocumentAnalysis) -> Vec<(&str, &str, &BTreeMap<String, Value>)> {
    if let Some(model) = &analysis.valid_model {
        model
            .model
            .elements
            .iter()
            .map(|e| (e.id.as_str(), e.class.as_str(), &e.values))
            .collect()
    } else {
        analysis
            .recovered
            .as_ref()
            .unwrap()
            .objects
            .iter()
            .map(|e| (e.id(), e.class(), e.values()))
            .collect()
    }
}
fn classifier_identity(schema: &intersect_model::definitions::schema::Schema, key: &str) -> String {
    let declaration = schema
        .declarations
        .get(key)
        .expect("source-established class identity");
    format!("{}#//{}", declaration.namespace, declaration.name)
}
fn compare_complete_objects(
    package: &Package,
    analysis: &document::DocumentAnalysis,
    row: &Value,
    failures: &mut Vec<String>,
) {
    let raw = objects(analysis);
    let actual: Vec<_> = containment_order(package, analysis)
        .into_iter()
        .map(|i| raw[i])
        .collect();
    let expected = row["resource"]["model"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if actual.len() != expected.len() {
        failures.push(format!(
            "{} {} object count {} vs{}",
            row["id"],
            row["text"],
            actual.len(),
            expected.len()
        ));
        return;
    }
    let wanted_spans = row["resource"]["model_nodes"].as_array().unwrap();
    assert_eq!(
        wanted_spans.len(),
        actual.len(),
        "all final model object spans need correspondence"
    );
    for (i, object) in actual.iter().enumerate() {
        let (start, end) = if let Some(valid) = &analysis.valid_model {
            let element = valid
                .model
                .elements
                .iter()
                .find(|e| e.id == object.0)
                .unwrap();
            (element.start, element.end)
        } else {
            let element = analysis
                .recovered
                .as_ref()
                .unwrap()
                .objects
                .iter()
                .find(|e| e.id() == object.0)
                .unwrap();
            (element.start(), element.end())
        };
        let text = analysis.snapshot.document.text();
        let start = text[..start].encode_utf16().count();
        let end = text[..end].encode_utf16().count();
        if wanted_spans[i]["utf16_start"] != json!(start)
            || wanted_spans[i]["utf16_end"] != json!(end)
        {
            failures.push(format!(
                "{} {} final object{i} span native={start}..{end} expected={}",
                row["id"], row["text"], wanted_spans[i]
            ));
        }
    }
    for (i, ((_, class, values), expected)) in actual.iter().zip(&expected).enumerate() {
        if classifier_identity(&package.schema, class) != expected["class"].as_str().unwrap() {
            failures.push(format!("{} {} class{i}: {class}", row["id"], row["text"]));
            continue;
        }
        let expected_values = expected["values"].as_object().unwrap();
        if values.len() != expected_values.len() {
            failures.push(format!(
                "{} {} object{i} feature count",
                row["id"], row["text"]
            ));
            continue;
        }
        for (name, value) in *values {
            let feature = package.schema.effective_feature(class, name).unwrap();
            let wanted = &expected_values[name];
            if !feature.feature.reference {
                if value != wanted {
                    failures.push(format!(
                        "{} {} object{i}.{name}: {value} vs{wanted}",
                        row["id"], row["text"]
                    ));
                }
                continue;
            }
            let values: Vec<_> = if feature.feature.upper != 1 {
                value.as_array().unwrap().iter().collect()
            } else {
                vec![value]
            };
            let wanted: Vec<_> = if feature.feature.upper != 1 {
                wanted.as_array().unwrap().iter().collect()
            } else {
                vec![wanted]
            };
            if values.len() != wanted.len() {
                failures.push(format!(
                    "{} {} object{i}.{name} reference count",
                    row["id"], row["text"]
                ));
                continue;
            }
            for (value, wanted) in values.into_iter().zip(wanted) {
                if value.is_null() {
                    if !wanted.is_null() {
                        failures.push(format!(
                            "{} {} object{i}.{name} unexpected null",
                            row["id"], row["text"]
                        ));
                    }
                    continue;
                }
                let id = value.as_str().unwrap();
                if let Some((target, (_, class, _))) =
                    actual.iter().enumerate().find(|(_, o)| o.0 == id)
                {
                    if wanted["object"] != json!(target)
                        || wanted["class"] != json!(classifier_identity(&package.schema, class))
                    {
                        failures.push(format!(
                            "{} {} object{i}.{name} target {value} vs{wanted}",
                            row["id"], row["text"]
                        ));
                    }
                } else if let Some(spelling) = id.strip_prefix('?') {
                    let references = row["references_after_resolution"].as_array().unwrap();
                    let reference = references
                        .iter()
                        .find(|r| r["owner"] == json!(i) && r["feature"] == json!(name))
                        .expect("actual unresolved source reference");
                    let nodes = reference["nodes"].as_array().unwrap();
                    let spelling_from_nodes: String = nodes
                        .iter()
                        .flat_map(|n| n["leaves"].as_array().unwrap())
                        .filter(|n| n["hidden"] == false)
                        .map(|n| n["text"].as_str().unwrap())
                        .collect();
                    let target = &reference["targets"][0];
                    if spelling != spelling_from_nodes
                        || target["proxy"] != true
                        || wanted["external"] != target["uri"]
                        || wanted["class"] != target["class"]
                    {
                        failures.push(format!(
                            "{} {} object{i}.{name} unresolved source identity {value} vs{wanted}",
                            row["id"], row["text"]
                        ));
                    }
                } else {
                    failures.push(format!(
                        "{} {} object{i}.{name} unknown target {value}",
                        row["id"], row["text"]
                    ));
                }
            }
        }
    }
}

#[test]
fn actual_named_and_construction_node_models() {
    let cases = cases();
    let mut failures = Vec::new();
    for data in [
        include_str!("document_tests/named_nodes_units.json"),
        include_str!("document_tests/construction_nodes_units.json"),
    ] {
        let data: Value = serde_json::from_str(data).unwrap();
        let mut checked = 0;
        for c in cases.iter().filter(|c| {
            data["rows"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| compilation_case(r) == c["id"].as_str().unwrap())
        }) {
            let package = ordinary::compile(inputs(c), grammar::Profile::NativeV65).unwrap();
            let language = document::DocumentLanguage::prepare(
                &package,
                Default::default(),
                document::AnalysisLimits::workbench(),
            )
            .unwrap();
            for row in data["rows"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| compilation_case(r) == c["id"].as_str().unwrap())
            {
                checked += 1;
                let source = Source {
                    uri: "memory:node-analysis/complete-consumers".into(),
                    text: row["text"].as_str().unwrap().into(),
                };
                let result = language.analyze(
                    &source,
                    row["entry"].as_str(),
                    EvaluationLimits::workbench(),
                );
                let analysis = match result {
                    Ok(a) => a,
                    Err(e) => {
                        failures.push(format!(
                            "{} {} entry{} unavailable: {e}",
                            c["id"], row["text"], row["entry"]
                        ));
                        continue;
                    }
                };
                let snapshot = &analysis.snapshot;
                let mut model_row = row.clone();
                if row["entry"].is_string() {
                    compare_parser_stage(language.package(), &analysis, row, &mut failures);
                }
                // IParser syntax/node observations precede linking and resource validation.
                // Full named evaluation retains those later shared stages; use the exact
                // default resource observation only when its entry is the actual same rule.
                if row["entry"].as_str().is_some_and(|entry| {
                    Some(entry)
                        == language
                            .package()
                            .grammar
                            .as_ref()
                            .map(|g| g.entry.as_str())
                }) {
                    model_row["resource"] = row["default_resource"].clone();
                }
                let syntax_accepted = !objects(&analysis).is_empty()
                    && snapshot
                        .document
                        .nodes()
                        .iter()
                        .all(|n| n.data.syntax_error.is_none());
                if row.get("entry").is_some() {
                    if syntax_accepted != row["resource"]["accepted"].as_bool().unwrap() {
                        failures.push(format!(
                            "{} {} entry{} syntax-stage acceptance",
                            c["id"], row["text"], row["entry"]
                        ));
                    }
                } else if analysis.valid_model.is_some()
                    != row["resource"]["accepted"].as_bool().unwrap()
                {
                    failures.push(format!(
                        "{} {} full resource publication",
                        c["id"], row["text"]
                    ));
                }
                assert!(
                    !(analysis.valid_model.is_some()
                        && (!analysis.diagnostics.is_empty()
                            || !snapshot.syntax_issues.is_empty()))
                );
                compare_complete_objects(language.package(), &analysis, &model_row, &mut failures);
                let leaves:Vec<_>=snapshot.document.nodes().iter().filter(|n|n.kind==intersect_model::core::text_document::NodeKind::Leaf).map(|n|json!({"text":document_text(&snapshot.document,n.total_range),"hidden":n.data.hidden,"utf16_start":n.range.start,"utf16_end":n.range.end,"syntax_error":n.data.syntax_error.map(|i|issue_observation(&snapshot.syntax_issues[i]))})).collect();
                if json!(leaves) != row["resource"]["all_root_leaves"] {
                    failures.push(format!(
                        "{} {} entry{} leaves native={} expected={}",
                        c["id"],
                        row["text"],
                        row["entry"],
                        json!(leaves),
                        row["resource"]["all_root_leaves"]
                    ));
                }
                let actual:Vec<_>=snapshot.document.nodes().iter().enumerate().map(|(i,n)|json!({"parent":n.parent,"kind":n.kind,"start":n.range.start,"end":n.range.end,"total_start":n.total_range.start,"total_end":n.total_range.end,"semantic":semantic_ordinal(language.package(),&analysis,intersect_model::core::text_document::NodeId(i)),"direct":n.data.direct_semantic_association,"error":n.data.syntax_error.map(|i|issue_observation(&snapshot.syntax_issues[i]))})).collect();
                let expected:Vec<_>=row["nodes"]["nodes"].as_array().unwrap().iter().map(|n|json!({"parent":n["parent"],"kind":n["kind"],"start":n["utf16_start"],"end":n["utf16_end"],"total_start":n["total_utf16_start"],"total_end":n["total_utf16_end"],"semantic":n["semantic_object"],"direct":n["direct_semantic_association"],"error":n["syntax_error"]["message"]})).collect();
                if actual != expected {
                    failures.push(format!(
                        "{} {} entry{} tree native={} expected={}",
                        c["id"],
                        row["text"],
                        row["entry"],
                        json!(actual),
                        json!(expected)
                    ));
                }
            }
        }
        assert_eq!(
            checked,
            data["rows"].as_array().unwrap().len(),
            "no exact observation may be skipped"
        );
    }
    assert!(
        failures.is_empty(),
        "{} exact original named/construction mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn document_text(
    document: &intersect_model::core::text_document::TextDocument,
    range: intersect_model::core::text_document::TextRange,
) -> Value {
    let units: Vec<_> = document.units_in(range).unwrap().collect();
    match String::from_utf16(&units) {
        Ok(text) => json!(text),
        Err(_) => json!({"utf16_units":units}),
    }
}

fn compilation_case(row: &Value) -> &str {
    row["compilation_case_id"]
        .as_str()
        .or_else(|| row["id"].as_str())
        .unwrap()
}
/// Verification-only correspondence to upstream EObject containment traversal.
/// Native creation IDs/order and all raw oracle observations remain unchanged.
fn containment_order(package: &Package, analysis: &document::DocumentAnalysis) -> Vec<usize> {
    let actual = objects(analysis);
    let resources = if let Some(model) = &analysis.valid_model {
        &model.model.resources
    } else {
        &analysis.recovered.as_ref().unwrap().resources
    };
    let mut pending: Vec<_> = resources
        .values()
        .flat_map(|r| r.roots.iter().cloned())
        .collect();
    pending.reverse();
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    while let Some(id) = pending.pop() {
        let i = actual
            .iter()
            .position(|e| e.0 == id)
            .expect("source-rooted actual object");
        assert!(seen.insert(i), "ambiguous/cyclic containment identity");
        order.push(i);
        let mut children = Vec::new();
        for member in package.schema.effective_features(actual[i].1).unwrap() {
            if member.feature.containment {
                let value = &actual[i].2[&member.feature.name];
                if let Some(values) = value.as_array() {
                    children.extend(values.iter().map(|v| v.as_str().unwrap().to_owned()));
                } else if let Some(value) = value.as_str() {
                    children.push(value.to_owned());
                }
            }
        }
        pending.extend(children.into_iter().rev());
    }
    assert_eq!(
        order.len(),
        actual.len(),
        "unmapped/orphan native objects cannot be discarded"
    );
    order
}
fn semantic_ordinal(
    package: &Package,
    analysis: &document::DocumentAnalysis,
    node: intersect_model::core::text_document::NodeId,
) -> Option<usize> {
    analysis
        .snapshot
        .semantic_object(node)
        .unwrap()
        .map(|index| {
            containment_order(package, analysis)
                .iter()
                .position(|i| *i == index)
                .expect("mapped actual semantic object")
        })
}

fn issue_observation(issue: &document::SyntaxIssue) -> Value {
    match &issue.message_utf16 {
        Some(units) => json!({"utf16_units":units}),
        None => json!(issue.message),
    }
}

#[test]
fn original_initializer_context_keeps_complete_native_success_outputs() {
    let data: Value = serde_json::from_str(include_str!(
        "document_tests/original_provider_positive.json"
    ))
    .unwrap();
    let compilation: BundleCompilation =
        serde_json::from_value(data["original_compilation"].clone()).unwrap();
    let package = ordinary::compile(compilation, grammar::Profile::NativeV65).unwrap();
    let language = document::DocumentLanguage::prepare(
        &package,
        Default::default(),
        document::AnalysisLimits::workbench(),
    )
    .unwrap();
    for row in data["rows"].as_array().unwrap() {
        let source: Source = serde_json::from_value(row["source"].clone()).unwrap();
        let analysis = language
            .analyze(&source, None, EvaluationLimits::workbench())
            .unwrap();
        let actual = analysis
            .valid_model
            .as_ref()
            .expect("original selected initializer success");
        assert_eq!(json!(actual), row["result"], "{}", row["label"]);
        assert!(
            actual
                .model
                .elements
                .iter()
                .filter(|e| e.class == "Type")
                .all(|e| e.values["origin"].as_str().is_some())
        );
    }
}

#[test]
fn all_original_node_source_and_feature_associations() {
    let cases = cases();
    let data: Value =
        serde_json::from_str(include_str!("document_tests/all_resource_nodes.json")).unwrap();
    let mut failures = Vec::new();
    for c in cases.iter().filter(|c| {
        data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == c["id"])
    }) {
        let package = ordinary::compile(inputs(c), grammar::Profile::NativeV65).unwrap();
        let language = document::DocumentLanguage::prepare(
            &package,
            Default::default(),
            document::AnalysisLimits::workbench(),
        )
        .unwrap();
        for row in data["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["id"] == c["id"])
        {
            let source = Source {
                uri: "memory:node-analysis/source-links".into(),
                text: row["text"].as_str().unwrap().into(),
            };
            let analysis = language
                .analyze(&source, None, EvaluationLimits::workbench())
                .unwrap();
            compare_node_origins(language.package(), &analysis, row, &mut failures);
            for feature in &analysis.snapshot.feature_nodes {
                let class = &analysis.snapshot.semantic_objects[feature.owner_object].class;
                assert_eq!(
                    language
                        .package()
                        .schema
                        .feature_by_id(class, &feature.identity)
                        .unwrap()
                        .feature
                        .name,
                    feature.feature
                );
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} exact node source/feature mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
fn compare_node_origins(
    package: &Package,
    analysis: &document::DocumentAnalysis,
    row: &Value,
    failures: &mut Vec<String>,
) {
    let snapshot = &analysis.snapshot;
    let raw = objects(analysis);
    let order = containment_order(package, analysis);
    let bindings: Value =
        serde_json::from_str(include_str!("document_tests/source_origin_bindings.json")).unwrap();
    let bindings: Vec<_> = bindings["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["id"] == row["id"])
        .collect();
    assert_eq!(bindings.len(), 1, "exact source-loaded binding graph");
    let bindings = bindings[0]["observation"]["source_node_bindings"]
        .as_array()
        .unwrap();
    let expected = row["nodes"]["nodes"].as_array().unwrap();
    if expected.len() != snapshot.document.nodes().len() {
        return;
    }
    for (index, (node, wanted)) in snapshot.document.nodes().iter().zip(expected).enumerate() {
        assert_eq!(
            document_text(&snapshot.document, node.total_range),
            wanted["text"],
            "complete source-unit composite/leaf text at {index}"
        );
        if let Some(issue) = node.data.syntax_error {
            let actual = &snapshot.syntax_issues[issue];
            assert_eq!(
                actual.issue_code,
                wanted["syntax_error"]["issue_code"].as_str().unwrap()
            );
            assert_eq!(
                json!(actual.issue_data),
                wanted["syntax_error"]["issue_data"]
            );
        }

        let actual = node.data.origin.map(|at| &snapshot.origins()[at]);
        let wanted = &wanted["grammar"];
        match (actual, wanted.is_null()) {
            (None, true) => {}
            (Some(origin), false) => {
                let kind = match origin.kind {
                    document::OriginKind::ParserRule => "ParserRule",
                    document::OriginKind::GeneratedParserRule => "GeneratedParserRule",
                    document::OriginKind::GeneratedRuleCall => "GeneratedRuleCall",
                    document::OriginKind::TerminalRule => "TerminalRule",
                    document::OriginKind::Keyword => "Keyword",
                    document::OriginKind::RuleCall => "RuleCall",
                    document::OriginKind::CrossReference => "CrossReference",
                    document::OriginKind::Action => "Action",
                };
                let source = origin
                    .expression
                    .as_ref()
                    .or(origin.declaration.as_ref())
                    .map(|s| json!(s.source));
                let bound: Vec<_> = bindings
                    .iter()
                    .filter(|binding| binding["generated_raw_uri"] == wanted["raw_uri"])
                    .collect();
                assert_eq!(
                    bound.len(),
                    1,
                    "every generated raw EObject URI needs one source-loaded structural binding"
                );
                let bound = bound[0];
                let span = origin
                    .expression
                    .as_ref()
                    .or(origin.declaration.as_ref())
                    .expect("authored node needs exact original source span");
                assert_eq!(json!(span.source), bound["authored_source"]);
                assert_eq!(bound["kind"], kind);
                assert_eq!(bound["generated_proxy"], false);
                assert_eq!(bound["authored_proxy"], false);
                let text = &package
                    .compilation
                    .as_ref()
                    .unwrap()
                    .bundle
                    .sources
                    .iter()
                    .find(|s| s.id == span.source)
                    .expect("exact original source bytes")
                    .text;
                let start = text[..span.start].encode_utf16().count();
                let end = text[..span.end].encode_utf16().count();
                if bound["authored_node"]["utf16_start"] != json!(start)
                    || bound["authored_node"]["utf16_end"] != json!(end)
                {
                    failures.push(format!("{} {} node{index} exact authored source range native={start}..{end} expected={}",row["id"],row["text"],bound));
                }

                if wanted["kind"] != kind
                    || wanted["proxy"] != false
                    || source != Some(wanted["authored_owner_source"].clone())
                {
                    failures.push(format!(
                        "{} {} node{index} grammar kind/source native={} expected={}",
                        row["id"],
                        row["text"],
                        json!(origin),
                        wanted
                    ));
                }
            }
            _ => failures.push(format!(
                "{} {} node{index} missing/unexpected original grammar source",
                row["id"], row["text"]
            )),
        }
    }
    let mut expected_features: BTreeMap<(usize, String), Vec<usize>> = BTreeMap::new();
    for association in row["nodes"]["semantic_associations"].as_array().unwrap() {
        let object = association["object"].as_u64().unwrap() as usize;
        assert_eq!(
            classifier_identity(&package.schema, raw[order[object]].1),
            association["class"]
        );
        for feature in association["features"].as_array().unwrap() {
            let member = package
                .schema
                .effective_feature(raw[order[object]].1, feature["name"].as_str().unwrap())
                .unwrap();
            assert_eq!(
                format!(
                    "{}/{}",
                    classifier_identity(&package.schema, member.owner),
                    feature["name"].as_str().unwrap()
                ),
                feature["identity"].as_str().unwrap()
            );
            let nodes:Vec<_>=feature["nodes"].as_array().unwrap().iter().map(|n|{if let Some(index)=n["node"].as_u64(){index as usize}else{let candidates:Vec<_>=expected.iter().filter(|actual|actual["grammar"]["raw_uri"]==n["grammar"]["raw_uri"]&&actual["grammar"]["authored_owner_source"]==n["grammar"]["authored_owner_source"]&&actual["utf16_start"]==n["utf16_start"]&&actual["utf16_end"]==n["utf16_end"]).collect();assert_eq!(candidates.len(),1,"raw upstream wrapper node:null requires unique exact grammar/source/range correspondence");candidates[0]["id"].as_u64().unwrap()as usize}}).collect();
            if !nodes.is_empty() {
                expected_features.insert((object, feature["name"].as_str().unwrap().into()), nodes);
            }
        }
    }
    let mut actual_features: BTreeMap<(usize, String), Vec<usize>> = BTreeMap::new();
    for feature in &snapshot.feature_nodes {
        actual_features
            .entry((
                order
                    .iter()
                    .position(|i| *i == feature.owner_object)
                    .expect("every native source feature owner must map uniquely"),
                feature.feature.clone(),
            ))
            .or_default()
            .push(feature.node.0);
    }
    if actual_features != expected_features {
        failures.push(format!(
            "{} {} source feature nodes actual={actual_features:?} expected={expected_features:?}",
            row["id"], row["text"]
        ));
    }
}

#[test]
fn absent_behavior_selector_stays_transport_rejected() {
    let observed: Value = serde_json::from_str(include_str!(
        "document_tests/absent_behavior_transport_negative.json"
    ))
    .unwrap();
    let error =
        serde_json::from_value::<BundleCompilation>(observed["request"]["compilation"].clone())
            .unwrap_err();
    assert!(
        error.to_string().contains("missing field `rcore_behavior`"),
        "{error}"
    );
}

fn parsed_objects(
    analysis: &document::DocumentAnalysis,
) -> Vec<(&str, &str, &document::ParsedValues)> {
    analysis
        .snapshot
        .parse_stage
        .objects
        .iter()
        .map(|o| (o.id.as_str(), o.class.as_str(), &o.values))
        .collect()
}
fn parse_order(package: &Package, analysis: &document::DocumentAnalysis) -> Vec<usize> {
    let raw = parsed_objects(analysis);
    let mut children = vec![Vec::new(); raw.len()];
    let mut inbound = vec![0usize; raw.len()];
    for (i, (_, class, values)) in raw.iter().enumerate() {
        for member in package.schema.effective_features(class).unwrap() {
            if member.feature.containment {
                let value = &values[&member.feature.name];
                let ids: Vec<_> = if let Some(values) = value.as_array() {
                    values.iter().map(|v| v.as_str().unwrap()).collect()
                } else {
                    value.as_str().into_iter().collect()
                };
                for id in ids {
                    let target = raw
                        .iter()
                        .position(|e| e.0 == id)
                        .expect("parse containment must name an actual parse-stage object");
                    inbound[target] += 1;
                    assert_eq!(inbound[target], 1, "ambiguous multiple containment");
                    children[i].push(target);
                }
            }
        }
    }
    let roots: Vec<_> = inbound
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (*n == 0).then_some(i))
        .collect();
    assert!(
        roots.len() <= 1,
        "unmapped parse-stage forest cannot be silently dropped"
    );
    if let Some(entry::EntryValue::Object(id)) = &analysis.snapshot.parse_stage.returned {
        assert_eq!(
            roots.iter().map(|i| raw[*i].0).collect::<Vec<_>>(),
            vec![id.as_str()],
            "actual parser return must identify its full root"
        );
    }
    let mut pending: Vec<_> = roots.into_iter().rev().collect();
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    while let Some(i) = pending.pop() {
        assert!(seen.insert(i), "cyclic parse-stage containment");
        order.push(i);
        pending.extend(children[i].iter().rev());
    }
    assert_eq!(
        order.len(),
        raw.len(),
        "all parse-stage objects must be mapped"
    );
    order
}
fn compare_parser_stage(
    package: &Package,
    analysis: &document::DocumentAnalysis,
    row: &Value,
    failures: &mut Vec<String>,
) {
    let raw = parsed_objects(analysis);
    let order = parse_order(package, analysis);
    let actual: Vec<_> = order.iter().map(|i| raw[*i]).collect();
    let expected = row["resource"]["model"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if actual.len() != expected.len() {
        failures.push(format!(
            "{} {} pre-link object count {} vs{}",
            row["id"],
            row["text"],
            actual.len(),
            expected.len()
        ));
        return;
    }
    let wanted_spans = row["resource"]["model_nodes"].as_array().unwrap();
    assert_eq!(
        wanted_spans.len(),
        actual.len(),
        "all actual pre-link model object spans need correspondence"
    );
    for (i, index) in order.iter().enumerate() {
        let element = &analysis.snapshot.parse_stage.objects[*index];
        let text = analysis.snapshot.document.text();
        let start = text[..element.start].encode_utf16().count();
        let end = text[..element.end].encode_utf16().count();
        if wanted_spans[i]["utf16_start"] != json!(start)
            || wanted_spans[i]["utf16_end"] != json!(end)
        {
            failures.push(format!(
                "{} {} actual pre-link object{i} span native={start}..{end} expected={}",
                row["id"], row["text"], wanted_spans[i]
            ));
        }
    }
    for (i, ((_, class, values), wanted)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            classifier_identity(&package.schema, class),
            wanted["class"].as_str().unwrap(),
            "pre-link class exact source identity"
        );
        let wanted = wanted["values"].as_object().unwrap();
        assert_eq!(values.len(), wanted.len(), "pre-link all feature count");
        for (name, value) in *values {
            let member = package.schema.effective_feature(class, name).unwrap();
            let wanted = &wanted[name];
            if !member.feature.reference {
                if value != wanted {
                    failures.push(format!(
                        "{} {} pre-link object{i}.{name}: {value} vs{wanted}",
                        row["id"], row["text"]
                    ));
                }
                continue;
            }
            let values: Vec<_> = if member.feature.upper != 1 {
                value.as_array().unwrap().iter().collect()
            } else {
                vec![value]
            };
            let wanted: Vec<_> = if member.feature.upper != 1 {
                wanted.as_array().unwrap().iter().collect()
            } else {
                vec![wanted]
            };
            assert_eq!(
                values.len(),
                wanted.len(),
                "pre-link reference multiplicity"
            );
            for (value, wanted) in values.into_iter().zip(wanted) {
                if value.is_null() {
                    assert!(
                        wanted.is_null(),
                        "parse-stage unlinked reference state differs"
                    );
                } else {
                    let id = value.as_str().unwrap();
                    if let Some(spelling) = id.strip_prefix('?') {
                        assert!(
                            wanted.is_null(),
                            "upstream detached IParser pending feature is null"
                        );
                        let matches: Vec<_> = analysis
                            .snapshot
                            .parse_stage
                            .references
                            .iter()
                            .filter(|r| {
                                r.active
                                    && r.owner == actual[i].0
                                    && r.field == *name
                                    && r.spelling == spelling
                            })
                            .collect();
                        assert_eq!(
                            matches.len(),
                            1,
                            "native pending raw value must match its exact active source state"
                        );
                    } else {
                        let (target, (_, class, _)) =
                            actual.iter().enumerate().find(|(_, o)| o.0 == id).expect(
                                "pre-link reference must identify an actual contained object",
                            );
                        assert_eq!(wanted["object"], json!(target));
                        assert_eq!(
                            wanted["class"],
                            json!(classifier_identity(&package.schema, class))
                        );
                    }
                }
            }
        }
    }
    let mut checked = BTreeSet::new();
    for pending in analysis
        .snapshot
        .parse_stage
        .references
        .iter()
        .filter(|r| r.active)
    {
        assert_eq!(&pending.source, analysis.snapshot.document.source());
        assert_eq!(
            pending.source.sha256,
            intersect_model::definitions::bundle::SourceId::for_text(
                &pending.source.uri,
                analysis.snapshot.document.text()
            )
            .sha256
        );
        assert!(
            pending.start <= pending.end
                && analysis
                    .snapshot
                    .document
                    .text()
                    .get(pending.start..pending.end)
                    .is_some()
        );
        assert_eq!(
            row["entry"].as_str(),
            package.grammar.as_ref().map(|g| g.entry.as_str()),
            "a nondefault pending-reference parser needs its own source-linked resource observation"
        );
        let ordinal = actual.iter().position(|o| o.0 == pending.owner).unwrap();
        let references = row["references_after_resolution"].as_array().unwrap();
        let matches: Vec<_> = references
            .iter()
            .enumerate()
            .filter(|(_, r)| r["owner"] == json!(ordinal) && r["feature"] == pending.field)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "unique full source reference correspondence"
        );
        let (index, expected) = matches[0];
        assert!(
            checked.insert(index),
            "scalar source reference mapped twice"
        );
        let nodes = expected["nodes"].as_array().unwrap();
        assert_eq!(
            nodes.len(),
            1,
            "each scalar pending reference retains exact source occurrence"
        );
        let node = &nodes[0];
        let spelling: String = node["leaves"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["hidden"] == false)
            .map(|n| n["text"].as_str().unwrap())
            .collect();
        assert_eq!(pending.spelling, spelling);
        assert_eq!(
            analysis.snapshot.document.text()[..pending.start]
                .encode_utf16()
                .count(),
            node["utf16_start"].as_u64().unwrap() as usize
        );
        assert_eq!(
            analysis.snapshot.document.text()[..pending.end]
                .encode_utf16()
                .count(),
            node["utf16_end"].as_u64().unwrap() as usize
        );
        assert!(
            pending.index.is_none(),
            "scalar fixture cannot qualify unchecked collection pending indices"
        );
        assert_eq!(
            classifier_identity(&package.schema, &pending.target),
            expected["targets"][0]["class"].as_str().unwrap()
        );
    }
    for (index, reference) in row["references_after_resolution"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        if !checked.contains(&index) {
            let nodes = reference["nodes"].as_array().unwrap();
            assert_eq!(
                nodes.len(),
                1,
                "unmatched source reference requires an exact empty occurrence"
            );
            let node = &nodes[0];
            assert_eq!(node["utf16_start"], node["utf16_end"]);
            assert!(node["leaves"].as_array().unwrap().is_empty());
            assert!(reference["targets"].as_array().unwrap().is_empty());
            let owner = reference["owner"].as_u64().unwrap() as usize;
            assert!(
                actual[owner].2[reference["feature"].as_str().unwrap()].is_null(),
                "empty spelling retains null field and no phantom pending identity"
            );
            assert!(
                !analysis
                    .snapshot
                    .parse_stage
                    .references
                    .iter()
                    .any(|r| r.active
                        && r.owner == actual[owner].0
                        && r.field == reference["feature"].as_str().unwrap())
            );
            checked.insert(index);
        }
    }
    assert_eq!(
        checked.len(),
        row["references_after_resolution"].as_array().unwrap().len(),
        "every actual and empty source-reference occurrence must be checked"
    );
}

#[test]
fn all_named_and_construction_node_source_and_feature_associations() {
    let cases = cases();
    let mut failures = Vec::new();
    for data in [
        include_str!("document_tests/named_nodes_units.json"),
        include_str!("document_tests/construction_nodes_units.json"),
    ] {
        let data: Value = serde_json::from_str(data).unwrap();
        let mut checked = 0;
        for c in cases.iter().filter(|c| {
            data["rows"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| compilation_case(r) == c["id"].as_str().unwrap())
        }) {
            let package = ordinary::compile(inputs(c), grammar::Profile::NativeV65).unwrap();
            let language = document::DocumentLanguage::prepare(
                &package,
                Default::default(),
                document::AnalysisLimits::workbench(),
            )
            .unwrap();
            for row in data["rows"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| compilation_case(r) == c["id"].as_str().unwrap())
            {
                checked += 1;
                let source = Source {
                    uri: "memory:node-analysis/complete-source-links".into(),
                    text: row["text"].as_str().unwrap().into(),
                };
                let analysis = language
                    .analyze(
                        &source,
                        row["entry"].as_str(),
                        EvaluationLimits::workbench(),
                    )
                    .unwrap();
                compare_node_origins(language.package(), &analysis, row, &mut failures);
            }
        }
        assert_eq!(checked, data["rows"].as_array().unwrap().len());
    }
    assert!(
        failures.is_empty(),
        "{} original named/construction source association differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn actual_native_prediction_dependencies_cover_unconsumed_suffix_and_end() {
    let c = cases().remove(0);
    let mut input = inputs(&c);
    let old = input.xtext.clone().unwrap();
    let at = input
        .bundle
        .sources
        .iter()
        .position(|s| s.id == old)
        .unwrap();
    let mut source = DefinitionSource::new(
        "memory:document-mechanics/NativeDependency.xtext",
        SourceKind::Xtext,
        "grammar document.mechanics.NativeDependency hidden(WS)\nimport \"urn:intersect:xtext:mini-kerml:terminal-spellings:1\" as k\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nModel returns k::Model: name=First 'finish'?;\nFirst returns ecore::EString: 'a' ('b'|'c')*;\nterminal WS: (' '| '\\t' | '\\n')+;\n",
    );
    source.dependencies = input.bundle.sources[at].dependencies.clone();
    input.xtext = Some(source.id.clone());
    input.bundle.sources[at] = source;
    let p = ordinary::compile(input, grammar::Profile::NativeV65).unwrap();
    let language = document::DocumentLanguage::prepare(
        &p,
        Default::default(),
        document::AnalysisLimits::workbench(),
    )
    .unwrap();
    for text in ["a finish", "a"] {
        let source = Source {
            uri: "memory:native-prediction-dependency/input".into(),
            text: text.into(),
        };
        let analysis = language
            .analyze(&source, None, EvaluationLimits::workbench())
            .unwrap();
        let model = analysis.valid_model.as_ref().unwrap();
        let snapshot = &analysis.snapshot;
        let resource = &model.model.resources[&source.uri];
        assert!(!snapshot.prediction_dependencies.is_empty());
        for dep in &snapshot.prediction_dependencies {
            assert!(
                dep.range.start <= dep.range.end && dep.range.end <= snapshot.document.utf16_len()
            );
            assert!(dep.occurrence.starts_with("rule/"));
            assert_eq!(dep.hidden_policy, vec!["WS"]);
            assert!(snapshot.document.nodes().get(dep.node.0).is_some());
        }
        if text == "a finish" {
            let dep = snapshot
                .prediction_dependencies
                .iter()
                .find(|d| snapshot.document.nodes()[d.node.0].total_range.end < d.range.end)
                .expect("actual predictor probes a caller suffix outside consumed child node");
            let edit = intersect_model::core::text_document::TextRange { start: 3, end: 4 };
            assert!(dep.range.start <= edit.start && dep.range.end >= edit.end);
            let invalid = snapshot
                .invalidated_nodes(
                    resource,
                    edit,
                    document::InvalidationLimits {
                        operations: 10_000,
                        nodes: 1000,
                    },
                )
                .unwrap();
            assert!(invalid.contains(&dep.node));
            assert!(
                snapshot
                    .invalidated_nodes(
                        resource,
                        edit,
                        document::InvalidationLimits {
                            operations: 0,
                            nodes: 1000
                        }
                    )
                    .unwrap_err()
                    .contains("operation")
            );
            assert!(
                snapshot
                    .invalidated_nodes(
                        resource,
                        edit,
                        document::InvalidationLimits {
                            operations: 10_000,
                            nodes: 0
                        }
                    )
                    .unwrap_err()
                    .contains("node")
            );
            let mut stale = resource.clone();
            stale.content_revision += 1;
            assert!(
                snapshot
                    .invalidated_nodes(
                        &stale,
                        edit,
                        document::InvalidationLimits {
                            operations: 10_000,
                            nodes: 1000
                        }
                    )
                    .unwrap_err()
                    .contains("revision")
            );
        } else {
            assert!(
                snapshot
                    .prediction_dependencies
                    .iter()
                    .any(|d| d.reached_end
                        && d.range.start == snapshot.document.utf16_len()
                        && d.range.end == snapshot.document.utf16_len())
            );
        }
    }
}

#[test]
fn workspace_analysis_preserves_cross_resource_linking_and_quarantines_all_invalid_sources() {
    let c = cases()
        .into_iter()
        .find(|c| c["id"] == "reference_adds_scope")
        .unwrap();
    let package = ordinary::compile(inputs(&c), grammar::Profile::NativeV65).unwrap();
    let language = document::DocumentLanguage::prepare(
        &package,
        Default::default(),
        document::AnalysisLimits::workbench(),
    )
    .unwrap();
    let sources = vec![
        Source {
            uri: "memory:node-analysis/workspace/a".into(),
            text: "type:a;use:f=b;".into(),
        },
        Source {
            uri: "memory:node-analysis/workspace/b".into(),
            text: "type:b;use:g=a;".into(),
        },
    ];
    let analysis = language
        .analyze_sources(&sources, None, EvaluationLimits::workbench())
        .unwrap();
    let valid = analysis
        .valid_model
        .as_ref()
        .expect("full actual cross-resource linking");
    let frozen = json!(valid);
    assert_eq!(json!(valid), json!(evaluate(&package, &sources)));
    assert_eq!(analysis.snapshots.len(), 2);
    for (snapshot, source) in analysis.snapshots.iter().zip(&sources) {
        assert_eq!(
            snapshot.document.source(),
            &intersect_model::definitions::bundle::SourceId::for_text(&source.uri, &source.text)
        );
        assert_eq!(snapshot.semantic_objects.len(), 3);
        assert!(
            snapshot
                .semantic_objects
                .iter()
                .all(|o| o.id.starts_with(&format!("{}#", source.uri)))
        );
        for node in snapshot.document.nodes() {
            if let Some(index) = node.data.semantic_object {
                assert!(
                    snapshot.semantic_objects[index]
                        .id
                        .starts_with(&format!("{}#", source.uri))
                );
            }
        }
        assert!(
            snapshot
                .parse_stage
                .references
                .iter()
                .filter(|r| r.active)
                .all(|r| r.source == *snapshot.document.source())
        );
    }
    let mut invalid = sources.clone();
    invalid[1].text = "type:b;use:g=;".into();
    let partial = language
        .analyze_sources(&invalid, None, EvaluationLimits::workbench())
        .unwrap();
    assert!(partial.valid_model.is_none());
    assert!(partial.recovered.is_some());
    assert_eq!(partial.snapshots.len(), 2);
    assert_eq!(
        json!(analysis.valid_model.as_ref().unwrap()),
        frozen,
        "later recovered analysis cannot mutate an already owned valid evaluation"
    );
    assert!(
        language
            .analyze_sources(
                &[sources[0].clone(), sources[0].clone()],
                None,
                EvaluationLimits::workbench()
            )
            .unwrap_err()
            .contains("Duplicate")
    );
}
#[test]
fn durable_analysis_exact_and_one_short_limits_never_publish_partial_as_valid() {
    let c = cases()
        .into_iter()
        .find(|c| c["id"] == "namespace_nested_restore")
        .unwrap();
    let p = ordinary::compile(inputs(&c), grammar::Profile::NativeV65).unwrap();
    let source = Source {
        uri: "memory:document-budget/input".into(),
        text: "[ab c]done".into(),
    };
    // Use an original accepted source, retaining its exact native usage.
    let data: Value =
        serde_json::from_str(include_str!("document_tests/positive_nodes.json")).unwrap();
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == c["id"])
        .unwrap();
    let source = Source {
        text: row["text"].as_str().unwrap().into(),
        ..source
    };
    let generous = document::DocumentLanguage::prepare(
        &p,
        Default::default(),
        document::AnalysisLimits::workbench(),
    )
    .unwrap();
    let baseline = generous
        .analyze(&source, None, EvaluationLimits::workbench())
        .unwrap();
    let usage = baseline.snapshot.tree_usage;
    assert!(baseline.valid_model.is_some());
    for mode in 0..4 {
        let mut limits = document::AnalysisLimits::workbench();
        match mode {
            0 => limits.tree.nodes = usage.nodes,
            1 => limits.tree.operations = usage.operations,
            2 => limits.tree.logical_storage_bytes = usage.peak_logical_storage_bytes,
            _ => limits.tree.source_bytes = source.text.len(),
        };
        let exact = document::DocumentLanguage::prepare(&p, Default::default(), limits).unwrap();
        assert!(
            exact
                .analyze(&source, None, EvaluationLimits::workbench())
                .unwrap()
                .valid_model
                .is_some()
        );
        match mode {
            0 => limits.tree.nodes -= 1,
            1 => limits.tree.operations -= 1,
            2 => limits.tree.logical_storage_bytes -= 1,
            _ => limits.tree.source_bytes -= 1,
        };
        let short = document::DocumentLanguage::prepare(&p, Default::default(), limits).unwrap();
        assert!(
            short
                .analyze(&source, None, EvaluationLimits::workbench())
                .is_err(),
            "one-short logical document limit{mode}"
        );
    }
    let mut limits = document::AnalysisLimits::workbench();
    limits.resources = 0;
    let short = document::DocumentLanguage::prepare(&p, Default::default(), limits).unwrap();
    assert!(
        short
            .analyze(&source, None, EvaluationLimits::workbench())
            .unwrap_err()
            .contains("resource budget")
    );
}
#[test]
fn serialized_node_programs_and_origins_cannot_replace_exact_source_replay() {
    let c = cases()
        .into_iter()
        .find(|c| c["id"] == "namespace_nested_restore")
        .unwrap();
    let package = ordinary::compile(inputs(&c), grammar::Profile::NativeV65).unwrap();
    let mut forged = package.clone();
    forged
        .grammar
        .as_mut()
        .unwrap()
        .rules
        .get_mut("Model")
        .unwrap()
        .body
        .original_range = Some(0..0);
    match document::DocumentLanguage::prepare(
        &forged,
        Default::default(),
        document::AnalysisLimits::workbench(),
    ) {
        Err(_) => {}
        Ok(language) => assert_eq!(
            json!(language.package()),
            json!(package),
            "fresh source replay must erase forged executable origin cache"
        ),
    };
    let old = ordinary::compile(inputs(&c), grammar::Profile::NativeV64).unwrap();
    assert!(
        document::DocumentLanguage::prepare(
            &old,
            Default::default(),
            document::AnalysisLimits::workbench()
        )
        .is_err()
    );
}

#[test]
fn typed_node_links_require_owned_arena_and_current_model_schema_resource() {
    let c = cases()
        .into_iter()
        .find(|c| c["id"] == "namespace_nested_restore")
        .unwrap();
    let package = ordinary::compile(inputs(&c), grammar::Profile::NativeV65).unwrap();
    let language = document::DocumentLanguage::prepare(
        &package,
        Default::default(),
        document::AnalysisLimits::workbench(),
    )
    .unwrap();
    let data: Value =
        serde_json::from_str(include_str!("document_tests/positive_nodes.json")).unwrap();
    let row = data["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == c["id"])
        .unwrap();
    let source = Source {
        uri: "memory:typed-document-links/input".into(),
        text: row["text"].as_str().unwrap().into(),
    };
    let first = language
        .analyze(&source, None, EvaluationLimits::workbench())
        .unwrap();
    let other = language
        .analyze(&source, None, EvaluationLimits::workbench())
        .unwrap();
    let model = &first.valid_model.as_ref().unwrap().model;
    let node = first
        .snapshot
        .document
        .handle(intersect_model::core::text_document::NodeId(0))
        .unwrap();
    let object = first
        .snapshot
        .linked_object(&node, model, &package.schema)
        .unwrap()
        .unwrap();
    assert_eq!(object.id, first.snapshot.semantic_objects[0].id);
    let foreign = other.snapshot.document.handle(node.node).unwrap();
    assert!(
        first
            .snapshot
            .linked_object(&foreign, model, &package.schema)
            .is_err()
    );
    let mut stale = model.clone();
    stale
        .resources
        .get_mut(&source.uri)
        .unwrap()
        .content_revision += 1;
    assert!(
        first
            .snapshot
            .linked_object(&node, &stale, &package.schema)
            .unwrap_err()
            .contains("revision")
    );
    let mut stale = model.clone();
    stale.schema_revision = "changed".into();
    assert!(
        first
            .snapshot
            .linked_object(&node, &stale, &package.schema)
            .unwrap_err()
            .contains("schema")
    );
    let mut stale = model.clone();
    stale.elements[0].id = "other".into();
    assert!(
        first
            .snapshot
            .linked_object(&node, &stale, &package.schema)
            .unwrap_err()
            .contains("absent")
    );
}
