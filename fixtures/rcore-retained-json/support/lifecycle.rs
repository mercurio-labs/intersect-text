// Verification only: actual shared Session, exact authored programs and literal outputs.
use intersect_model::{
    definitions::{
        bundle::DefinitionBundle,
        rcore_behavior::{
            function_lowering::compile_function,
            ir_v2::{Limits, ValueWire},
        },
    },
    rcore_behavior_functions::{
        EvaluationSession, ExecutionContext, ExecutionControl, ExecutionLimits, SessionProgress,
    },
};
use serde_json::{Value, json};
use std::{cell::Cell, num::NonZeroUsize};
struct Cancel(Cell<usize>);
impl ExecutionControl for Cancel {
    fn cancellation_requested(&self) -> bool {
        self.0.set(self.0.get() + 1);
        true
    }
}
fn make<'a>(
    owned: bool,
    b: &'a DefinitionBundle,
    i: &'a intersect_model::definitions::rcore_behavior::ir_v2::VerifiedExecutable,
    a: &'a [ValueWire],
    l: &'a ExecutionLimits,
) -> EvaluationSession<'a> {
    if owned {
        EvaluationSession::new_owned(i.clone(), a.to_vec(), b.clone(), vec![], None, l.clone())
            .unwrap()
    } else {
        EvaluationSession::new(i, a, &ExecutionContext::new(b), l).unwrap()
    }
}
fn finish(mut s: EvaluationSession<'_>, expected: &Value) -> Value {
    for _ in 0..100000 {
        match s.resume(NonZeroUsize::new(256).unwrap()) {
            SessionProgress::Pending => {}
            SessionProgress::Complete(v) => {
                let value = serde_json::to_value(v.as_ref().as_ref().unwrap()).unwrap();
                assert_eq!(&value, expected);
                assert!(matches!(
                    s.resume(NonZeroUsize::MIN),
                    SessionProgress::Finished
                ));
                return value;
            }
            SessionProgress::Finished => panic!("early Finished"),
        }
    }
    panic!("finite verification deadline")
}
pub fn observe() -> Value {
    let cases: Vec<Value> = serde_json::from_str(include_str!("../lifecycle-cases.json")).unwrap();
    let mut rows = Vec::new();
    for c in cases {
        let r = &c["request"];
        let b: DefinitionBundle = serde_json::from_value(r["bundle"].clone()).unwrap();
        let entry = serde_json::from_value(r["entry"].clone()).unwrap();
        let selection = serde_json::from_value(r["selection"].clone()).unwrap();
        let image = compile_function(&b, &entry, &selection, &Limits::default()).unwrap();
        assert_eq!(
            serde_json::to_value(image.program()).unwrap(),
            r["expected_program"]
        );
        let args: Vec<ValueWire> = serde_json::from_value(r["arguments"].clone()).unwrap();
        let limits = ExecutionLimits::default();
        let positive = c["id"] == "bytes_large";
        for owned in [false, true] {
            if positive {
                let mut s = make(owned, &b, &image, &args, &limits);
                let mut offsets = Vec::new();
                let mut terminal = None;
                for _ in 0..100000 {
                    let p = s.resume(NonZeroUsize::MIN);
                    if let Some(("hex", at, len)) = s.retained_scalar_validation() {
                        assert!(matches!(p, SessionProgress::Pending));
                        assert_eq!(len, 240000);
                        offsets.push(at);
                        let own = s.retained_arena_ownership();
                        assert_eq!(&own[1..3], &[0, 0]);
                        assert_eq!(own[4], 0);
                    }
                    match p {
                        SessionProgress::Pending => {}
                        SessionProgress::Complete(v) => {
                            terminal = Some(*v);
                            break;
                        }
                        SessionProgress::Finished => panic!("early Finished"),
                    }
                }
                let actual = serde_json::to_value(terminal.unwrap().unwrap()).unwrap();
                assert_eq!(actual, c["complete"]["evaluation"]);
                assert_eq!(actual["value"], c["literal_expected_echo"]);
                assert_eq!(offsets, (0..240000).step_by(256).collect::<Vec<_>>());
                rows.push(json!({"id":c["id"],"owned":owned,"action":"all-hex-boundaries","offsets":offsets,"evaluation":actual}));
            }
            let points: &[usize] = if positive {
                &[0, 256, 120064, 239872]
            } else {
                &[0]
            };
            for &target in points {
                for action in ["cancel", "drop"] {
                    let mut s = make(owned, &b, &image, &args, &limits);
                    let mut reached = false;
                    for _ in 0..100000 {
                        assert!(matches!(
                            s.resume(NonZeroUsize::MIN),
                            SessionProgress::Pending
                        ));
                        if s.retained_scalar_validation() == Some(("hex", target, 240000)) {
                            reached = true;
                            break;
                        }
                    }
                    assert!(reached);
                    let before = s.retained_arena_ownership();
                    assert_eq!(&before[1..3], &[0, 0]);
                    assert_eq!(before[4], 0);
                    let cancellation = if action == "cancel" {
                        let control = Cancel(Cell::new(0));
                        let SessionProgress::Complete(result) =
                            s.resume_with_control(NonZeroUsize::MAX, &control)
                        else {
                            panic!("cancel before next hex unit")
                        };
                        let error = result.unwrap_err();
                        assert_eq!(error.kind,intersect_model::rcore_behavior_functions::ExecutionErrorKind::Cancelled);
                        assert_eq!(error.consumed.value_bytes, before[0]);
                        assert_eq!(error.consumed.value_nodes, 0);
                        assert_eq!(
                            s.retained_scalar_validation(),
                            Some(("hex", target, 240000))
                        );
                        let calls = control.0.get();
                        assert!(calls > 0);
                        assert!(matches!(
                            s.resume_with_control(NonZeroUsize::MAX, &control),
                            SessionProgress::Finished
                        ));
                        assert_eq!(control.0.get(), calls);
                        serde_json::to_value(error).unwrap()
                    } else {
                        Value::Null
                    };
                    drop(s);
                    let recovery = if positive {
                        finish(
                            make(owned, &b, &image, &args, &limits),
                            &c["complete"]["evaluation"],
                        )
                    } else {
                        let mut retry = make(owned, &b, &image, &args, &limits);
                        let mut result = None;
                        for _ in 0..100000 {
                            match retry.resume(NonZeroUsize::new(256).unwrap()) {
                                SessionProgress::Pending => {}
                                SessionProgress::Complete(v) => {
                                    result = Some(*v);
                                    break;
                                }
                                SessionProgress::Finished => panic!("early retry Finished"),
                            }
                        }
                        let failure = serde_json::to_value(result.unwrap().unwrap_err()).unwrap();
                        assert_eq!(failure, c["complete"]["failure"]);
                        failure
                    };
                    rows.push(json!({"id":c["id"],"owned":owned,"action":action,"offset":target,"arena_before":before,"cancellation":cancellation,"recovery":recovery}));
                }
            }
            if !positive {
                let mut s = make(owned, &b, &image, &args, &limits);
                for _ in 0..100000 {
                    assert!(matches!(
                        s.resume(NonZeroUsize::MIN),
                        SessionProgress::Pending
                    ));
                    if s.retained_scalar_validation() == Some(("hex", 0, 240000)) {
                        break;
                    }
                }
                let SessionProgress::Complete(v) = s.resume(NonZeroUsize::MIN) else {
                    panic!("actual canonical error")
                };
                let failure = serde_json::to_value(v.unwrap_err()).unwrap();
                assert_eq!(failure, c["complete"]["failure"]);
                let control = Cancel(Cell::new(0));
                assert!(matches!(
                    s.resume_with_control(NonZeroUsize::MAX, &control),
                    SessionProgress::Finished
                ));
                assert_eq!(control.0.get(), 0);
                rows.push(json!({"id":c["id"],"owned":owned,"action":"error-before-later-checkpoint","failure":failure}));
            }
        }
    }
    json!({"rows":rows,"layout":EvaluationSession::retained_scalar_validation_layouts()})
}
