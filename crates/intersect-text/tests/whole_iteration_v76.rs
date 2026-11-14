//! Cached genuine Xtext2.38 observations. No JVM/generator or source repair.
//! A queue proof does not make these parser/model assertions pass implicitly.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
    evaluate_with_limits, ordinary,
};
use serde_json::{Value, json};

const NS: &str = "urn:intersect:oracle:WholeIterationGate";
const ECORE: &str = include_str!("../../../fixtures/xtext-whole-iteration-v76/GateModel.ecore");
fn input(name: &str, text: &str) -> BundleCompilation {
    let model = DefinitionSource::new(
        "memory:whole-iteration/GateModel.ecore",
        SourceKind::Ecore,
        ECORE,
    );
    let mut grammar = DefinitionSource::new(
        format!("memory:whole-iteration/{name}.xtext"),
        SourceKind::Xtext,
        text,
    );
    grammar.dependencies.push(model.id.clone());
    let mut rules = DefinitionSource::new(
        "memory:whole-iteration/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package emptyRules version \"1\";",
    );
    rules.dependencies.push(model.id.clone());
    let input = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: NS.into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: rules.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), grammar, rules],
            namespaces: vec![],
        },
    };
    let migrated = intersect_model::migration::migrate_ecore_bundle_with_limits(
        input.bundle.clone(),
        &[(
            model.id.clone(),
            "memory:whole-iteration/GateModel.rcore".into(),
        )]
        .into(),
        ParseLimits::default(),
    )
    .unwrap();
    assert_eq!(migrated.reports[0].original, model);
    intersect_text::migration::remap_compilation(&migrated, &input).unwrap()
}

fn graph_projection(
    package: &Package,
    evaluation: &intersect_text::Evaluation,
) -> Result<(Value, Vec<String>), String> {
    let roots: Vec<_> = evaluation
        .model
        .elements
        .iter()
        .filter(|object| object.class == "Root")
        .collect();
    if roots.len() != 1 {
        return Err(format!("Expected exactly one Root, found {}", roots.len()));
    }
    let root = roots[0];
    let children = root
        .values
        .get("children")
        .and_then(Value::as_array)
        .ok_or("Missing actual children collection")?;
    let mut objects = vec![root];
    for child in children {
        let identity = child
            .as_str()
            .ok_or("Child has no native object identity")?;
        let object = evaluation
            .model
            .elements
            .iter()
            .find(|object| object.id == identity)
            .ok_or("Actual containment target missing")?;
        if object.class != "Target" {
            return Err("Actual child has the wrong class".into());
        }
        objects.push(object);
    }
    if objects.len() != evaluation.model.elements.len() {
        return Err("Extra/unowned native objects are not normalized away".into());
    }
    let identities: Vec<_> = objects.iter().map(|object| object.id.clone()).collect();
    let rows: Result<Vec<_>, String> = objects.iter().enumerate().map(|(index, object)| {
        if object.type_identity != package.schema.identity(&object.class) { return Err("Native object classifier identity mismatch".into()); }
        let mut values = serde_json::to_value(&object.values).map_err(|e|e.to_string())?;
        if index == 0 { values["children"] = json!((1..objects.len()).collect::<Vec<_>>()); }
        Ok(json!({"class":format!("{NS}#//{}",object.class),"values":values,"span_bytes":[object.start,object.end]}))
    }).collect();
    Ok((json!(rows?), identities))
}

fn expected_graph(row: &Value) -> Value {
    let raw = row["raw_complete_dto"]["graph"].as_array().unwrap();
    let offsets = row["offset_comparison_only"]["graph"].as_array().unwrap();
    json!(raw.iter().zip(offsets).map(|(object, offset)| {
        let values: serde_json::Map<String, Value> = object["values"].as_object().unwrap().iter().map(|(name, field)|(name.clone(),field["eGet"].clone())).collect();
        json!({"class":object["class"],"values":values,"span_bytes":offset["semantic"]["utf8_bytes"]})
    }).collect::<Vec<_>>())
}

fn expected_features(package: &Package, row: &Value) -> Value {
    let mut features = Vec::new();
    for object in row["raw_complete_dto"]["graph"].as_array().unwrap() {
        let class = object["class"]
            .as_str()
            .unwrap()
            .rsplit("#//")
            .next()
            .unwrap();
        for field in object["feature_nodes"].as_array().unwrap() {
            let name = field["member"]["feature"].as_str().unwrap();
            let identity = &package
                .schema
                .effective_feature(class, name)
                .unwrap()
                .identity;
            for node in field["nodes"].as_array().unwrap() {
                features.push(json!({"owner":object["id"],"feature":name,"identity":identity,"semantic_utf16":[node["start"],node["end"]],"total_utf16":[node["total_start"],node["total_end"]]}));
            }
        }
    }
    features.sort_by_key(|value| serde_json::to_string(value).unwrap());
    json!(features)
}

#[test]
fn genuine_whole_iteration_28_cases_keep_positive_graphs_and_report_negative_gaps() {
    let observations: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/oracle-observations.json"
    ))
    .unwrap();
    assert_eq!(observations.len(), 28);
    let mut outcomes = Vec::new();
    let mut findings = Vec::new();
    let mut positives = 0;
    let mut negatives = 0;
    for (name, text) in [
        (
            "IterationX",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationX.xtext"),
        ),
        (
            "IterationXY",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationXY.xtext"),
        ),
        (
            "IterationNested",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationNested.xtext"),
        ),
        (
            "IterationCallerTail",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationCallerTail.xtext"),
        ),
        (
            "IterationHidden",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationHidden.xtext"),
        ),
        (
            "IterationNullable",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationNullable.xtext"),
        ),
    ] {
        let compiled = ordinary::compile(input(name, text), Profile::NativeV76);
        if let Ok(package) = &compiled {
            assert_eq!(package.schema.uri, NS);
            assert_eq!(package.grammar.as_ref().unwrap().entry, "Root");
            assert!(
                package
                    .compilation
                    .as_ref()
                    .unwrap()
                    .bundle
                    .sources
                    .iter()
                    .any(|source| source.kind == SourceKind::Xtext && source.text == text)
            );
            let children = package
                .schema
                .effective_feature("Root", "children")
                .unwrap();
            assert!(
                children.feature.reference
                    && children.feature.containment
                    && !children.feature.derived
                    && children.feature.changeable
            );
            assert_eq!(children.feature.upper, -1);
        }
        let prepared = compiled.as_ref().ok().map(|package| {
            DocumentLanguage::prepare(package, Default::default(), AnalysisLimits::workbench())
        });
        for row in observations
            .iter()
            .filter(|row| row["case"]["grammar"] == name)
        {
            let expected = &row["raw_complete_dto"];
            let accepted = expected["accepted"].as_bool().unwrap();
            if accepted {
                positives += 1;
            } else {
                negatives += 1;
            }
            let index = row["case"]["index"].as_u64().unwrap();
            // Known, explicit NativeV76 gap: nullable repetitions were never
            // supported before NativeV77 (nullable_full_repeat_v77 covers the
            // V77 admission). The exact rejection is required; anything else,
            // including unexpected acceptance, remains a finding.
            if name == "IterationNullable" && compiled.is_ok() {
                findings.push(format!(
                    "{name}/{index}: NativeV76 unexpectedly admitted the nullable repetition"
                ));
            }
            let package = match &compiled {
                Ok(package) => package,
                Err(errors) => {
                    let expected_gap = name == "IterationNullable"
                        && serde_json::to_value(errors).unwrap()
                            == json!([{"source":"memory:whole-iteration/IterationNullable.xtext","start":177,"end":242,"message":"Nullable repetitions are unsupported"}]);
                    if !expected_gap {
                        findings.push(format!(
                            "{name}/{index}: ordinary frontend rejected the unchanged genuine grammar"
                        ));
                    }
                    outcomes.push(json!({"grammar":name,"index":index,"stage":"frontend-rejected","expected_v76_gap":expected_gap,"oracle_accepted":accepted,"diagnostics":errors,"qualified":false}));
                    continue;
                }
            };
            let source = Source {
                uri: format!("memory:whole-iteration/{name}-{index}.model"),
                text: expected["text"].as_str().unwrap().into(),
            };
            let actual = evaluate_with_limits(
                package,
                std::slice::from_ref(&source),
                EvaluationLimits::workbench(),
            );
            let native_accepted = actual.diagnostics.is_empty();
            if native_accepted != accepted {
                findings.push(format!(
                    "{name}/{index}: native acceptance differs from genuine Xtext"
                ));
            }
            let mut document = Value::Null;
            if let Some(Ok(language)) = &prepared {
                match language.analyze(&source, None, EvaluationLimits::workbench()) {
                    Ok(analysis) => {
                        if accepted {
                            if analysis.valid_model.is_none()
                                || !analysis.snapshot.syntax_issues().is_empty()
                            {
                                findings
                                    .push(format!("{name}/{index}: positive document is invalid"));
                            }
                            if let Ok((_, ids)) = graph_projection(package, &actual) {
                                let mut features = Vec::new();
                                for feature in analysis.snapshot.feature_nodes() {
                                    let Some(owner) = analysis
                                        .snapshot
                                        .semantic_objects()
                                        .get(feature.owner_object)
                                    else {
                                        findings.push(format!(
                                            "{name}/{index}: feature owner index is unavailable"
                                        ));
                                        continue;
                                    };
                                    if let Some(ordinal) = ids.iter().position(|id| id == &owner.id)
                                    {
                                        let Some(node) = analysis
                                            .snapshot
                                            .document()
                                            .nodes()
                                            .get(feature.node.0)
                                        else {
                                            findings.push(format!(
                                                "{name}/{index}: feature node index is unavailable"
                                            ));
                                            continue;
                                        };
                                        features.push(json!({"owner":ordinal,"feature":feature.feature,"identity":feature.identity,"semantic_utf16":[node.range.start,node.range.end],"total_utf16":[node.total_range.start,node.total_range.end]}));
                                    } else {
                                        findings.push(format!("{name}/{index}: feature owner has no actual model object"));
                                    }
                                }
                                features.sort_by_key(|value| serde_json::to_string(value).unwrap());
                                if json!(features) != expected_features(package, row) {
                                    findings.push(format!("{name}/{index}: genuine member identity/feature node intervals differ"));
                                }
                            }
                            if analysis
                                .snapshot
                                .document()
                                .nodes()
                                .first()
                                .is_none_or(|root| {
                                    root.total_range.start != 0
                                        || root.total_range.end
                                            != source.text.encode_utf16().count()
                                })
                            {
                                findings.push(format!(
                                    "{name}/{index}: native node tree does not cover whole input"
                                ));
                            }
                        }
                        document = serde_json::to_value(analysis).unwrap();
                    }
                    Err(error) => {
                        if accepted {
                            findings.push(format!(
                                "{name}/{index}: positive document analysis returned error"
                            ));
                        }
                        document = json!({"error":error});
                    }
                }
            } else if let Some(Err(errors)) = &prepared {
                if accepted {
                    findings.push(format!(
                        "{name}/{index}: positive document language preparation rejected"
                    ));
                }
                document = json!({"preparation_errors":errors});
            }
            if accepted {
                match graph_projection(package, &actual) {
                    Ok((graph, _)) => {
                        if graph != expected_graph(row) {
                            findings.push(format!("{name}/{index}: genuine child partition/values/classifier/object byte intervals differ"));
                        }
                    }
                    Err(error) => findings.push(format!("{name}/{index}: {error}")),
                }
            }
            outcomes.push(json!({"grammar":name,"index":index,"oracle_accepted":accepted,"native":actual,"document":document,"negative_qualification_gap":(!accepted).then_some("Exact native diagnostics/recovery graph remain recorded. Whole-resource evaluation may roll back objects retained by Xtext; document recovered graph is preserved, not equated or hidden. No negative graph/diagnostic parity award."),"expected_raw_dto_case":{"grammar":name,"index":index}}));
        }
    }
    assert_eq!((positives, negatives), (20, 8));
    assert_eq!(outcomes.len(), 28);
    println!("WHOLE_ITERATION_NATIVE28 {}",serde_json::to_string(&json!({"state":"native_actual_observations","profile":Profile::NativeV76,"positive_cases":positives,"negative_cases":negatives,"findings":findings,"outcomes":outcomes})).unwrap());
    assert!(
        findings.is_empty(),
        "Genuine predicate qualification findings: {findings:?}"
    );
}

fn recovered_graph_projection(
    package: &Package,
    recovered: &intersect_text::document::RecoveredGraph,
) -> (Value, Vec<String>) {
    assert_eq!(recovered.schema_revision, package.schema.revision);
    let roots: Vec<_> = recovered
        .objects
        .iter()
        .filter(|object| object.class() == "Root")
        .collect();
    assert_eq!(roots.len(), 1, "One actual recovered Root is required");
    let root = roots[0];
    let children = root
        .values()
        .get("children")
        .and_then(Value::as_array)
        .unwrap();
    let mut objects = vec![root];
    for child in children {
        let identity = child.as_str().unwrap();
        let object = recovered
            .objects
            .iter()
            .find(|object| object.id() == identity)
            .unwrap();
        assert_eq!(object.class(), "Target");
        objects.push(object);
    }
    assert_eq!(
        objects.len(),
        recovered.objects.len(),
        "Extra/unowned recovered objects are not normalized away"
    );
    let identities: Vec<_> = objects
        .iter()
        .map(|object| object.id().to_owned())
        .collect();
    let graph: Vec<_> = objects.iter().enumerate().map(|(index, object)| {
        assert_eq!(object.type_identity(), package.schema.identity(object.class()));
        let mut values = serde_json::to_value(object.values()).unwrap();
        if index == 0 { values["children"] = json!((1..objects.len()).collect::<Vec<_>>()); }
        json!({"class":format!("{NS}#//{}",object.class()),"values":values,"span_bytes":[object.start(),object.end()]})
    }).collect();
    (json!(graph), identities)
}

#[test]
fn genuine_five_negative_documents_preserve_oracle_graphs_and_transactional_evaluation() {
    let observations: Vec<Value> = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/oracle-observations.json"
    ))
    .unwrap();
    let baseline: Value = serde_json::from_str(include_str!(
        "../../../fixtures/xtext-whole-iteration-v76/negative-native-baseline-v76.json"
    ))
    .unwrap();
    assert_eq!(baseline["outcomes"].as_array().unwrap().len(), 5);
    let mut complete = Vec::new();
    for (name, text, cases) in [
        (
            "IterationX",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationX.xtext"),
            vec![(0_u64, 0_usize)],
        ),
        (
            "IterationXY",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationXY.xtext"),
            vec![(1_u64, 3_usize), (3_u64, 7_usize)],
        ),
        (
            "IterationNested",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationNested.xtext"),
            vec![(4_u64, 7_usize)],
        ),
        (
            "IterationCallerTail",
            include_str!("../../../fixtures/xtext-whole-iteration-v76/IterationCallerTail.xtext"),
            vec![(0_u64, 0_usize)],
        ),
    ] {
        let package = ordinary::compile(input(name, text), Profile::NativeV76).unwrap();
        let language =
            DocumentLanguage::prepare(&package, Default::default(), AnalysisLimits::workbench())
                .unwrap();
        for (index, eof_byte) in cases {
            let row = observations
                .iter()
                .find(|row| row["case"]["grammar"] == name && row["case"]["index"] == index)
                .unwrap();
            let prior = baseline["outcomes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["grammar"] == name && row["index"] == index)
                .unwrap();
            assert_eq!(row["raw_complete_dto"]["accepted"], false);
            let source = Source {
                uri: format!("memory:whole-iteration/{name}-{index}.model"),
                text: row["raw_complete_dto"]["text"].as_str().unwrap().into(),
            };
            let ordinary = evaluate_with_limits(
                &package,
                std::slice::from_ref(&source),
                EvaluationLimits::workbench(),
            );
            // Raw transactional evaluation deliberately rolls the rejected resource back.
            assert!(ordinary.model.elements.is_empty());
            assert!(ordinary.references.is_empty() && ordinary.derived.is_empty());
            assert_eq!(
                serde_json::to_value(&ordinary.diagnostics).unwrap(),
                json!([{"source":source.uri,"start":eof_byte,"end":eof_byte,"message":"Expected 'x'"}])
            );
            let resource = ordinary.model.resources.get(&source.uri).unwrap();
            assert!(resource.roots.is_empty() && resource.source.is_none());
            assert_eq!(
                resource.diagnostics,
                vec![format!("{eof_byte}..{eof_byte}: Expected 'x'")]
            );
            assert_eq!(
                serde_json::to_value(&ordinary).unwrap(),
                prior["native"],
                "Whole ordinary DTO changed for {name}/{index}"
            );

            let document = language
                .analyze(&source, None, EvaluationLimits::workbench())
                .unwrap();
            assert!(document.valid_model.is_none());
            let recovered = document.recovered.as_ref().unwrap();
            let (graph, ids) = recovered_graph_projection(&package, recovered);
            assert_eq!(
                graph,
                expected_graph(row),
                "Genuine recovered values/containment/object-byte intervals differ for {name}/{index}"
            );
            let resource = recovered.resources.get(&source.uri).unwrap();
            assert_eq!(resource.roots.as_slice(), std::slice::from_ref(&ids[0]));
            assert_eq!(
                resource.source.as_ref().unwrap().revision,
                document.snapshot.document().source().sha256
            );
            assert_eq!(resource.source.as_ref().unwrap().uri, source.uri);
            let actual_diagnostics: Vec<_> = document.diagnostics.iter().map(|diagnostic| {
                assert_eq!(diagnostic.source, source.uri);
                json!({"message":diagnostic.message,"start":diagnostic.start,"end":diagnostic.end})
            }).collect();
            let expected_diagnostics: Vec<_> = row["offset_comparison_only"]["diagnostics"].as_array().unwrap().iter().map(|issue| {
                json!({"message":issue["raw"]["message"],"start":issue["span"]["utf8_bytes"][0],"end":issue["span"]["utf8_bytes"][1]})
            }).collect();
            assert_eq!(
                actual_diagnostics, expected_diagnostics,
                "Exposed genuine document errors differ for {name}/{index}"
            );
            let issues = document.snapshot.syntax_issues();
            assert_eq!(issues.len(), 1);
            assert_eq!(
                (issues[0].range.start, issues[0].range.end),
                (eof_byte, eof_byte),
                "The actual EOF issue is not clamped to its attached source node"
            );
            let mut features = Vec::new();
            for feature in document.snapshot.feature_nodes() {
                let owner = document
                    .snapshot
                    .semantic_objects()
                    .get(feature.owner_object)
                    .unwrap();
                let ordinal = ids.iter().position(|id| id == &owner.id).unwrap();
                assert_eq!(
                    &owner.classifier,
                    package.schema.declarations.get(&owner.class).unwrap()
                );
                assert_eq!(
                    feature.identity,
                    package
                        .schema
                        .effective_feature(&owner.class, &feature.feature)
                        .unwrap()
                        .identity
                );
                let node = document
                    .snapshot
                    .document()
                    .nodes()
                    .get(feature.node.0)
                    .unwrap();
                features.push(json!({"owner":ordinal,"feature":feature.feature,"identity":feature.identity,"semantic_utf16":[node.range.start,node.range.end],"total_utf16":[node.total_range.start,node.total_range.end]}));
            }
            features.sort_by_key(|value| serde_json::to_string(value).unwrap());
            assert_eq!(
                json!(features),
                expected_features(&package, row),
                "Genuine member/source identity and node intervals differ for {name}/{index}"
            );
            let root = document.snapshot.document().nodes().first().unwrap();
            assert_eq!(
                (root.total_range.start, root.total_range.end),
                (0, source.text.encode_utf16().count())
            );
            assert_eq!(
                serde_json::to_value(&document).unwrap(),
                prior["document"],
                "Whole document DTO changed for {name}/{index}"
            );
            complete.push(json!({"grammar":name,"index":index,"ordinary":ordinary,"document":document,"oracle_raw":row["raw_complete_dto"],"oracle_offsets_comparison_only":row["offset_comparison_only"]}));
        }
    }
    assert_eq!(complete.len(), 5);
    println!("WHOLE_ITERATION_NEGATIVE5 {}", serde_json::to_string(&json!({"state":"five_genuine_negative_comparisons","scope":"Full raw DTOs retained; transactional evaluation differs intentionally from recovered document analysis. Nullable3 remain frontend-rejected/unqualified; this is not full nullable/predicate support.","outcomes":complete})).unwrap());
}
