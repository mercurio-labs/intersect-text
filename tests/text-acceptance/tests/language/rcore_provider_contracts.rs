use intersect_text::compiler;
use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};

const MODEL: &str = r#"rcore 1; package providers version "1";
metamodel "urn:providers" prefix p {
  class Item {
    attr name: String [0,1];
    attr origin: String [0,1];
    attr exposed: String [0,1] derived;
  }
}"#;
const HEADER: &str =
    "rcore 1; package contracts version \"1\"; import model \"urn:providers\" as p;";
const GRAMMAR: &str = r#"grammar test.Providers with org.eclipse.xtext.common.Terminals
import "urn:providers" as p
Item returns p::Item: 'item' name=ID;"#;

#[test]
fn old_json_behavior_is_rejected_by_direct_and_retained_bundle_compilation() {
    use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
    let old = r#"{"version":1,"scope":"unique_names","derived":[],"validations":[]}"#;
    assert!(compiler::compile_rcore_language(MODEL, GRAMMAR, old).is_err());
    let package = compiler::compile_rcore_language(
        MODEL,
        GRAMMAR,
        &format!("{HEADER} projection p.Item.exposed from origin;"),
    )
    .unwrap();
    let mut input = package.compilation.unwrap();
    let index = input
        .bundle
        .sources
        .iter()
        .position(|s| s.id == input.rcore_behavior)
        .unwrap();
    let mut source = DefinitionSource::new(
        input.rcore_behavior.uri.clone(),
        SourceKind::RcoreBehavior,
        old,
    );
    source.dependencies = input.bundle.sources[index].dependencies.clone();
    input.rcore_behavior = source.id.clone();
    input.bundle.sources[index] = source;
    input.bundle.validate().unwrap();
    let errors = compiler::compile_bundle(input).unwrap_err();
    assert!(errors.iter().any(|error| error.source.contains("behavior")));
}

#[test]
fn declared_count_and_incoming_contracts_preserve_original_feature_types() {
    let model = include_str!("../../../../fixtures/library/library-structure.rcore");
    let grammar = include_str!("../../../../fixtures/library/library.xtext");
    let behavior = r#"rcore 1; package libraryProviders version "1";
import model "urn:intersect:example:library:1" as lib;
derive lib.Library.copyCount using count(copies);
derive lib.BookCopy.available using noIncoming(lib.Loan.copy);
validate lib.Loan.singleLoan(self) {
  require size(incoming::<lib.Loan>(self.copy, "copy")) <= 1
    code "singleLoan" message "A book copy cannot have more than one active loan.";
}"#;
    let package = compiler::compile_rcore_language(model, grammar, behavior).unwrap();
    assert_eq!(
        package.schema.classes["Library"].features["copyCount"].target,
        "EInt"
    );
    let sources = [Source { uri: "catalog.library".into(), text: "library Catalog { copy dune \"Dune\"; member ada; loan first copy dune to ada; loan second copy dune to ada; }".into() }];
    let result = evaluate(&package, &sources);
    assert_eq!(result.derived["catalog.library#0"]["copyCount"], 1);
    assert_eq!(result.derived["catalog.library#1"]["available"], false);
    assert_eq!(result.diagnostics.len(), 2);
    assert_eq!(
        evaluate(&compiler::recompile(&package).unwrap(), &sources).derived,
        result.derived
    );
    for invalid in [
        behavior.replace("count(copies)", "count(name)"),
        behavior.replace("noIncoming(lib.Loan.copy)", "noIncoming(lib.Member.name)"),
    ] {
        assert!(compiler::compile_rcore_language(model, grammar, &invalid).is_err());
    }
}

#[test]
fn canonical_initializers_and_projections_execute_and_replay() {
    let behavior = format!(
        r#"{HEADER}
initialize p.Item using "intersect.constant-attributes" version "1" {{ origin = "constructed"; }}
projection p.Item.exposed from origin writable;"#
    );
    let package = compiler::compile_rcore_with_profile(
        MODEL,
        Some(GRAMMAR),
        Some(&behavior),
        Profile::NativeV1,
    )
    .unwrap();
    let source = Source {
        uri: "memory:item".into(),
        text: "item sample".into(),
    };
    for package in [&package, &compiler::recompile(&package).unwrap()] {
        let result = evaluate(package, std::slice::from_ref(&source));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let item = &result.model.elements[0];
        assert_eq!(item.values["origin"], "constructed");
        assert_eq!(result.derived[&item.id]["exposed"], "constructed");
        assert!(item.initialization.is_some());
    }
    let mut missing = package.clone();
    missing.compilation = None;
    assert!(intersect_model::initialization::PackageInitializers::new(&missing).is_err());
    let native = format!(
        "{MODEL} behavior {{ import model \"urn:providers\" as p; projection p.Item.exposed from origin; }}"
    );
    assert!(
        intersect_model::compile_rcore(&native)
            .unwrap_err()
            .to_string()
            .contains("provider contract adapters")
    );
}

#[test]
fn canonical_contracts_reject_invalid_members_values_and_projection_shapes() {
    for declaration in [
        r#"initialize p.Item using "intersect.constant-attributes" version "1" { origin = missing(); }"#,
        r#"initialize p.Item using "intersect.constant-attributes" version "1" { origin; }"#,
        r#"initialize p.Item using "intersect.constant-attributes" version "1" { origin = "a"; origin = "b"; }"#,
        r#"initialize p.Unknown using "host" version "1" {}"#,
        r#"initialize p.Item using "host" version "1" { origin = "a"; }"#,
        r#"projection p.Item.exposed from missing;"#,
        r#"projection p.Item from origin;"#,
    ] {
        let errors = compiler::compile_rcore_with_profile(
            MODEL,
            Some(GRAMMAR),
            Some(&format!("{HEADER} {declaration}")),
            Profile::NativeV1,
        )
        .unwrap_err();
        assert!(
            errors.iter().all(|e| e.source.contains("behavior")),
            "{declaration}: {errors:?}"
        );
    }
}

#[test]
fn canonical_converter_retains_exact_grammar_identity_and_boolean_execution() {
    let model = intersect_text_acceptance::migrate_model_fixture(include_str!(
        "../../../../fixtures/xtext/rule-converters/model.ecore"
    ))
    .unwrap();
    let grammar = include_str!("../../../../fixtures/xtext/rule-converters/model.xtext");
    let header = "rcore 1; package converters version \"1\";";
    let baseline = compiler::compile_rcore_with_profile(
        &model,
        Some(grammar),
        Some(header),
        Profile::NativeV8,
    )
    .unwrap();
    let span = &baseline.grammar.as_ref().unwrap().declaration_sources["Nonunique"];
    let behavior = format!(
        r#"{header}
convert "evidence.RuleConverters" rule "Nonunique"
from {:?} sha256 {:?} span {} {}
to "http://www.eclipse.org/emf/2002/Ecore#//EBoolean"
using "intersect.xtext.keyword-boolean" version "1" {{ "nonunique" = false; }}"#,
        span.source.uri, span.source.sha256, span.start, span.end
    );
    let package = compiler::compile_rcore_with_profile(
        &model,
        Some(grammar),
        Some(&behavior),
        Profile::NativeV8,
    )
    .unwrap();
    let source = Source {
        uri: "memory:converter".into(),
        text: "feature item nonunique;".into(),
    };
    for package in [&package, &compiler::recompile(&package).unwrap()] {
        let result = evaluate(package, std::slice::from_ref(&source));
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements[0].values["isUnique"], false);
    }
    let stale = behavior.replace(&span.source.sha256, &"0".repeat(64));
    assert!(
        compiler::compile_rcore_with_profile(
            &model,
            Some(grammar),
            Some(&stale),
            Profile::NativeV8
        )
        .unwrap_err()
        .iter()
        .any(|e| e.message.contains("identity mismatch"))
    );
    let duplicate = behavior.replace(
        "\"nonunique\" = false;",
        "\"nonunique\" = false; \"nonunique\" = true;",
    );
    assert!(
        compiler::compile_rcore_with_profile(
            &model,
            Some(grammar),
            Some(&duplicate),
            Profile::NativeV8
        )
        .unwrap_err()
        .iter()
        .any(|e| e.message.contains("Duplicate conversion keyword"))
    );
}
