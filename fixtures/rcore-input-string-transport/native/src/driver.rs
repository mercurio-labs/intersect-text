use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceId, SourceKind},
    conversion::ReferenceContext,
    rcore_behavior::{
        function_lowering::compile_function,
        ir,
        ir_v2::{DataWire, EntrySelection, Limits, ValueWire, VerifiedExecutable},
        modules::SymbolId,
        types::Scalar,
    },
};
use intersect_model::rcore_behavior_functions::{
    EvaluationSession, ExecutionContext, ExecutionControl, ExecutionLimits, SessionProgress,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::num::NonZeroUsize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub case: String,
    pub id: String,
    pub owned: bool,
    pub per_value: Option<usize>,
    pub nodes: Option<usize>,
    pub depth: Option<usize>,
    pub quantum: usize,
    pub limit: Option<usize>,
    pub cancel: Option<usize>,
    pub drop_at: Option<usize>,
}
fn parsed<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|e| e.to_string())
}
fn compile(
    bundle: &DefinitionBundle,
    entry: &SourceId,
    selection: &EntrySelection,
) -> Result<VerifiedExecutable, String> {
    compile_function(bundle, entry, selection, &Limits::default())
        .map_err(|e| format!("Fixed source compilation failed: {e:?}"))
}
pub(crate) fn fixture(
    id: &str,
) -> Result<
    (
        VerifiedExecutable,
        Vec<ValueWire>,
        DefinitionBundle,
        Vec<ReferenceContext>,
    ),
    String,
> {
    if let Some(key) = id.strip_prefix("nested/") {
        let (invalid, key) = match key.strip_prefix("invalid/") {
            Some(key) => (true, key),
            None => (false, key),
        };
        let cases: Value = serde_json::from_str(include_str!("../../inputs-v1/nested-cases.json"))
            .map_err(|e| e.to_string())?;
        let case = cases
            .as_array()
            .ok_or("Missing nested cases")?
            .iter()
            .find(|v| v["id"] == key)
            .ok_or("Unknown nested case")?;
        let request = &case[if invalid { "invalid" } else { "sync" }];
        let bundle = parsed(request["bundle"].clone())?;
        let entry = parsed(request["entry"].clone())?;
        let selection = parsed(request["selection"].clone())?;
        let image = compile(&bundle, &entry, &selection)?;
        if serde_json::to_value(image.program()).map_err(|e| e.to_string())?
            != request["expected_program"]
        {
            return Err("Original nested complete program changed".into());
        }
        return Ok((image, parsed(request["arguments"].clone())?, bundle, vec![]));
    }
    if id == "model-original-240000" {
        let mut f: Value =
            serde_json::from_str(include_str!("../../inputs-v1/model-original-240000.json"))
                .map_err(|e| e.to_string())?;
        let request = f
            .get_mut("request")
            .and_then(Value::as_object_mut)
            .ok_or("Missing original request")?;
        let mut take = |name: &str| {
            request
                .remove(name)
                .ok_or_else(|| format!("Missing fixed field {name}"))
        };
        let bundle = parsed(take("bundle")?)?;
        let entry = parsed(take("entry")?)?;
        let selection = parsed(take("selection")?)?;
        let image = compile(&bundle, &entry, &selection)?;
        if serde_json::to_value(image.program()).map_err(|e| e.to_string())?
            != take("expected_program")?
        {
            return Err("Original full model program changed".into());
        }
        return Ok((
            image,
            parsed(take("arguments")?)?,
            bundle,
            parsed(take("reference_contexts")?)?,
        ));
    }
    let input: Value = serde_json::from_str(include_str!("../../inputs-v1/inputs.json"))
        .map_err(|e| e.to_string())?;
    let source = DefinitionSource::new(
        input["uri"].as_str().ok_or("Missing fixed URI")?,
        SourceKind::RcoreBehavior,
        input["source"].as_str().ok_or("Missing fixed source")?,
    );
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let selection = EntrySelection {
        symbol: SymbolId {
            source: source.id.clone(),
            declaration: 0,
        },
        types: vec![],
        arguments: vec![],
    };
    let image = compile(&bundle, &source.id, &selection)?;
    let expected: Value =
        serde_json::from_str(include_str!("../../inputs-v1/expected-program.json"))
            .map_err(|e| e.to_string())?;
    if serde_json::to_value(image.program()).map_err(|e| e.to_string())? != expected {
        return Err("Original full pure program changed".into());
    }
    let payload = if id == "ascii240000" {
        Value::String("x".repeat(240000))
    } else {
        input["cases"]
            .as_array()
            .ok_or("Missing fixed cases")?
            .iter()
            .find(|v| v["id"] == id)
            .ok_or("Unknown fixed case")?["payload"]
            .clone()
    };
    let function = image
        .program()
        .functions
        .get(image.program().entry.0 as usize)
        .ok_or("Missing fixed entry")?;
    let ty = *function
        .parameters
        .first()
        .ok_or("Missing fixed parameter")?;
    Ok((
        image,
        vec![ValueWire {
            ty,
            data: DataWire::Scalar(ir::Value {
                ty: ir::ValueType::Scalar(Scalar::String),
                payload,
            }),
        }],
        bundle,
        vec![],
    ))
}
struct Control(bool);
impl ExecutionControl for Control {
    fn cancellation_requested(&self) -> bool {
        self.0
    }
}
pub fn run(request: Request) -> Result<Value, String> {
    if ![1, 7].contains(&request.quantum) {
        return Err(
            "Verification supports original q1/q7 only; native MAX observations remain separate"
                .into(),
        );
    }
    let quantum = NonZeroUsize::new(request.quantum).ok_or("Zero quantum")?;
    let (image, args, bundle, references) = fixture(&request.id)?;
    let ids = bundle
        .sources
        .iter()
        .map(|s| s.id.clone())
        .collect::<Vec<_>>();
    let mut limits = ExecutionLimits::default();
    if let Some(n) = request.limit {
        limits.scalar.max_value_bytes = n;
    }
    if let Some(n) = request.per_value {
        limits.scalar.values.max_value_bytes = n;
    }
    if let Some(n) = request.nodes {
        limits.max_value_nodes = n;
    }
    if let Some(n) = request.depth {
        limits.max_value_depth = n;
    }
    let mut context = ExecutionContext::new(&bundle);
    context.references = &references;
    let mut session = if request.owned {
        EvaluationSession::new_owned(
            image.clone(),
            args.clone(),
            bundle.clone(),
            references.clone(),
            None,
            limits.clone(),
        )
    } else {
        EvaluationSession::new(&image, &args, &context, &limits)
    }
    .map_err(|e| format!("Session construction: {e:?}"))?;
    let mut events = vec![];
    for poll in 0..200000 {
        if request.drop_at == Some(poll) {
            drop(session);
            return Ok(
                json!({"case":request.case,"source_ids":ids,"events":events,"dropped":true,"finished":false}),
            );
        }
        let result = session.resume_with_control(quantum, &Control(request.cancel == Some(poll)));
        match result {
            SessionProgress::Pending => events.push(json!({"pending":true})),
            SessionProgress::Complete(result) => {
                events.push(json!({"complete":result}));
                if !matches!(session.resume(NonZeroUsize::MIN), SessionProgress::Finished) {
                    return Err("Completed session did not finish".into());
                }
                return Ok(
                    json!({"case":request.case,"source_ids":ids,"events":events,"dropped":false,"finished":true}),
                );
            }
            SessionProgress::Finished => {
                return Err("Session finished without terminal result".into());
            }
        }
    }
    Err("Verification session exceeded fixed harness cap".into())
}
