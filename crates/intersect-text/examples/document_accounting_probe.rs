//! Verification-only actual public document limits and physical type layout observations.
use intersect_model::core::text_document::{
    DocumentBuilder, DocumentLimits, DocumentMark, Node, NodeData, NodeId, TextDocument, TextRange,
};
use intersect_model::definitions::{
    bundle::{SourceId, SourceSpan},
    compiler::OwnedRuntimeSourceArrayPreparation,
};
use intersect_text::package::{BundleCompilation, Package};
use intersect_text::{
    EvaluationLimits, Source,
    document::{AnalysisLimits, DocumentLanguage},
};
use serde_json::{Value, json};
use std::mem::size_of;
fn run(package: &Package, limits: AnalysisLimits) -> Result<Value, Value> {
    let language = DocumentLanguage::prepare(package, Default::default(), limits)
        .map_err(|diagnostics| json!({"stage":"prepare","diagnostics":diagnostics}))?;
    let source = Source {
        uri: "memory:document-accounting-original/model".into(),
        text: "type:a;use:f=a;".into(),
    };
    let analysis = language
        .analyze(&source, None, EvaluationLimits::native())
        .map_err(|error| json!({"stage":"analyze","error":error}))?;
    let value = json!({"diagnostics":analysis.diagnostics,"snapshot":analysis.snapshot,"valid_model":analysis.valid_model,"recovered":analysis.recovered});
    if value["valid_model"].is_null() {
        return Err(json!({"stage":"invalid_analysis","analysis":value}));
    }
    Ok(value)
}
fn minimum(package: &Package, association: bool) -> Value {
    let defaults = AnalysisLimits::workbench();
    let mut low = 0;
    let mut high = if association {
        defaults.association_bytes
    } else {
        defaults.tree.logical_storage_bytes
    };
    let mut steps = Vec::new();
    let attempt = |cap| {
        let mut limits = defaults;
        if association {
            limits.association_bytes = cap;
        } else {
            limits.tree.logical_storage_bytes = cap;
        }
        run(package, limits)
    };
    assert!(attempt(high).is_ok());
    while low < high {
        let mid = low + (high - low) / 2;
        let trial = attempt(mid);
        steps.push(match &trial {
            Ok(_) => json!({"cap":mid,"ok":true}),
            Err(error) => json!({"cap":mid,"ok":false,"error":error}),
        });
        if trial.is_ok() {
            high = mid
        } else {
            low = mid + 1
        }
    }
    assert!(low > 0);
    let exact = attempt(low).expect("actual exact minimum succeeds");
    let short = attempt(low - 1).expect_err("one-short fails through actual admission");
    json!({"minimum_successful_cap":low,"exact":{"cap":low,"ok":true,"analysis":exact},"one_short":{"cap":low-1,"ok":false,"error":short},"search":steps})
}
fn observation() -> Value {
    let all: Value =
        serde_json::from_str(include_str!("../src/document_tests/cases.json")).unwrap();
    let cases = all["cases"].as_array().unwrap();
    let selected: Vec<_> = cases
        .iter()
        .filter(|case| case["id"] == "reference_adds_scope")
        .collect();
    assert_eq!(selected.len(), 1);
    let compilation: BundleCompilation =
        serde_json::from_value(selected[0]["compilation"].clone()).unwrap();
    let package = intersect_text::ordinary::compile(
        compilation.clone(),
        intersect_text::grammar::Profile::NativeV65,
    )
    .unwrap();
    let outer = size_of::<OwnedRuntimeSourceArrayPreparation>();
    assert!(outer + 128 <= 2048);
    json!({"scope":"BEFORE private NativeV65 accounting correction; actual unchanged public DocumentLanguage prepare/analyze, exact original source graph, exact/one-short thresholds; no alternate parser or allocator capacity claim","compilation":compilation,"source":{"uri":"memory:document-accounting-original/model","text":"type:a;use:f=a;"},"physical_layouts":{"pointer_bytes":size_of::<usize>(),"document_builder":size_of::<DocumentBuilder>(),"document_limits":size_of::<DocumentLimits>(),"document_mark":size_of::<DocumentMark>(),"node":size_of::<Node>(),"node_data":size_of::<NodeData>(),"node_id":size_of::<NodeId>(),"text_range":size_of::<TextRange>(),"text_document":size_of::<TextDocument>(),"source_id":size_of::<SourceId>(),"source_span":size_of::<SourceSpan>(),"analysis_limits":size_of::<AnalysisLimits>(),"owned_runtime_source_array_preparation":outer,"original_holder_reserved":2048,"original_holder_envelope":outer+128},"baseline":run(&package,AnalysisLimits::workbench()).unwrap(),"tree_bytes":minimum(&package,false),"association_bytes":minimum(&package,true)})
}
fn main() {
    println!("{}", observation());
}
static OUTPUT: std::sync::Mutex<Vec<u8>> = std::sync::Mutex::new(Vec::new());
#[unsafe(no_mangle)]
pub extern "C" fn observe_accounting() -> usize {
    let bytes = serde_json::to_vec(&observation()).unwrap();
    let mut output = OUTPUT.lock().unwrap();
    *output = bytes;
    output.len()
}
#[unsafe(no_mangle)]
pub extern "C" fn accounting_output() -> *const u8 {
    OUTPUT.lock().unwrap().as_ptr()
}
