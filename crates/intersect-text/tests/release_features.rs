//! Handwritten (not generated): the Release 1 Xtext feature suite (gate G1,
//! decision D1). One small language that is not KerML-shaped
//! (`fixtures/xtext/release-features`: a parent formula grammar and a
//! spreadsheet grammar extending it) uses every Xtext feature the three pinned
//! grammars use, in the shapes they use them. Each test runs the production
//! path the KerML and SysML loads take: NativeV80 guarded draft preparation
//! (`InputCheckedCompletionScopes`), the completed migration origin,
//! `OpenDraftCapability` + `DraftLanguage::parse_runtime_into` into a
//! `FrozenStore`, then linking with Rcore `bind scope` bindings. The feature
//! list and results are in docs/xtext-release-feature-list.md.
use intersect_text::{
    EvaluationLimits, Source,
    document::AnalysisLimits,
    draft::{DraftLanguage, GrammarPolicy, OpenDraftCapability},
    ordinary,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};
use {
    intersect_model::core::frozen::FeatureId, intersect_model::core::frozen::FrozenError,
    intersect_model::core::frozen::FrozenStore, intersect_model::core::frozen::LinkView,
    intersect_model::core::frozen::Metamodel, intersect_model::core::frozen::ObjId,
    intersect_model::core::model::Element, intersect_model::core::model::Model,
    intersect_model::core::query::QueryBudget, intersect_model::core::storage,
    intersect_model::definitions::bundle::DefinitionBundle,
    intersect_model::definitions::bundle::DefinitionSource,
    intersect_model::definitions::bundle::SourceKind,
    intersect_model::definitions::ecore::ParseLimits,
    intersect_model::definitions::rcore_behavior::ir_v2::Limits,
    intersect_model::definitions::rcore_scope::compile_bindings_with_source_limits,
    intersect_model::definitions::rcore_scope::source_scope_bindings,
    intersect_model::frozen_scopes::RcoreScopes, intersect_text::grammar::Profile,
    intersect_text::grammar::RuntimeGuardedPolicy, intersect_text::package::BundleCompilation,
    intersect_text::package::CompilationSourcePolicy,
    intersect_text::preparation::SharedDraftRuntimeLanguageDefinition,
    intersect_text::preparation::SourceBoundDraftRuntimeDefinition,
};

const ECORE: &str = include_str!("../../../fixtures/xtext/release-features/sheets.ecore");
const FORMULAS: &str = include_str!("../../../fixtures/xtext/release-features/Formulas.xtext");
const SHEETS: &str = include_str!("../../../fixtures/xtext/release-features/Sheets.xtext");
const SCOPING: &str = include_str!("../../../fixtures/xtext/release-features/sheets-scoping.rcore");
const NS: &str = "urn:intersect:fixture:release-sheets:1";
const LIMITS: ParseLimits = ParseLimits {
    source_bytes: 1_000_000,
};

/// Prepare the fixture language with `sheets` as the entry grammar text (the
/// fixture's, or a copy with one construct changed for a negative control),
/// exactly as the KerML and SysML owners are prepared.
fn bound_with(sheets: &str) -> Result<SourceBoundDraftRuntimeDefinition, String> {
    let model = DefinitionSource::new("memory:release/sheets.ecore", SourceKind::Ecore, ECORE);
    let mut parent =
        DefinitionSource::new("memory:release/Formulas.xtext", SourceKind::Xtext, FORMULAS);
    parent.dependencies.push(model.id.clone());
    let mut entry = DefinitionSource::new("memory:release/Sheets.xtext", SourceKind::Xtext, sheets);
    entry
        .dependencies
        .extend([model.id.clone(), parent.id.clone()]);
    let mut rules = DefinitionSource::new(
        "memory:release/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package releaseRules version \"1\";",
    );
    rules.dependencies.push(model.id.clone());
    let original = BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: NS.into(),
        xtext: Some(entry.id.clone()),
        rcore_behavior: rules.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model.clone(), parent, entry, rules],
            namespaces: vec![],
        },
    };
    let completed = intersect_model::migration::migrate_ecore_bundle_completed_with_limits(
        original.bundle.clone(),
        &[(model.id, "memory:release/sheets.rcore".into())].into(),
        ParseLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    let policy = CompilationSourcePolicy::default();
    let owner = ordinary::prepare_with_runtime_guarded_policy(
        intersect_text::migration::remap_compilation(completed.report(), &original)?,
        Profile::NativeV80,
        &[],
        None,
        policy,
        policy,
        RuntimeGuardedPolicy::InputCheckedCompletionScopes,
        None,
    )
    .map_err(|e| format!("{e:?}"))?;
    SharedDraftRuntimeLanguageDefinition::new(Arc::new(owner))
        .bind_completed_origin(completed, policy)
        .map_err(|e| format!("{e:?}"))
}

/// One parsed document: its diagnostics and the constructed (unlinked) model.
struct Parsed {
    diagnostics: Vec<String>,
    proxies: usize,
    model: Model,
}

/// The constructed documents, the linked and validated model (or the linker's
/// diagnostics), and every link answer recorded as
/// (owner path, feature, spelling) -> target paths.
struct Loaded {
    parsed: Vec<Parsed>,
    linked: Result<Model, String>,
    answers: BTreeMap<(String, String, String), Vec<String>>,
}

/// Parse `documents` into one `FrozenStore` with the fixture language and,
/// when every document parsed cleanly, link and validate it.
fn load(documents: &[(&str, &str)]) -> Result<Loaded, String> {
    load_with(SHEETS, documents)
}

fn load_with(grammar: &str, documents: &[(&str, &str)]) -> Result<Loaded, String> {
    let bound = bound_with(grammar)?;
    let definition = bound
        .prepare(CompilationSourcePolicy::default())
        .map_err(|e| format!("{e:?}"))?;
    let inner = GrammarPolicy::new(&definition);
    let capability = OpenDraftCapability::new(&definition, &inner)?;
    let language = DraftLanguage::bind(&definition, &capability, AnalysisLimits::workbench())?;
    let mm = Arc::new(Metamodel::freeze(definition.schema()).map_err(|e| e.to_string())?);
    let mut store = FrozenStore::new(mm.clone());
    let mut parsed = Vec::new();
    for (uri, text) in documents {
        // Each document alone, to observe exactly what it constructs.
        let mut alone = FrozenStore::new(mm.clone());
        let source = Source {
            uri: (*uri).into(),
            text: (*text).into(),
        };
        let (evaluation, proxies, _) = language.parse_runtime_into(
            &source,
            EvaluationLimits::native(),
            &mut alone,
            &mut QueryBudget::new(usize::MAX),
        )?;
        parsed.push(Parsed {
            diagnostics: evaluation
                .diagnostics
                .iter()
                .map(|d| format!("{}..{}: {}", d.start, d.end, d.message))
                .collect(),
            proxies,
            model: storage::export(&alone)?,
        });
        language.parse_runtime_into(
            &source,
            EvaluationLimits::native(),
            &mut store,
            &mut QueryBudget::new(usize::MAX),
        )?;
    }
    if parsed.iter().any(|p| !p.diagnostics.is_empty()) {
        return Ok(Loaded {
            parsed,
            linked: Err("not linked: parse diagnostics".into()),
            answers: BTreeMap::new(),
        });
    }
    let structure = definition
        .sources()
        .sources
        .iter()
        .find(|source| source.kind == SourceKind::Rcore)
        .ok_or("Missing migrated structure")?
        .clone();
    let mut scoping = DefinitionSource::new(
        "memory:release/sheets-scoping.rcore",
        SourceKind::RcoreBehavior,
        SCOPING,
    );
    scoping.dependencies.push(structure.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![structure, scoping.clone()],
        namespaces: vec![],
    };
    let declarations: Vec<_> = source_scope_bindings(&scoping, LIMITS)
        .map_err(|e| format!("{e:?}"))?
        .iter()
        .map(|b| (b.declaration, false))
        .collect();
    let scopes = compile_bindings_with_source_limits(
        &bundle,
        &scoping.id,
        &declarations,
        &Limits::default(),
        LIMITS,
    )
    .map_err(|e| format!("{e:?}"))?;
    let resolvers = RcoreScopes::new(&mm, &bundle, &scopes).map_err(|e| e.to_string())?;
    let recorded = RefCell::new(BTreeMap::new());
    let resolve = |view: &LinkView<'_>,
                   owner: ObjId,
                   feature: FeatureId,
                   spelling: &str|
     -> Result<Option<ObjId>, FrozenError> {
        let found = resolvers.candidates(view, owner, feature, spelling)?;
        recorded.borrow_mut().insert(
            (
                view.identity(owner).unwrap_or("").to_owned(),
                mm.feature_name(feature).to_owned(),
                spelling.to_owned(),
            ),
            found
                .iter()
                .map(|o| view.identity(*o).unwrap_or("").to_owned())
                .collect::<Vec<_>>(),
        );
        Ok(match found.as_slice() {
            [one] => Some(*one),
            _ => None,
        })
    };
    let linked = match store
        .into_builder()
        .map_err(|e| e.to_string())?
        .link(&resolve)
    {
        Ok(linked) => linked,
        Err(diagnostics) => {
            return Ok(Loaded {
                parsed,
                linked: Err(format!("{diagnostics:?}")),
                answers: BTreeMap::new(),
            });
        }
    };
    let linked = linked
        .validate()
        .map_err(|e| format!("{e:?}"))?
        .to_model()
        .map_err(|e| e.to_string())?;
    let path = |id: &str| path_of(&linked, id);
    let answers = recorded
        .into_inner()
        .into_iter()
        .map(|((owner, feature, spelling), found)| {
            (
                (path(&owner), feature, spelling),
                found.iter().map(|f| path(f)).collect(),
            )
        })
        .collect();
    Ok(Loaded {
        parsed,
        linked: Ok(linked),
        answers,
    })
}

/// `sheet.cell`-style path of an element: the names of its named ancestors
/// and itself; unnamed elements show their class.
fn path_of(model: &Model, id: &str) -> String {
    let by_id: BTreeMap<&str, &Element> =
        model.elements.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    for element in &model.elements {
        for feature in [
            "sheets", "cells", "imports", "formula", "operands", "operand",
        ] {
            let children = match element.values.get(feature) {
                Some(Value::Array(items)) => items.iter().filter_map(Value::as_str).collect(),
                Some(Value::String(one)) => vec![one.as_str()],
                _ => vec![],
            };
            for child in children {
                parent.insert(child, element.id.as_str());
            }
        }
    }
    let mut parts = Vec::new();
    let mut at = Some(id);
    while let Some(current) = at {
        if let Some(element) = by_id.get(current) {
            match element.values.get("name").and_then(Value::as_str) {
                Some(name) => parts.push(name.to_owned()),
                None if parts.is_empty() => parts.push(element.class.clone()),
                None => {}
            }
        }
        at = parent.get(current).copied();
    }
    parts.reverse();
    parts.join(".")
}

fn compact(model: &Model) -> Vec<String> {
    model
        .elements
        .iter()
        .map(|e| {
            let values: BTreeMap<_, _> = e
                .values
                .iter()
                .filter(|(_, v)| !v.is_null() && *v != &json!([]))
                .map(|(k, v)| {
                    let v = match v.get("literal") {
                        Some(l) => l["path"][1].clone(),
                        None => v.clone(),
                    };
                    (k.clone(), v)
                })
                .collect();
            format!(
                "{} {} {}..{} {}",
                e.id,
                e.class,
                e.start,
                e.end,
                json!(values)
            )
        })
        .collect()
}

/// The one document of `loaded`, which must parse without diagnostics.
fn clean(loaded: &Loaded) -> Vec<String> {
    assert_eq!(loaded.parsed.len(), 1);
    let parsed = &loaded.parsed[0];
    assert_eq!(parsed.diagnostics, Vec::<String>::new());
    compact(&parsed.model)
}

/// Parse `text` alone and return its diagnostics; a rejected document must
/// construct nothing.
fn rejected(grammar: &str, text: &str) -> Result<Vec<String>, String> {
    let loaded = load_with(grammar, &[("p.sheets", text)])?;
    let parsed = &loaded.parsed[0];
    assert!(
        parsed.model.elements.is_empty(),
        "{text}: {:?}",
        compact(&parsed.model)
    );
    assert!(loaded.linked.is_err());
    Ok(parsed.diagnostics.clone())
}

/// Features 1-4 and 15: grammar inheritance (`with`), metamodel imports,
/// grammar-level hidden terminals inherited by the child grammar, calls to
/// inherited rules and an `@Override` of an inherited rule.
#[test]
fn inheritance_imports_and_inherited_hidden_terminals_on_the_production_path() -> Result<(), String>
{
    // Comments and whitespace of the parent's hidden terminals (block
    // comment, line comment, tab, CRLF) are skipped by the child grammar;
    // spans start at the first visible token.
    let loaded = load(&[("p.sheets", "/* c */ sheet\tS // x\r\n{ cell a = empty; }")])?;
    assert_eq!(
        clean(&loaded),
        [
            r#"p.sheets#0 Workbook 8..41 {"sheets":["p.sheets#1"]}"#,
            r#"p.sheets#1 Sheet 8..41 {"access":0,"cells":["p.sheets#2"],"locked":false,"name":"S"}"#,
            r#"p.sheets#2 Cell 24..39 {"formula":"p.sheets#3","kind":0,"name":"a"}"#,
            // `empty` exists only in the child's override of the parent's
            // `Literal`, called from the parent's `Primary`.
            r#"p.sheets#3 EmptyLit 33..38 {}"#,
        ]
    );
    // Without the override the inherited rule is used and `empty` is not a
    // formula.
    let original = "@Override\nLiteral returns sh::Expr :\n\tNumberLit | RealLit | BoolLit | TextLit | EmptyLit\n;";
    assert!(SHEETS.contains(original));
    assert_eq!(
        rejected(&SHEETS.replace(original, ""), "sheet S { cell a = empty; }")?,
        ["19..24: No viable predicted alternative"]
    );
    // A missing parent grammar and an override of a rule the parent does not
    // have are rejected when the language is prepared.
    let missing = SHEETS.replace(
        "with fixture.release.Formulas",
        "with fixture.release.Missing",
    );
    let error = bound_with(&missing)
        .err()
        .ok_or("Missing parent accepted")?;
    assert!(
        error.contains("Missing exact parent grammar dependency"),
        "{error}"
    );
    let orphan = SHEETS.replace(
        "@Override\nLiteral returns",
        "@Override\nNotInParent returns",
    );
    let error = bound_with(&orphan)
        .err()
        .ok_or("Orphan override accepted")?;
    assert!(error.contains("NotInParent"), "{error}");
    Ok(())
}

/// Features 5, 6, 8 and 12: data type rules (composed of keywords,
/// terminals and other data type rules), data type rules returning EDouble
/// and EBoolean, terminal rules (ranges, negation, until, terminal calls,
/// escapes, `returns Ecore::EInt`) and assigned calls of each rule kind.
#[test]
fn data_type_rules_terminal_rules_and_value_conversion() -> Result<(), String> {
    let text = "sheet S { cell a = .5; cell b = 2.25; cell c = 1e3; cell d = 42; cell e = \"a\\\"b\"; cell 'my cell' = 'my cell'; cell f = false; }";
    assert_eq!(
        clean(&load(&[("p.sheets", text)])?),
        [
            r#"p.sheets#0 Workbook 0..127 {"sheets":["p.sheets#1"]}"#,
            r#"p.sheets#1 Sheet 0..127 {"access":0,"cells":["p.sheets#2","p.sheets#4","p.sheets#6","p.sheets#8","p.sheets#10","p.sheets#12","p.sheets#14"],"locked":false,"name":"S"}"#,
            r#"p.sheets#2 Cell 10..22 {"formula":"p.sheets#3","kind":0,"name":"a"}"#,
            // Real returns Ecore::EDouble: 0.5, 2.25 and 1000.0 (EXP_VALUE
            // calls the INT_VALUE terminal).
            r#"p.sheets#3 RealLit 19..21 {"value":{"bits":"3fe0000000000000","kind":"float64"}}"#,
            r#"p.sheets#4 Cell 23..37 {"formula":"p.sheets#5","kind":0,"name":"b"}"#,
            r#"p.sheets#5 RealLit 32..36 {"value":{"bits":"4002000000000000","kind":"float64"}}"#,
            r#"p.sheets#6 Cell 38..51 {"formula":"p.sheets#7","kind":0,"name":"c"}"#,
            r#"p.sheets#7 RealLit 47..50 {"value":{"bits":"408f400000000000","kind":"float64"}}"#,
            r#"p.sheets#8 Cell 52..64 {"formula":"p.sheets#9","kind":0,"name":"d"}"#,
            // terminal INT_VALUE returns Ecore::EInt.
            r#"p.sheets#9 NumberLit 61..63 {"value":42}"#,
            r#"p.sheets#10 Cell 65..81 {"formula":"p.sheets#11","kind":0,"name":"e"}"#,
            // Xtext registers default converters only for ID, STRING and INT:
            // TEXT and QUOTED keep their token text, escapes included.
            r#"p.sheets#11 TextLit 74..80 {"value":"\"a\\\"b\""}"#,
            r#"p.sheets#12 Cell 82..109 {"formula":"p.sheets#13","kind":0,"name":"'my cell'"}"#,
            r#"p.sheets#13 CellRef 99..108 {"cell":"?'my cell'"}"#,
            r#"p.sheets#14 Cell 110..125 {"formula":"p.sheets#15","kind":0,"name":"f"}"#,
            // Bool returns Ecore::EBoolean.
            r#"p.sheets#15 BoolLit 119..124 {"value":false}"#,
        ]
    );
    // A data type rule's value is its token text with each run of hidden
    // tokens between tokens collapsed to one space, as Xtext's
    // NodeModelUtils.getTokenText.
    assert_eq!(
        clean(&load(&[("p.sheets", "use  Rates /* c */ .\r\n * ;")])?),
        [
            r#"p.sheets#0 Workbook 0..26 {"imports":["p.sheets#1"]}"#,
            r#"p.sheets#1 Import 0..26 {"importedNamespace":"Rates . *"}"#,
        ]
    );
    for (text, diagnostic) in [
        // Real needs a digit after '.'.
        (
            "sheet S { cell a = 1.; }",
            "21..22: No viable predicted alternative",
        ),
        // TEXT is not terminated: no terminal matches.
        (
            "sheet S { cell a = \"open; }",
            "19..20: No viable lexical alternative",
        ),
        // INT_VALUE returns EInt: conversion fails outside its range.
        (
            "sheet S { cell a = 99999999999; }",
            "30..31: INT outside EInt range: EInt integer out of range: \"99999999999\"",
        ),
    ] {
        assert_eq!(rejected(SHEETS, text)?, [diagnostic], "{text}");
    }
    Ok(())
}

/// Features 7, 10, 11 and 12: enum rules with keyword literals, keywords
/// reserved against identifiers, `=`, `+=` and `?=` assignments and an
/// assigned keyword alternative.
#[test]
fn enum_rules_keywords_and_assignment_operators() -> Result<(), String> {
    // Enum values show the literal's index in its Ecore enum.
    assert_eq!(
        clean(&load(&[(
            "p.sheets",
            "private locked sheet S { cell a : out [mm]; value b; }"
        )])?),
        [
            r#"p.sheets#0 Workbook 0..54 {"sheets":["p.sheets#1"]}"#,
            r#"p.sheets#1 Sheet 0..54 {"access":1,"cells":["p.sheets#2","p.sheets#3"],"locked":true,"name":"S"}"#,
            r#"p.sheets#2 Cell 25..43 {"kind":1,"name":"a","unit":"mm"}"#,
            // Absent `?=`, enum and keyword assignments keep the defaults.
            r#"p.sheets#3 Cell 44..52 {"kind":0,"name":"b"}"#,
        ]
    );
    for (text, diagnostic) in [
        // Not a Kind literal.
        (
            "sheet S { cell a : sideways; }",
            "19..27: No viable predicted alternative",
        ),
        // Not one of the assigned keyword alternatives.
        (
            "sheet S { cell a [inch]; }",
            "18..22: No viable predicted alternative",
        ),
        // A keyword is not an identifier.
        (
            "sheet S { cell cell; }",
            "15..19: No viable predicted alternative",
        ),
    ] {
        assert_eq!(rejected(SHEETS, text)?, [diagnostic], "{text}");
    }
    Ok(())
}

/// Features 9, 10 and 18: parser fragments setting features of their caller
/// and a first-token predicate `( -> Fragment )+` in the shape of KerML's
/// FeatureSpecializationPart; punctuation keywords without separators and an
/// identifier that starts with a keyword.
#[test]
fn fragments_and_first_token_predicates_fill_the_caller() -> Result<(), String> {
    let text =
        "sheet S { cell a : in <- b, c [kg] : out; cell b [kg] <- a; cell c<-a; cell cellar; }";
    let loaded = load(&[("p.sheets", text)])?;
    assert_eq!(loaded.parsed[0].proxies, 4);
    assert_eq!(
        clean(&loaded),
        [
            r#"p.sheets#0 Workbook 0..85 {"sheets":["p.sheets#1"]}"#,
            r#"p.sheets#1 Sheet 0..85 {"access":0,"cells":["p.sheets#2","p.sheets#3","p.sheets#4","p.sheets#5"],"locked":false,"name":"S"}"#,
            // Fragments construct no objects; `kind` is assigned twice and
            // keeps the last value, as in Xtext.
            r#"p.sheets#2 Cell 10..41 {"kind":1,"name":"a","sources":["?b","?c"],"unit":"kg"}"#,
            r#"p.sheets#3 Cell 42..59 {"kind":0,"name":"b","sources":["?a"],"unit":"kg"}"#,
            r#"p.sheets#4 Cell 60..70 {"kind":0,"name":"c","sources":["?a"]}"#,
            r#"p.sheets#5 Cell 71..83 {"kind":0,"name":"cellar"}"#,
        ]
    );
    // A unit is allowed once.
    assert_eq!(
        rejected(SHEETS, "sheet S { cell a [kg] [cm]; }")?,
        ["22..23: Expected ';'"]
    );
    Ok(())
}

/// Features 14, 15 and 16: `{T.f+=current}` tree-rewrite actions building
/// left-associative trees, `{T}` actions, unassigned rule calls (parentheses
/// and delegation) and nested cardinalities.
#[test]
fn tree_rewrite_actions_build_left_associative_trees() -> Result<(), String> {
    let text = "sheet S { cell a = 1 - 2 - 3; cell b = (1 + 2) * 3 / 4; cell c = - - 1; }";
    assert_eq!(
        clean(&load(&[("p.sheets", text)])?),
        [
            r#"p.sheets#0 Workbook 0..73 {"sheets":["p.sheets#1"]}"#,
            r#"p.sheets#1 Sheet 0..73 {"access":0,"cells":["p.sheets#2","p.sheets#8","p.sheets#16"],"locked":false,"name":"S"}"#,
            // (1 - 2) - 3
            r#"p.sheets#2 Cell 10..29 {"formula":"p.sheets#6","kind":0,"name":"a"}"#,
            r#"p.sheets#3 NumberLit 19..20 {"value":1}"#,
            r#"p.sheets#4 Binary 19..24 {"op":"-","operands":["p.sheets#3","p.sheets#5"]}"#,
            r#"p.sheets#5 NumberLit 23..24 {"value":2}"#,
            r#"p.sheets#6 Binary 19..28 {"op":"-","operands":["p.sheets#4","p.sheets#7"]}"#,
            r#"p.sheets#7 NumberLit 27..28 {"value":3}"#,
            // ((1 + 2) * 3) / 4: the parenthesized Sum is returned by Primary.
            r#"p.sheets#8 Cell 30..55 {"formula":"p.sheets#14","kind":0,"name":"b"}"#,
            r#"p.sheets#9 NumberLit 40..41 {"value":1}"#,
            r#"p.sheets#10 Binary 40..45 {"op":"+","operands":["p.sheets#9","p.sheets#11"]}"#,
            r#"p.sheets#11 NumberLit 44..45 {"value":2}"#,
            r#"p.sheets#12 Binary 39..50 {"op":"*","operands":["p.sheets#10","p.sheets#13"]}"#,
            r#"p.sheets#13 NumberLit 49..50 {"value":3}"#,
            r#"p.sheets#14 Binary 39..54 {"op":"/","operands":["p.sheets#12","p.sheets#15"]}"#,
            r#"p.sheets#15 NumberLit 53..54 {"value":4}"#,
            // -(-(1)) through the {sh::Negate} action.
            r#"p.sheets#16 Cell 56..71 {"formula":"p.sheets#17","kind":0,"name":"c"}"#,
            r#"p.sheets#17 Negate 65..70 {"operand":"p.sheets#18"}"#,
            r#"p.sheets#18 Negate 67..70 {"operand":"p.sheets#19"}"#,
            r#"p.sheets#19 NumberLit 69..70 {"value":1}"#,
        ]
    );
    // `{sh::Workbook}` constructs the root of an empty document.
    assert_eq!(
        clean(&load(&[("p.sheets", "")])?),
        [r#"p.sheets#0 Workbook 0..0 {}"#]
    );
    assert_eq!(
        rejected(SHEETS, "sheet S { cell a = 1 +; }")?,
        ["22..23: No viable predicted alternative"]
    );
    Ok(())
}

/// Feature 17 and 19: syntactic predicates `=> imports += Import` and
/// `'{' => Fragment* '}'` in the shape of SysML's body items; syntax errors
/// construct nothing.
#[test]
fn syntactic_predicates_select_the_predicated_alternative() -> Result<(), String> {
    let text = "sheet S { use T.*; cell a; use U.u; } sheet T; sheet U { }";
    assert_eq!(
        clean(&load(&[("p.sheets", text)])?),
        [
            r#"p.sheets#0 Workbook 0..58 {"sheets":["p.sheets#1","p.sheets#5","p.sheets#6"]}"#,
            r#"p.sheets#1 Sheet 0..37 {"access":0,"cells":["p.sheets#3"],"imports":["p.sheets#2","p.sheets#4"],"locked":false,"name":"S"}"#,
            r#"p.sheets#2 Import 10..18 {"importedNamespace":"T.*"}"#,
            r#"p.sheets#3 Cell 19..26 {"kind":0,"name":"a"}"#,
            r#"p.sheets#4 Import 27..35 {"importedNamespace":"U.u"}"#,
            r#"p.sheets#5 Sheet 38..46 {"access":0,"locked":false,"name":"T"}"#,
            r#"p.sheets#6 Sheet 47..58 {"access":0,"locked":false,"name":"U"}"#,
        ]
    );
    for (text, diagnostic) in [
        (
            "sheet S { cell }",
            "15..16: No viable predicted alternative",
        ),
        (
            "sheet S { cell a = 1; } garbage",
            "24..31: Unexpected trailing input",
        ),
    ] {
        assert_eq!(rejected(SHEETS, text)?, [diagnostic], "{text}");
    }
    Ok(())
}

/// Features 13, 21 and 22: cross references `[T|Path]` written with a data
/// type rule, linked across resources through Rcore `bind scope` bindings:
/// local names, a sheet-level single import, a workbook wildcard import,
/// qualified names and a quoted name.
#[test]
fn cross_references_link_through_rcore_scopes() -> Result<(), String> {
    let rates = (
        "rates.sheets",
        "sheet Rates { cell r = 1; cell 'tax rate' = 2; }\nsheet Other { cell r = 3; cell only = 4; }",
    );
    let main = (
        "main.sheets",
        "use Rates.*;\nsheet Main extends Rates {\n  use Other.only;\n  cell a <- r, 'tax rate' = a + only + Other.r + Rates.r;\n}",
    );
    let loaded = load(&[rates, main])?;
    assert_eq!(loaded.parsed[0].diagnostics, Vec::<String>::new());
    assert_eq!(loaded.parsed[0].proxies, 0);
    assert_eq!(loaded.parsed[1].diagnostics, Vec::<String>::new());
    assert_eq!(loaded.parsed[1].proxies, 7);
    let answer = |owner: &str, feature: &str, spelling: &str, target: &str| {
        (
            (owner.to_owned(), feature.to_owned(), spelling.to_owned()),
            vec![target.to_owned()],
        )
    };
    let expected: BTreeMap<_, _> = [
        // By name among every resource.
        answer("Main", "base", "Rates", "Rates"),
        // Through the workbook's `use Rates.*`.
        answer("Main.a", "sources", "r", "Rates.r"),
        answer("Main.a", "sources", "'tax rate'", "Rates.'tax rate'"),
        // A cell of the enclosing sheet.
        answer("Main.a.CellRef", "cell", "a", "Main.a"),
        // Through the sheet's `use Other.only`.
        answer("Main.a.CellRef", "cell", "only", "Other.only"),
        // Qualified.
        answer("Main.a.CellRef", "cell", "Other.r", "Other.r"),
        answer("Main.a.CellRef", "cell", "Rates.r", "Rates.r"),
    ]
    .into_iter()
    .collect();
    assert_eq!(loaded.answers, expected);
    // The linked model holds the targets, not the spellings.
    let model = loaded.linked?;
    let sources = model
        .elements
        .iter()
        .find(|e| e.class == "Cell" && e.values.get("name") == Some(&json!("a")))
        .ok_or("Missing cell a")?
        .values
        .get("sources")
        .cloned()
        .ok_or("Missing sources")?;
    let targets: Vec<String> = sources
        .as_array()
        .ok_or("sources is not a list")?
        .iter()
        .map(|id| path_of(&model, id.as_str().unwrap_or("")))
        .collect();
    assert_eq!(targets, ["Rates.r", "Rates.'tax rate'"]);
    Ok(())
}

/// Features 21 and 22, negative: a name no scope finds and a name two
/// wildcard imports both find are link errors, not first matches.
#[test]
fn unresolved_and_ambiguous_references_are_link_errors() -> Result<(), String> {
    let rates = (
        "rates.sheets",
        "sheet Rates { cell r = 1; }\nsheet Other { cell r = 3; }",
    );
    for (text, error) in [
        (
            "sheet M { cell a = missing; }",
            r#"Diagnostics([Unresolved { object: "bad.sheets#3", feature: "cell", spelling: "missing" }])"#,
        ),
        (
            "use Rates.*; use Other.*; sheet M { cell a = r; }",
            r#"Diagnostics([Unresolved { object: "bad.sheets#5", feature: "cell", spelling: "r" }])"#,
        ),
    ] {
        let loaded = load(&[rates, ("bad.sheets", text)])?;
        assert!(loaded.parsed.iter().all(|p| p.diagnostics.is_empty()));
        assert_eq!(loaded.linked.err().as_deref(), Some(error), "{text}");
    }
    Ok(())
}
