use super::*;
use intersect_text::inference::InferenceRequest;
use serde_json::json;

pub(super) fn verify() -> Result<(), String> {
    const GRAMMAR: &str = "grammar independent.Greeting with org.eclipse.xtext.common.Terminals\ngenerate greeting \"urn:staged:greeting\"\nGreeting: 'hello' name=ID;";
    let compilation = inputs(GRAMMAR);
    require(
        intersect_text::ordinary::compile(compilation.clone(), Profile::NativeV82).is_err(),
        "Ordinary compilation silently activated inferred declarations",
    )?;
    let request = InferenceRequest {
        version: 1,
        grammar: compilation.xtext.ok_or("Missing grammar selection")?,
        bundle: compilation.bundle,
    };
    let inferred = request
        .compile_with_selected_profile(
            Default::default(),
            Some(Profile::NativeV82),
            &[],
            None,
            None,
        )
        .map_err(diagnostics)?;
    let source = [Source {
        uri: "hello.greeting".into(),
        text: "hello World".into(),
    }];
    let result = intersect_text::evaluate(inferred.package(), &source);
    require(
        result.diagnostics.is_empty(),
        &format!("Inferred diagnostics: {:?}", result.diagnostics),
    )?;
    require(
        result.model.elements.len() == 1
            && result.model.elements[0].values.get("name") == Some(&json!("World")),
        "Inferred declaration/value differs",
    )?;
    let retained = inferred
        .package()
        .xtext_inference
        .as_ref()
        .ok_or("Missing inference recipe")?;
    require(
        retained.bundle.source(&retained.grammar)?.text == GRAMMAR,
        "Inference lost original grammar bytes",
    )?;
    let mut metadata = inferred.package().clone();
    metadata.grammar = None;
    let rebuilt = intersect_text::recompile(&metadata).map_err(diagnostics)?;
    require(
        rebuilt.grammar.is_some()
            && intersect_text::evaluate(&rebuilt, &source).model == result.model,
        "Inference replay trusted generated metadata instead of original sources",
    )?;
    let mut invalid = request.clone();
    invalid.version = 2;
    require(
        invalid.compile(Default::default()).is_err(),
        "Unsupported inference version accepted",
    )?;
    require(
        !intersect_text::evaluate(
            inferred.package(),
            &[Source {
                uri: source[0].uri.clone(),
                text: "hello 123".into(),
            }],
        )
        .diagnostics
        .is_empty(),
        "Invalid inferred document accepted",
    )?;
    require(
        intersect_text::evaluate(inferred.package(), &source).model == result.model,
        "Inference failed to recover after rejection",
    )?;
    Ok(())
}
