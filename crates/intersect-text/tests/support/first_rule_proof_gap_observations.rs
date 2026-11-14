//! Handwritten verification shared by native tests and the isolated WASM probe.
//! All language admission uses the existing public native implementation.
use intersect_text::grammar::SourceLimits;
use intersect_text::package::BundleCompilation;
use serde_json::{Value, json};

fn input() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../../fixtures/xtext/contextual-first-rule-proof-gaps/native/native-cases.json"
    ))
    .map_err(|e| e.to_string())
}
fn fixture(path: &str) -> Result<Value, String> {
    let text = match path {
        "golden" => include_str!(
            "../../../../fixtures/xtext/contextual-first-rule-proof-gaps/native/helper/helper-record.json"
        ),
        "oracle" => include_str!(
            "../../../../fixtures/xtext/contextual-first-rule-proof-gaps/oracle/record/observations.json"
        ),
        "before" => include_str!(
            "../../../../fixtures/xtext/contextual-first-rule-proof-gaps/native/before/record.json"
        ),
        _ => return Err("Unknown verification fixture".into()),
    };
    serde_json::from_str(text).map_err(|e| e.to_string())
}
fn ensure(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}
fn cases(value: &Value) -> Result<&[Value], String> {
    value["cases"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "Missing source cases".into())
}
fn selected(id: &str) -> Result<BundleCompilation, String> {
    let all = input()?;
    let case = cases(&all)?
        .iter()
        .find(|c| c["id"] == id)
        .ok_or("Missing exact original graph")?;
    serde_json::from_value(case["compilation"].clone()).map_err(|e| e.to_string())
}
fn observe(compilation: &BundleCompilation, limits: SourceLimits) -> Result<Value, String> {
    let entry = compilation
        .xtext
        .as_ref()
        .ok_or("Missing grammar SourceId")?;
    let result = match intersect_text::ordinary::validate_authored_program(
        &compilation.bundle,
        entry,
        limits,
    ) {
        Ok(()) => json!({"ok":true}),
        Err(diagnostic) => {
            let origin = serde_json::to_value(&diagnostic.origin).map_err(|e| e.to_string())?;
            json!({"ok":false,"diagnostic":diagnostic,"origin":origin})
        }
    };
    Ok(result)
}
fn failure(
    compilation: &BundleCompilation,
    limits: SourceLimits,
    message: &str,
) -> Result<Value, String> {
    let result = observe(compilation, limits)?;
    ensure(
        result["ok"] == false,
        "A negative source control was admitted",
    )?;
    ensure(
        result["diagnostic"]["message"]
            .as_str()
            .is_some_and(|s| s.contains(message)),
        "Unexpected negative-control diagnostic",
    )?;
    Ok(result)
}
pub fn baseline() -> Result<Value, String> {
    let original = input()?;
    let mut rows = Vec::new();
    let limits = SourceLimits::default();
    for case in cases(&original)? {
        let compilation: BundleCompilation =
            serde_json::from_value(case["compilation"].clone()).map_err(|e| e.to_string())?;
        let entry = compilation
            .xtext
            .as_ref()
            .ok_or("Missing exact original entry")?;
        compilation.bundle.validate()?;
        let result = observe(&compilation, limits)?;
        rows.push(json!({"id":case["id"],"request":{"bundle":compilation.bundle,"entry":entry,"source_limits":{"source_bytes":limits.source_bytes,"declarations":limits.declarations}},"result":result}));
    }
    Ok(
        json!({"version":1,"scope":"Public authored-source validation only under fixed NativeV61. No executable package, generation/runtime, inferred-profile or full-support award.","input":original,"cases":rows}),
    )
}
fn independent_oracle_coverage(actual: &Value) -> Result<Value, String> {
    let golden = fixture("golden")?;
    ensure(
        actual == &golden,
        "Complete helper observations differ from first native golden",
    )?;
    let oracle = fixture("oracle")?;
    let oracle = oracle
        .as_array()
        .ok_or("Missing complete oracle observations")?;
    let before = fixture("before")?;
    let all = input()?;
    let mut covered = Vec::new();
    for ((source, observed), upstream) in cases(&all)?.iter().zip(cases(actual)?).zip(oracle) {
        ensure(
            source["id"] == observed["id"] && source["id"] == upstream["id"],
            "Graph identity mismatch",
        )?;
        let compilation: BundleCompilation =
            serde_json::from_value(source["compilation"].clone()).map_err(|e| e.to_string())?;
        let validations = upstream["metadata"]["observation"]["validation"]
            .as_array()
            .ok_or("Missing validator observations")?;
        let mut errors = Vec::new();
        for grammar in validations {
            ensure(
                grammar["failure"].is_null(),
                "A validator adapter failure is not an issue",
            )?;
            for issue in grammar["issues"]
                .as_array()
                .ok_or("Missing validator issues")?
            {
                if issue["severity"] == "ERROR" {
                    errors.push((&grammar["source"], issue));
                }
            }
        }
        ensure(
            observed["result"]["ok"] == errors.is_empty(),
            "Helper admission differs from loaded default validator",
        )?;
        if observed["result"]["ok"] == false {
            let diagnostic = &observed["result"]["diagnostic"];
            let owner = compilation
                .bundle
                .sources
                .iter()
                .find(|s| s.id.uri == diagnostic["source"])
                .ok_or("Diagnostic source not in original world")?;
            let pins = source["source_pins"]
                .as_array()
                .ok_or("Missing source pins")?;
            let pin = pins
                .iter()
                .find(|p| p["source_id"] == json!(owner.id))
                .ok_or("Missing original grammar identity")?;
            ensure(
                errors.iter().any(|(name, issue)| {
                    *name == &pin["grammar"] && issue["message"] == diagnostic["message"]
                }),
                "Native issue lacks matching original upstream source issue",
            )?;
            let origin = &observed["result"]["origin"];
            ensure(
                origin["kind"] == "source"
                    && origin["source"] == json!(owner.id)
                    && origin["span"]["source"] == json!(owner.id),
                "Diagnostic lost native exact SourceId",
            )?;
            ensure(
                origin["span"]["start"] == diagnostic["start"]
                    && origin["span"]["end"] == diagnostic["end"],
                "Owned span differs from native diagnostic",
            )?;
        }
        let old = cases(&before)?
            .iter()
            .find(|c| c["id"] == source["id"])
            .ok_or("Missing original package failure")?;
        let package_failure = intersect_text::ordinary::compile(
            compilation,
            intersect_text::grammar::Profile::NativeV61,
        );
        let diagnostics = package_failure
            .err()
            .ok_or("Source helper must not imply package support")?;
        ensure(
            json!(diagnostics) == old["compile"]["result"]["diagnostics"],
            "Complete original package diagnostic changed",
        )?;
        covered.push(json!({"id":source["id"],"helper_admitted":observed["result"]["ok"],"package_compiled":false,"generation":upstream["generation"],"javac":upstream["compilation"],"runtime_inputs":upstream["runtime"].as_array().ok_or("Missing runtime stage")?.len()}));
    }
    ensure(covered.len() == 5, "Coverage must include all five graphs")?;
    Ok(json!(covered))
}
fn source_hash_forgery() -> Result<Value, String> {
    let mut forged = selected("wildcard_original_control")?;
    let entry = forged.xtext.clone().ok_or("Missing original entry")?;
    let source = forged
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == entry)
        .ok_or("Missing entry source")?;
    source.text.push(' ');
    failure(&forged, SourceLimits::default(), "digest")
}
fn exact_used_closure() -> Result<Value, String> {
    let original = selected("scalar_fragment_model_child")?;
    let entry = original.xtext.clone().ok_or("Missing child entry")?;
    let parent = original
        .bundle
        .sources
        .iter()
        .find(|s| {
            s.kind == intersect_model::definitions::bundle::SourceKind::Xtext && s.id != entry
        })
        .ok_or("Missing exact parent")?
        .clone();
    let mut missing = original.clone();
    missing
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == entry)
        .ok_or("Missing child")?
        .dependencies
        .retain(|id| id != &parent.id);
    let missing_result = failure(
        &missing,
        SourceLimits::default(),
        "Missing exact parent grammar dependency",
    )?;
    let mut unrelated = original.clone();
    let mut duplicate = parent;
    duplicate.id.uri = "memory:proof-gaps/unselected-same-name-parent.xtext".into();
    unrelated.bundle.sources.push(duplicate.clone());
    let unrelated_result = observe(&unrelated, SourceLimits::default())?;
    ensure(
        unrelated_result["ok"] == true,
        "Undeclared same-name source contaminated exact used closure",
    )?;
    unrelated
        .bundle
        .sources
        .iter_mut()
        .find(|s| s.id == entry)
        .ok_or("Missing child")?
        .dependencies
        .push(duplicate.id);
    let ambiguous_result = failure(
        &unrelated,
        SourceLimits::default(),
        "Ambiguous exact parent grammar dependency",
    )?;
    Ok(
        json!({"missing_declared_parent":missing_result,"unselected_same_name":unrelated_result,"ambiguous_declared_parent":ambiguous_result}),
    )
}
fn source_limits() -> Result<Value, String> {
    let original = selected("wildcard_original_control")?;
    let length = original
        .bundle
        .sources
        .iter()
        .map(|source| source.text.len())
        .max()
        .ok_or("Missing exact source world")?;
    let exact = SourceLimits {
        source_bytes: length,
        declarations: 4,
    };
    let exact_result = observe(&original, exact)?;
    ensure(
        exact_result["ok"] == true,
        &format!("Exact existing source/declaration limits must admit: {exact_result}"),
    )?;
    let bytes = failure(
        &original,
        SourceLimits {
            source_bytes: length - 1,
            ..exact
        },
        "budget",
    )?;
    let declarations = failure(
        &original,
        SourceLimits {
            declarations: 3,
            ..exact
        },
        "budget",
    )?;
    Ok(
        json!({"exact_limits":{"source_bytes":length,"declarations":4},"exact":exact_result,"one_byte_below":bytes,"one_declaration_below":declarations}),
    )
}
pub fn suite() -> Result<Value, String> {
    let actual = baseline()?;
    let coverage = independent_oracle_coverage(&actual)?;
    Ok(
        json!({"version":1,"scope":"Verification only: shared native/WASM source-admission observations and negative controls. No production/profile or package/runtime support award.","baseline":actual,"coverage":coverage,"source_hash_forgery":source_hash_forgery()?,"exact_used_closure":exact_used_closure()?,"source_limits":source_limits()?}),
    )
}
