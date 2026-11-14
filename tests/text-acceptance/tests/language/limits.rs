use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_with_limits};

fn package() -> intersect_text::package::Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap()
}

#[test]
fn native_evaluation_exceeds_former_element_byte_and_resource_limits() {
    let p = package();
    let mut text = String::from("library Scale {\n");
    for i in 0..512 {
        text.push_str(&format!("member m{i};\n"));
    }
    text.push('}');
    text.push_str(&" ".repeat(256_001));
    let mut sources = vec![Source {
        uri: "large".into(),
        text,
    }];
    for i in 0..8 {
        sources.push(Source {
            uri: format!("resource{i}"),
            text: format!("library Empty{i} {{}}"),
        });
    }
    let result = evaluate(&p, &sources);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 521);
    assert_eq!(result.model.elements[512].values["name"], "m511");
    result.model.validate(&p.schema).unwrap();
}

#[test]
fn explicit_budgets_have_inclusive_boundaries_and_reject_overages() {
    let p = package();
    let sources = [Source {
        uri: "small".into(),
        text: "library Small { member a; }".into(),
    }];
    let exact = EvaluationLimits {
        resources: Some(1),
        input_bytes: Some(sources[0].text.len()),
        elements_per_resource: Some(2),
        ..EvaluationLimits::native()
    };
    let native = evaluate(&p, &sources);
    let bounded = evaluate_with_limits(&p, &sources, exact);
    assert!(bounded.diagnostics.is_empty(), "{:?}", bounded.diagnostics);
    assert_eq!(native.model, bounded.model);
    assert_eq!(native.derived, bounded.derived);
    for limits in [
        EvaluationLimits {
            resources: Some(0),
            ..exact
        },
        EvaluationLimits {
            input_bytes: Some(sources[0].text.len() - 1),
            ..exact
        },
        EvaluationLimits {
            elements_per_resource: Some(1),
            ..exact
        },
        EvaluationLimits {
            steps_per_resource: Some(0),
            ..exact
        },
        EvaluationLimits {
            tokens_per_resource: Some(1),
            ..exact
        },
        EvaluationLimits {
            nesting: 1,
            ..exact
        },
    ] {
        let result = evaluate_with_limits(&p, &sources, limits);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("budget")),
            "{limits:?}"
        );
        assert!(result.model.elements.is_empty());
    }
    assert_eq!(EvaluationLimits::default(), EvaluationLimits::native());
}

#[test]
fn native_does_not_inherit_the_workbench_step_budget() {
    let p = package();
    let source = [Source {
        uri: "small".into(),
        text: "library Small {}".into(),
    }];
    let limited = evaluate_with_limits(
        &p,
        &source,
        EvaluationLimits {
            steps_per_resource: Some(1),
            ..EvaluationLimits::native()
        },
    );
    assert!(
        limited
            .diagnostics
            .iter()
            .any(|d| d.message == "Execution budget exceeded")
    );
    assert!(evaluate(&p, &source).diagnostics.is_empty());
}

#[test]
fn native_passes_former_token_and_step_ceilings_using_the_same_interpreter() {
    let p = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        &include_str!("../../../../fixtures/library/library.xtext")
            .replace("'library' name=ID", "'library' 'marker'* name=ID"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let source = [Source {
        uri: "tokens".into(),
        text: format!("library {} Scale {{}}", "marker ".repeat(100_001)),
    }];
    let result = evaluate(&p, &source);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 1);
    for limits in [
        EvaluationLimits {
            tokens_per_resource: Some(50_000),
            ..EvaluationLimits::native()
        },
        EvaluationLimits {
            steps_per_resource: Some(100_000),
            ..EvaluationLimits::native()
        },
    ] {
        let result = evaluate_with_limits(&p, &source, limits);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.message.contains("budget"))
        );
        assert!(result.model.elements.is_empty());
    }
    assert!(intersect_text::grammar::lex(&source[0].text).is_err());
}
