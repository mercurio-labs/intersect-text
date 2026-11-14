// Verification consumer only. The tracked pointer is an allocator observation,
// never an input to the model, session, admission, or semantic implementation.
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    rcore_behavior::{
        function_lowering::compile_function,
        ir,
        ir_v2::{DataWire, EntrySelection, Limits, ValueWire},
        modules::SymbolId,
        types::Scalar,
    },
};
use intersect_model::rcore_behavior_functions::{
    EvaluationSession, ExecutionContext, ExecutionControl, ExecutionErrorKind, ExecutionLimits,
    SessionProgress,
};
use serde_json::{Value, json};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    num::NonZeroUsize,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};
struct WatchedAllocator;
static WATCH: AtomicUsize = AtomicUsize::new(0);
static FREES: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for WatchedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if WATCH
            .compare_exchange(ptr as usize, 0, Relaxed, Relaxed)
            .is_ok()
        {
            FREES.fetch_add(1, Relaxed);
        }
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, size) };
        if !result.is_null()
            && WATCH
                .compare_exchange(ptr as usize, 0, Relaxed, Relaxed)
                .is_ok()
        {
            FREES.fetch_add(1, Relaxed);
        }
        result
    }
}
#[global_allocator]
static ALLOCATOR: WatchedAllocator = WatchedAllocator;
struct Cancel;
impl ExecutionControl for Cancel {
    fn cancellation_requested(&self) -> bool {
        true
    }
}
fn progress(value: &SessionProgress) -> Value {
    match value {
        SessionProgress::Pending => json!({"pending": true}),
        SessionProgress::Complete(result) => json!({"complete": result}),
        SessionProgress::Finished => json!({"finished": true}),
    }
}
fn pointer(value: &ValueWire) -> usize {
    let DataWire::Scalar(value) = &value.data else {
        panic!("literal scalar input")
    };
    let text = value
        .payload
        .as_str()
        .or_else(|| value.payload.get("hex").and_then(Value::as_str))
        .unwrap();
    text.as_ptr() as usize
}
fn matched(
    phase: &Option<intersect_model::rcore_behavior_functions::RetainedNumericPhase>,
    target: &str,
    middle: usize,
) -> bool {
    let Some((side, state, _, _)) = phase else {
        return false;
    };
    let Some((stage, offset, _)) = state else {
        return false;
    };
    match target {
        "left-resource" => *side == "left" && *stage == "resource-size",
        "left-measure-start" => *side == "left" && *stage == "measure" && *offset == 0,
        "left-measure-middle" => *side == "left" && *stage == "measure" && *offset >= middle,
        "right-measure-middle" => *side == "right" && *stage == "measure" && *offset >= middle,
        "kernel" => *side == "kernel",
        "result-measure" => *side == "result" && *stage == "measure",
        _ => false,
    }
}
pub fn capture() -> Value {
    let fixtures: Value = serde_json::from_str(include_str!("../inputs/literals.json")).unwrap();
    let mut rows = Vec::new();
    for row in fixtures.as_array().unwrap() {
        if !matches!(
            row["id"].as_str().unwrap(),
            "string_equal_240000" | "bytes_equal_240000" | "string_escaped_unicode"
        ) {
            continue;
        }
        let source = &row["source"];
        let definition = DefinitionSource::new(
            source["uri"].as_str().unwrap(),
            SourceKind::RcoreBehavior,
            source["text"].as_str().unwrap(),
        );
        assert_eq!(
            serde_json::to_value(&definition.id).unwrap()["sha256"],
            source["sha256"]
        );
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![definition.clone()],
            namespaces: vec![],
        };
        let selection = EntrySelection {
            symbol: SymbolId {
                source: definition.id.clone(),
                declaration: 0,
            },
            types: vec![],
            arguments: vec![],
        };
        let image =
            compile_function(&bundle, &definition.id, &selection, &Limits::default()).unwrap();
        let parameter = image.program().functions[image.program().entry.0 as usize].parameters[0];
        let kind = if row["kind"] == "Bytes" {
            Scalar::Bytes
        } else {
            Scalar::String
        };
        let args: Vec<_> = ["left", "right"]
            .into_iter()
            .map(|side| ValueWire {
                ty: parameter,
                data: DataWire::Scalar(ir::Value {
                    ty: ir::ValueType::Scalar(kind),
                    payload: row[side].clone(),
                }),
            })
            .collect();
        let middle = row["left"]
            .as_str()
            .or_else(|| row["left"].get("hex").and_then(Value::as_str))
            .unwrap()
            .len()
            / 2;
        let limits = ExecutionLimits::default();
        let context = ExecutionContext::new(&bundle);
        rows.push(json!({"operation":"fixture","case":row["id"],"definition":definition,"selection":selection,"program":image.program(),"limits":limits,"literal_expected":row["literal_expected"]}));
        for owned in [false, true] {
            for target in [
                "left-resource",
                "left-measure-start",
                "left-measure-middle",
                "right-measure-middle",
                "kernel",
                "result-measure",
            ] {
                for action in ["cancel", "drop"] {
                    let owner_args = args.clone();
                    let root_pointer = if owned {
                        pointer(&owner_args[0])
                    } else {
                        pointer(&args[0])
                    };
                    FREES.store(0, Relaxed);
                    WATCH.store(root_pointer, Relaxed);
                    let mut session = if owned {
                        EvaluationSession::new_owned(
                            image.clone(),
                            owner_args,
                            bundle.clone(),
                            vec![],
                            None,
                            limits.clone(),
                        )
                        .unwrap()
                    } else {
                        drop(owner_args);
                        EvaluationSession::new(&image, &args, &context, &limits).unwrap()
                    };
                    let mut polls = 0;
                    loop {
                        let phase = session.retained_numeric_phase();
                        if matched(&phase, target, middle) {
                            let before_arena = session.retained_arena_ownership();
                            assert_eq!(
                                FREES.load(Relaxed),
                                0,
                                "root input still retained before selected action"
                            );
                            let outcome = if action == "cancel" {
                                let cancelled =
                                    session.resume_with_control(NonZeroUsize::MIN, &Cancel);
                                let SessionProgress::Complete(result) = &cancelled else {
                                    panic!("cancel before next numeric unit")
                                };
                                let error = result.as_ref().as_ref().unwrap_err();
                                assert_eq!(error.kind, ExecutionErrorKind::Cancelled);
                                let cancelled_value = progress(&cancelled);
                                assert!(matches!(
                                    session.resume(NonZeroUsize::MAX),
                                    SessionProgress::Finished
                                ));
                                cancelled_value
                            } else {
                                json!({"dropped_pending":true})
                            };
                            assert_eq!(
                                FREES.load(Relaxed),
                                0,
                                "terminal state retains root owner until session drop"
                            );
                            drop(session);
                            let root_frees = FREES.load(Relaxed);
                            assert_eq!(root_frees, usize::from(owned));
                            WATCH.store(0, Relaxed);
                            rows.push(json!({"operation":"selected-action","case":row["id"],"owned":owned,"target":target,"action":action,"polls":polls,"actual_numeric_phase":phase,"arena_before":before_arena,"outcome":outcome,"root_input_frees_after_session_drop":root_frees,"root_input_owner":"owned session root or caller-borrowed args; no claim about all descendants/OS/cleanup latency"}));
                            break;
                        }
                        let value = session.resume(NonZeroUsize::MIN);
                        assert!(
                            matches!(value, SessionProgress::Pending),
                            "selected actual phase must exist before completion: {target}"
                        );
                        polls += 1;
                        assert!(polls < 100000);
                    }
                    // Same exact source/image/context after the failure/drop.
                    let mut recovery =
                        EvaluationSession::new(&image, &args, &context, &limits).unwrap();
                    let mut recovery_polls = 0;
                    let terminal = loop {
                        let value = recovery.resume(NonZeroUsize::new(256).unwrap());
                        recovery_polls += 1;
                        if let SessionProgress::Complete(result) = &value {
                            let evaluation = result.as_ref().as_ref().unwrap();
                            let DataWire::Scalar(scalar_value) = &evaluation.value.data else {
                                panic!("Boolean result")
                            };
                            assert_eq!(scalar_value.payload, row["literal_expected"]);
                            break progress(&value);
                        }
                        assert!(matches!(value, SessionProgress::Pending));
                        assert!(recovery_polls < 100000);
                    };
                    assert!(matches!(
                        recovery.resume(NonZeroUsize::MIN),
                        SessionProgress::Finished
                    ));
                    rows.push(json!({"operation":"recovery","case":row["id"],"owned_before":owned,"target":target,"action":action,"polls":recovery_polls,"outcome":terminal}));
                }
            }
        }
    }
    json!({"rows":rows,"scope":"actual typed default session numeric phases, root-input last-owner observation and cancel/drop/recovery; no whole heap/last descendant/OS/cleanup latency or public FunctionJobs claim"})
}
