use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    rcore_behavior::{
        function_lowering::compile_function,
        ir,
        ir_v2::{DataWire, EntrySelection, Limits, ValueWire},
        modules::SymbolId,
    },
};
use intersect_model::rcore_behavior_functions::{
    EvaluationSession, ExecutionContext, ExecutionLimits, SessionProgress,
};
use serde_json::{Value, json};
use std::num::NonZeroUsize;
pub fn capture() -> Value {
    let input: Value = serde_json::from_str(include_str!("../inputs/kernels.json")).unwrap();
    let mut rows = Vec::new();
    for row in input.as_array().unwrap() {
        let left: ir::Value = serde_json::from_value(row["left"].clone()).unwrap();
        let ir::ValueType::Scalar(left_kind) = left.ty else {
            panic!("scalar fixture")
        };
        let right: Option<ir::Value> = if row["right"].is_null() {
            None
        } else {
            Some(serde_json::from_value(row["right"].clone()).unwrap())
        };
        let symbol = match row["op"].as_str().unwrap() {
            "Add" => "+",
            "Subtract" => "-",
            "Multiply" => "*",
            "Divide" => "/",
            "Equal" => "==",
            "Not" => "!",
            "Negate" => "-",
            _ => panic!("frozen operation inventory"),
        };
        let comparison = row["op"] == "Equal";
        let result_type = if comparison {
            "Bool".to_string()
        } else {
            format!("{left_kind:?}")
        };
        let parameters = if let Some(right) = &right {
            let ir::ValueType::Scalar(right_kind) = right.ty else {
                panic!("scalar fixture")
            };
            format!("a:{left_kind:?},b:{right_kind:?}")
        } else {
            format!("a:{left_kind:?}")
        };
        let expression = if right.is_some() {
            format!("a{symbol}b")
        } else {
            format!("{symbol}a")
        };
        let text = format!(
            "rcore 1;package numeric_session_proof version \"1\";fn run({parameters})->{result_type}={expression};"
        );
        let definition = DefinitionSource::new(
            format!(
                "memory:s09-v22-typed-kernel-{}.rcore",
                row["id"].as_str().unwrap()
            ),
            SourceKind::RcoreBehavior,
            text,
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
        let image = match compile_function(&bundle, &definition.id, &selection, &Limits::default())
        {
            Ok(image) => image,
            Err(error) => {
                rows.push(json!({"operation":"compile-rejection","case":row["id"],"definition":definition,"selection":selection,"error":error,"input":row}));
                continue;
            }
        };
        let parameters = &image.program().functions[image.program().entry.0 as usize].parameters;
        let mut args = vec![ValueWire {
            ty: parameters[0],
            data: DataWire::Scalar(left),
        }];
        if let Some(right) = right {
            args.push(ValueWire {
                ty: parameters[1],
                data: DataWire::Scalar(right),
            });
        }
        rows.push(json!({"operation":"fixture","case":row["id"],"definition":definition,"selection":selection,"program":image.program(),"input":row}));
        let context = ExecutionContext::new(&bundle);
        for owned in [false, true] {
            for quantum in [1, 3, 256] {
                for policy in ["default", "zero-work", "zero-result", "one-value-byte"] {
                    let mut limits = ExecutionLimits::default();
                    match policy {
                        "zero-work" => limits.scalar.numeric.max_work = 0,
                        "zero-result" => limits.scalar.numeric.max_total_value_bytes = 0,
                        "one-value-byte" => limits.scalar.numeric.max_value_bytes = 1,
                        _ => {}
                    }
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
                    let mut pending = 0;
                    let result = loop {
                        let progress = session.resume(NonZeroUsize::new(quantum).unwrap());
                        match progress {
                            SessionProgress::Pending => {
                                pending += 1;
                                assert!(pending < 100000);
                            }
                            SessionProgress::Complete(result) => break result,
                            SessionProgress::Finished => panic!("missing terminal"),
                        }
                    };
                    if policy == "default" && !row["literal_result"].is_null() {
                        let evaluation = result.as_ref().as_ref().unwrap();
                        let DataWire::Scalar(value) = &evaluation.value.data else {
                            panic!("scalar result")
                        };
                        assert_eq!(serde_json::to_value(value).unwrap(), row["literal_result"]);
                    }
                    if policy == "default" && !row["literal_error"].is_null() {
                        assert!(result.is_err());
                    }
                    assert!(matches!(
                        session.resume(NonZeroUsize::MIN),
                        SessionProgress::Finished
                    ));
                    rows.push(json!({"operation":"evaluation","case":row["id"],"owned":owned,"quantum":quantum,"policy":policy,"limits":limits,"outcome":result,"pending":pending,"finished":true}));
                }
            }
        }
    }
    json!({"rows":rows,"scope":"actual shared typed unary/binary source consumers; compile rejections distinct from execution. Original callback-prefix facade and large input/resource controls retained separately."})
}
