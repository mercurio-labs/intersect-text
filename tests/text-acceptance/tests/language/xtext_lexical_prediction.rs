use intersect_text::grammar;
use intersect_text::grammar::LexicalAlternative;
use intersect_text::grammar::Profile;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};
use serde_json::{Value, json};
const ECORE: &str = include_str!("../../../../fixtures/xtext/predictive-lexical/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/predictive-lexical/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/predictive-lexical/model-behavior.rcore");
fn compile(text: &str) -> Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        text,
        RCORE_BEHAVIOR,
        Profile::NativeV2,
    )
    .unwrap()
}
fn controls() -> Value {
    serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/predictive-lexical/lexer-controls.json"
    ))
    .unwrap()
}
fn source(text: &str) -> Source {
    Source {
        uri: "prediction.kerml".into(),
        text: text.into(),
    }
}
fn oracle_package() -> Package {
    let oracle = controls();
    let words = oracle["keywords"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| {
            format!(
                "'{}'",
                v.as_str()
                    .unwrap()
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let terminals = &XTEXT[XTEXT.find("\nterminal DECIMAL_VALUE").unwrap()..];
    compile(&format!(
        r#"grammar oracle.KerML hidden(WS,ML_NOTE,SL_NOTE)
import "urn:intersect:xtext:mini-kerml:predictive-lexical:1" as k
import "http://www.eclipse.org/emf/2002/Ecore" as Ecore
Model returns k::Model: 'package' name=ID '{{' '}}';
Keywords returns k::Type: {words};
{terminals}"#
    ))
}
fn label(alternative: &LexicalAlternative, oracle: &Value) -> String {
    match alternative {
        LexicalAlternative::Terminal(name) => format!("RULE_{name}"),
        LexicalAlternative::Keyword(text) => oracle["keywords"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, v)| v.as_str() == Some(text))
            .unwrap()
            .0
            .clone(),
    }
}
#[test]
fn native_generation_matches_all_pinned_lexer_predictions_and_terminal_bodies() {
    let package = oracle_package();
    let grammar = package.grammar.as_ref().unwrap();
    let prediction = grammar.lexical_prediction.as_ref().unwrap();
    let oracle = controls();
    let cases = oracle["prediction_controls"].as_array().unwrap();
    assert_eq!(cases.len(), 947);
    for case in cases {
        let text = case["source"].as_str().unwrap();
        let actual = prediction
            .predict(text, &mut None)
            .ok()
            .map(|i| label(prediction.alternative(i).unwrap(), &oracle));
        assert_eq!(actual.as_deref(), case["label"].as_str(), "{text:?}");
    }
    let cases = oracle["terminal_body_controls"].as_array().unwrap();
    assert_eq!(cases.len(), 1323);
    for case in cases {
        let name = case["rule"].as_str().unwrap();
        let text = case["source"].as_str().unwrap();
        let actual = grammar.terminals[name]
            .match_len(text)
            .ok()
            .map(|n| text[..n].encode_utf16().count());
        assert_eq!(
            actual,
            case["length_utf16"].as_u64().map(|n| n as usize),
            "{name}: {text:?}"
        );
    }
}
#[test]
fn numeric_selection_preserves_committed_failures_and_keyword_boundaries() {
    let package = oracle_package();
    let grammar = package.grammar.as_ref().unwrap();
    let oracle = controls();
    let decision = grammar.lexical_prediction.as_ref().unwrap();
    let cases = oracle["numeric_token_controls"].as_array().unwrap();
    assert_eq!(cases.len(), 15);
    for case in cases {
        let text = case["source"].as_str().unwrap();
        let result = grammar::lex_borrowed_with_prediction(
            text,
            None,
            None,
            Some(&grammar.terminals),
            grammar.hidden.as_deref(),
            Some(decision),
            &mut None,
        );
        if case["errors"] == 0 {
            let actual: Vec<Value> = result
                .unwrap()
                .iter()
                .map(|t| {
                    let alternative = decision
                        .alternative(t.lexical_alternative.unwrap())
                        .unwrap();
                    let name = match alternative {
                        LexicalAlternative::Keyword(text) => format!("'{text}'"),
                        _ => label(alternative, &oracle),
                    };
                    json!({"name":name,"text":t.text})
                })
                .collect();
            assert_eq!(actual, *case["tokens"].as_array().unwrap(), "{text}");
        } else {
            let (at, error) = result.unwrap_err();
            assert_eq!(at, 0);
            assert!(error.contains("terminal"), "{text}: {error}");
        }
    }
}
#[test]
fn hidden_notes_match_pinned_channels_and_keep_regular_comments_significant() {
    let package = compile(XTEXT);
    let grammar = package.grammar.as_ref().unwrap();
    let decision = grammar.lexical_prediction.as_ref().unwrap();
    let controls: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/predictive-lexical/hidden-controls.json"
    ))
    .unwrap();
    assert_eq!(controls.len(), 8);
    for case in controls {
        let text = case["source"].as_str().unwrap();
        let result = evaluate(&package, &[source(text)]);
        assert_eq!(
            result.diagnostics.is_empty(),
            case["accepted"].as_bool().unwrap(),
            "{text:?}: {:?}",
            result.diagnostics
        );
        let all = grammar::lex_borrowed_with_prediction(
            text,
            None,
            None,
            Some(&grammar.terminals),
            Some(&[]),
            Some(decision),
            &mut None,
        );
        match all {
            Ok(tokens) => {
                let hidden:Vec<Value>=tokens.iter().filter_map(|token|{
                    let LexicalAlternative::Terminal(name)=decision.alternative(token.lexical_alternative.unwrap()).unwrap() else { return None };
                    if !grammar.hidden.as_ref().unwrap().contains(name) { return None }
                    Some(json!({"rule":name,"text":token.text,"offset":text[..token.start].encode_utf16().count(),"length":text[token.start..token.end].encode_utf16().count()}))
                }).collect();
                assert_eq!(hidden, *case["hidden"].as_array().unwrap(), "{text:?}");
            }
            Err(_) => {
                assert_eq!(case["accepted"], false);
                assert!(
                    case["hidden"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|t| t["rule"] == "unknown")
                );
            }
        }
    }
}
#[test]
fn predictive_fixture_executes_integer_and_scalar_paths_with_exact_source_replay() {
    let package = compile(XTEXT);
    let input = [
        source(include_str!(
            "../../../../fixtures/xtext/predictive-lexical/example.kerml"
        )),
        Source {
            uri: "more.kerml".into(),
            text: include_str!("../../../../fixtures/xtext/predictive-lexical/more.kerml").into(),
        },
    ];
    let result = evaluate(&package, &input);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 5);
    assert_eq!(result.model.elements[1].values["count"], 12);
    assert_eq!(result.model.elements[1].values["magnitude"], "1e3");
    assert_eq!(result.model.elements[4].values["count"], 3);
    assert_eq!(result.model.elements[4].values["magnitude"], "12E-3");
    assert_eq!(
        result.model.elements[2].values["target"],
        "prediction.kerml#1"
    );
    assert_eq!(
        result.model.elements[0].values["documentation"],
        "/* significant documentation */"
    );
    assert_eq!(package.sources["xtext"], XTEXT);
    let replay = intersect_text::compiler::recompile(&package).unwrap();
    assert_eq!(
        serde_json::to_value(&package).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    assert_eq!(evaluate(&replay, &input).model, result.model);
    for text in [
        "package P { type Wheel count 1e3; }",
        "package P { type Wheel count 2147483648; }",
        "package P { type Wheel magnitude 1e+; }",
        "package P { type package; }",
    ] {
        let rejected = evaluate(&package, &[source(text)]);
        assert!(rejected.model.elements.is_empty(), "{text}");
        assert!(!rejected.diagnostics.is_empty(), "{text}");
    }
    assert!(
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            XTEXT,
            RCORE_BEHAVIOR,
            Profile::NativeV1
        )
        .is_err()
    );
}
#[test]
fn prediction_and_parsing_share_the_original_step_ceiling() {
    let package = compile(XTEXT);
    let input = [source("package P { type Wheel count 12 magnitude 1e3; }")];
    let mut limits = EvaluationLimits::native();
    let (mut low, mut high) = (0, 100_000);
    limits.steps_per_resource = Some(high);
    assert!(
        evaluate_with_limits(&package, &input, limits)
            .diagnostics
            .is_empty()
    );
    while low + 1 < high {
        let mid = (low + high) / 2;
        limits.steps_per_resource = Some(mid);
        if evaluate_with_limits(&package, &input, limits)
            .diagnostics
            .is_empty()
        {
            high = mid
        } else {
            low = mid
        }
    }
    limits.steps_per_resource = Some(high);
    assert!(
        evaluate_with_limits(&package, &input, limits)
            .diagnostics
            .is_empty()
    );
    limits.steps_per_resource = Some(high - 1);
    let rejected = evaluate_with_limits(&package, &input, limits);
    assert!(rejected.model.elements.is_empty());
    assert_eq!(rejected.diagnostics.len(), 1);
    assert_eq!(rejected.diagnostics[0].message, "Execution budget exceeded");
    let decision = package
        .grammar
        .as_ref()
        .unwrap()
        .lexical_prediction
        .as_ref()
        .unwrap();
    assert!(
        decision
            .predict("package P {}", &mut Some(0))
            .unwrap_err()
            .contains("step budget")
    );
}

#[test]
fn keyword_text_cannot_consume_a_differently_selected_terminal() {
    let header = "grammar example.KeywordIdentity hidden()\nimport \"urn:intersect:xtext:mini-kerml:predictive-lexical:1\" as k\n";
    let terminal = "\nterminal TAG: 'f' -> 'oo';";
    let keyword = compile(&format!(
        "{header}Model returns k::Model: 'foo' 'x';{terminal}"
    ));
    let result = evaluate(&keyword, &[source("foox")]);
    assert!(result.model.elements.is_empty());
    assert_eq!(result.diagnostics.len(), 1);
    assert!(result.diagnostics[0].message.contains("Expected 'foo'"));
    let assigned = compile(&format!(
        "{header}Model returns k::Model: name=TAG 'x';{terminal}"
    ));
    let result = evaluate(&assigned, &[source("foox")]);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[0].values["name"], "foo");
}

#[test]
fn generated_prediction_states_have_an_explicit_compilation_ceiling() {
    let input = format!("{XTEXT}\nLarge returns k::Type: '{}';", ":".repeat(10_001));
    let error =
        intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
            ECORE,
            &input,
            RCORE_BEHAVIOR,
            Profile::NativeV2,
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("prediction budget"));
}

#[test]
fn explicit_terminal_return_types_take_precedence_over_builtin_names() {
    let text = r#"grammar example.DeclaredTypes hidden(WS)
import "urn:intersect:xtext:mini-kerml:predictive-lexical:1" as k
import "http://www.eclipse.org/emf/2002/Ecore" as Ecore
Model returns k::Model: 'package' name=INT '{' members+=Type* '}';
Type returns k::Type: 'type' name=INT 'count' count=STRING ';';
terminal INT: ('a'..'z')+;
terminal STRING returns Ecore::EInt: ('0'..'9')+;
terminal WS: ' '+;
"#;
    let result = evaluate(
        &compile(text),
        &[source("package p { type wheel count 12; }")],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements[0].values["name"], "p");
    assert_eq!(result.model.elements[1].values["name"], "wheel");
    assert_eq!(result.model.elements[1].values["count"], 12);
}
