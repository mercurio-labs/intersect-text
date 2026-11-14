//! Public ordinary compiler budget controls, moved with the frontend.
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar;
use intersect_text::grammar::GrammarAnalysisWorkBudget;
use intersect_text::grammar::Profile;
/// Handwritten control for the explicit-budget ordinary compile entry points.
/// One fragment called by many rules: the historical 10k grant is exhausted,
/// the default grant is not.
fn ordinary_budget_inputs() -> intersect_text::package::BundleCompilation {
    use intersect_model::definitions::bundle::DefinitionBundle;
    let mut ecore = "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:x='http://www.w3.org/2001/XMLSchema-instance' name='budget' nsURI='urn:ordinary-budget' nsPrefix='budget'><eClassifiers x:type='e:EClass' name='Base'/>".to_owned();
    let mut xtext =
        "grammar intersect.fixture.OrdinaryBudget\nimport \"urn:ordinary-budget\" as m\n"
            .to_owned();
    for index in 0..60 {
        ecore.push_str(&format!(
            "<eClassifiers x:type='e:EClass' name='A{index:02}' eSuperTypes='#//Base'/>"
        ));
        xtext.push_str(&format!("A{index:02} returns m::A{index:02}: F;\n"));
    }
    ecore.push_str("</e:EPackage>");
    xtext.push_str(&format!(
        "fragment F returns m::Base: {};\n",
        vec!["'x'"; 200].join(" ")
    ));
    let ecore = DefinitionSource::new("memory:budget/model.ecore", SourceKind::Ecore, ecore);
    let model = intersect_model::migration::migrate_ecore(ecore, "memory:budget/model.rcore")
        .unwrap()
        .converted
        .unwrap();
    let mut grammar =
        DefinitionSource::new("memory:budget/language.xtext", SourceKind::Xtext, xtext);
    grammar.dependencies.push(model.id.clone());
    let mut behavior = DefinitionSource::new(
        "memory:budget/rules.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1; package emptyRules version \"1\";",
    );
    behavior.dependencies.push(model.id.clone());
    intersect_text::package::BundleCompilation {
        materialized_types: vec![],
        ecore: model.id.clone(),
        namespace: "urn:ordinary-budget".into(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: behavior.id.clone(),
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![model, grammar, behavior],
            namespaces: vec![],
        },
    }
}

#[test]
fn ordinary_compile_honours_explicit_budget_and_keeps_default_path() {
    use intersect_text::compiler;
    use intersect_text::package::CompilationSourcePolicy;
    let input = ordinary_budget_inputs();
    let policy = CompilationSourcePolicy::default();
    // Durable profiles require a language-owned authored-source validator; this
    // test-only permissive one exercises the Xtext-owned entry points.
    let validate = |_: &intersect_model::definitions::bundle::DefinitionBundle,
                    _: &intersect_model::definitions::bundle::SourceId,
                    _: grammar::SourceLimits| Ok(());
    let compile = |budget: Option<GrammarAnalysisWorkBudget>| match budget {
        None => compiler::compile_bundle_with_borrowed_authored_validation(
            input.clone(),
            Profile::NativeV69,
            &[],
            None,
            policy,
            policy,
            &validate,
        ),
        Some(budget) => compiler::compile_bundle_with_context_and_budget(
            input.clone(),
            Profile::NativeV69,
            &[],
            None,
            policy,
            policy,
            Some(&validate),
            None,
            budget,
        ),
    };
    // Default path unchanged: the explicit default grant builds the same package.
    let default = compile(None).unwrap();
    assert_eq!(
        serde_json::to_value(&default).unwrap(),
        serde_json::to_value(compile(Some(GrammarAnalysisWorkBudget::default())).unwrap()).unwrap()
    );
    // The explicit historical grant is honoured: the same input rejects.
    let legacy = compile(Some(GrammarAnalysisWorkBudget::LEGACY_10K)).unwrap_err();
    assert_eq!(legacy.len(), 1);
    assert_eq!(legacy[0].message, "Grammar prediction budget exceeded");
}
