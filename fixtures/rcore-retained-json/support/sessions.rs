// Verification only: actual source compilation, public sessions and checked Rust provider.
use intersect_model::{
    definitions::{
        bundle::DefinitionBundle,
        rcore_behavior::{
            function_lowering::compile_function,
            ir_v2::{Limits, ValueWire},
        },
        semantic_bindings::Contract,
    },
    rcore_behavior_functions::{
        EvaluationSession, ExecutionContext, ExecutionLimits, SessionProgress,
    },
    semantic_bindings::{BindingProgress, Budget, NativeProvider, Registry},
};
use serde_json::{Value, json};
use std::{cell::Cell, num::NonZeroUsize, rc::Rc};
pub fn observe() -> Value {
    let requests: Value = serde_json::from_str(include_str!("../requests.json")).unwrap();
    let mut rows = Vec::new();
    for request in requests.as_array().unwrap() {
        let r = &request["request"];
        let id = request["id"].as_str().unwrap();
        let bundle: DefinitionBundle = serde_json::from_value(r["bundle"].clone()).unwrap();
        let entry = serde_json::from_value(r["entry"].clone()).unwrap();
        let selection = serde_json::from_value(r["selection"].clone()).unwrap();
        let image = compile_function(&bundle, &entry, &selection, &Limits::default()).unwrap();
        assert_eq!(
            serde_json::to_value(image.program()).unwrap(),
            r["expected_program"]
        );
        let args: Vec<ValueWire> = serde_json::from_value(r["arguments"].clone()).unwrap();
        let context = ExecutionContext::new(&bundle);
        for policy in ["default", "nodes-zero", "bytes-zero", "cumulative-zero"] {
            let mut limits = ExecutionLimits::default();
            match policy {
                "nodes-zero" => limits.max_value_nodes = 0,
                "bytes-zero" => limits.scalar.values.max_value_bytes = 0,
                "cumulative-zero" => limits.scalar.max_value_bytes = 0,
                _ => {}
            }
            for owned in [false, true] {
                let mut session = if owned {
                    EvaluationSession::new_owned(
                        image.clone(),
                        args.clone(),
                        bundle.clone(),
                        vec![],
                        None,
                        limits.clone(),
                    )
                    .unwrap()
                } else {
                    EvaluationSession::new(&image, &args, &context, &limits).unwrap()
                };
                let mut terminal = None;
                let mut polls = 0;
                for step in 0..100000 {
                    match session.resume(NonZeroUsize::MIN) {
                        SessionProgress::Pending => {}
                        SessionProgress::Complete(value) => {
                            terminal = Some(*value);
                            polls = step + 1;
                            break;
                        }
                        SessionProgress::Finished => panic!("early Finished"),
                    }
                }
                let result = terminal.expect("finite deadline");
                if policy == "default" && request["intended_positive"] == true {
                    assert_eq!(
                        serde_json::to_value(&result.as_ref().unwrap().value).unwrap(),
                        request["literal_expected_echo"]
                    );
                }
                assert!(matches!(
                    session.resume(NonZeroUsize::MIN),
                    SessionProgress::Finished
                ));
                rows.push(json!({"id":id,"route":"session","policy":policy,"owned":owned,"polls":polls,"terminal":result}));
            }
        }
        let contract = Contract::function(&bundle, &entry, &selection).unwrap();
        let calls = Rc::new(Cell::new(0));
        let seen = calls.clone();
        let provider = NativeProvider::new(
            &contract,
            "verification:shape-owner:v1",
            move |_: &intersect_model::semantic_bindings::Call<'_>, args: &[ValueWire]| {
                seen.set(seen.get() + 1);
                Ok(args[0].clone())
            },
        );
        let mut registry = Registry::default();
        registry.install(&contract, Rc::new(provider)).unwrap();
        let pinned = registry.pin(&contract).unwrap();
        let limits = ExecutionLimits::default();
        let budget = Budget::new(1_000_000_000);
        let mut binding = pinned.start(&args, &context, &limits, budget).unwrap();
        let mut terminal = None;
        let mut polls = 0;
        for step in 0..100000 {
            match binding.resume(NonZeroUsize::MIN) {
                BindingProgress::Pending => {}
                BindingProgress::Complete(value) => {
                    terminal = Some(*value);
                    polls = step + 1;
                    break;
                }
                BindingProgress::Finished => panic!("early binding Finished"),
            }
        }
        let result = terminal.expect("finite binding deadline");
        if request["intended_positive"] == true {
            assert_eq!(
                serde_json::to_value(&result.as_ref().unwrap().evaluation.value).unwrap(),
                request["literal_expected_echo"]
            );
            assert_eq!(calls.get(), 1)
        } else {
            assert_eq!(calls.get(), 0)
        }
        assert!(matches!(
            binding.resume(NonZeroUsize::MIN),
            BindingProgress::Finished
        ));
        rows.push(json!({"id":id,"route":"binding","polls":polls,"provider_calls":calls.get(),"terminal":result}));
    }
    #[cfg(feature = "allocation-profile")]
    let layout = json!({"root":EvaluationSession::retained_root_layouts(),"binding":EvaluationSession::retained_binding_root_layouts(),"shape":EvaluationSession::retained_shape_layouts(),"scalar":EvaluationSession::retained_scalar_validation_layouts()});
    #[cfg(not(feature = "allocation-profile"))]
    let layout = json!({"session":std::mem::size_of::<EvaluationSession<'static>>(),"binding":std::mem::size_of::<intersect_model::semantic_bindings::BindingSession<'static>>(),"old_owned_box_headers":[std::mem::size_of::<intersect_model::definitions::rcore_behavior::ir_v2::VerifiedExecutable>(),std::mem::size_of::<DefinitionBundle>(),std::mem::size_of::<ExecutionLimits>()]});
    json!({"layout":layout,"rows":rows})
}
