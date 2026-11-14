//! Verification consumer only: real registered providers through the ordinary decoder.
use intersect_model::{
    definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        compiler::import_datatype_bundle,
        conversion::{
            ConversionContract, ConversionProvider, ConversionRegistry,
            ConversionValidationAdapter, ConversionValidationCursor, ConversionValidationStep,
            ConvertedValue, InvocationContext, ProviderId,
        },
        rcore_behavior::{
            function_lowering::compile_function_with_conversions,
            ir_v2::{DataWire, EntrySelection, Limits, ValueWire},
            modules::SymbolId,
        },
        resources::ResourceSet,
    },
    rcore_behavior_functions::{
        EvaluationSession, ExecutionContext, ExecutionControl, ExecutionLimits, SessionProgress,
    },
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Good,
    BeginError,
    StepError,
    Underdeclared,
    SwallowWork,
    SwallowCancel,
}
struct Provider {
    contract: ConversionContract,
    mode: Cell<Mode>,
    calls: RefCell<Vec<Value>>,
    cancel: Cell<bool>,
    active: Cell<usize>,
    created: Cell<usize>,
    released: Cell<usize>,
}
impl ConversionProvider for Provider {
    fn contract(&self) -> &ConversionContract {
        &self.contract
    }
    fn retained_validation(&self) -> Option<&dyn ConversionValidationAdapter> {
        Some(self)
    }
    fn parse(&self, _: &str) -> Result<Value, String> {
        Err("No lexical conversion in edge fixture".into())
    }
    fn format(&self, _: &Value) -> Result<String, String> {
        Err("No formatting in edge fixture".into())
    }
    fn default_value(&self) -> Result<Option<Value>, String> {
        Ok(None)
    }
    fn validate(&self, _: &Value) -> Result<(), String> {
        Err("Unexpected opaque validation".into())
    }
}
struct Cursor<'a> {
    owner: &'a Provider,
    value: &'a str,
    offset: usize,
}
impl Drop for Cursor<'_> {
    fn drop(&mut self) {
        self.owner.active.set(self.owner.active.get() - 1);
        self.owner.released.set(self.owner.released.get() + 1);
    }
}
impl ConversionValidationCursor for Cursor<'_> {
    fn advance(
        &mut self,
        context: &InvocationContext<'_>,
    ) -> Result<ConversionValidationStep, String> {
        context.checkpoint()?;
        context.work(1)?;
        self.owner
            .calls
            .borrow_mut()
            .push(json!({"phase":"step","offset":self.offset,"observed":context.has_observer()}));
        if self.offset == 1 {
            match self.owner.mode.get() {
                Mode::StepError => return Err("Segmented validation rejected second byte".into()),
                Mode::SwallowWork => {
                    let refused = context.work(1_048_577);
                    assert!(refused.is_err());
                    return Ok(ConversionValidationStep::Complete);
                }
                Mode::SwallowCancel => {
                    self.owner.cancel.set(true);
                    assert!(context.checkpoint().is_err());
                    return Ok(ConversionValidationStep::Complete);
                }
                _ => {}
            }
        }
        self.offset += 1;
        Ok(if self.offset == self.value.len() {
            ConversionValidationStep::Complete
        } else {
            ConversionValidationStep::Pending
        })
    }
}
impl ConversionValidationAdapter for Provider {
    fn cursor_bytes(&self) -> usize {
        if self.mode.get() == Mode::Underdeclared {
            0
        } else {
            32
        }
    }
    fn begin<'a>(
        &'a self,
        value: &'a Value,
        context: &InvocationContext<'_>,
    ) -> Result<Box<dyn ConversionValidationCursor + 'a>, String> {
        context.checkpoint()?;
        self.calls
            .borrow_mut()
            .push(json!({"phase":"begin","observed":context.has_observer()}));
        if self.mode.get() == Mode::BeginError {
            return Err("Segmented validation factory rejected input".into());
        }
        let value = value
            .as_str()
            .ok_or("Segmented validation requires String payload")?;
        self.active.set(self.active.get() + 1);
        self.created.set(self.created.get() + 1);
        Ok(Box::new(Cursor {
            owner: self,
            value,
            offset: 0,
        }))
    }
}
struct Current<'a> {
    provider: &'a Provider,
    polls: Cell<usize>,
}
impl ExecutionControl for Current<'_> {
    fn cancellation_requested(&self) -> bool {
        self.polls.set(self.polls.get() + 1);
        self.provider.cancel.get()
    }
}
fn moved<'a>(value: EvaluationSession<'a>) -> EvaluationSession<'a> {
    let mut v = vec![value];
    v.pop().unwrap()
}
fn terminal(
    value: &mut EvaluationSession<'_>,
    provider: &Provider,
    q: NonZeroUsize,
) -> (Value, usize, usize) {
    let mut polls = 0;
    let mut observations = 0;
    loop {
        let control = Current {
            provider,
            polls: Cell::new(0),
        };
        let result = value.resume_with_control(q, &control);
        polls += 1;
        observations += control.polls.get();
        match result {
            SessionProgress::Pending => {}
            SessionProgress::Complete(result) => {
                let expected = serde_json::to_value(result).unwrap();
                let silent = Current {
                    provider,
                    polls: Cell::new(0),
                };
                assert!(matches!(
                    value.resume_with_control(q, &silent),
                    SessionProgress::Finished
                ));
                assert_eq!(silent.polls.get(), 0);
                return (expected, polls, observations);
            }
            SessionProgress::Finished => panic!("Missing complete result"),
        }
        assert!(polls < 100_000);
    }
}
pub fn observe() -> Value {
    let source = DefinitionSource::new(
        "memory:provider-edge.rcore",
        SourceKind::Rcore,
        "rcore 1;package edges version \"1\";metamodel \"urn:provider-edge\" prefix p {datatype TextTree instanceClass \"example.TextTree\" {}}behavior {import model \"urn:provider-edge\" as m;fn echo(value:m.TextTree)->m.TextTree=value;}",
    );
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let declaration = ResourceSet::new(&bundle)
        .unwrap()
        .resolve(&source.id, "#//TextTree")
        .unwrap();
    let datatype = import_datatype_bundle(&bundle, &declaration).unwrap();
    let provider = Rc::new(Provider {
        contract: ConversionContract {
            datatype: declaration.clone(),
            provider: ProviderId {
                name: "test.provider-edge".into(),
                version: "1".into(),
            },
            serializable: true,
            declarations: vec![],
            members: vec![],
        },
        mode: Cell::new(Mode::Good),
        calls: RefCell::new(vec![]),
        cancel: Cell::new(false),
        active: Cell::new(0),
        created: Cell::new(0),
        released: Cell::new(0),
    });
    let mut registry = ConversionRegistry::default();
    registry
        .register_shared(&datatype, provider.clone())
        .unwrap();
    let selection = EntrySelection {
        symbol: SymbolId {
            source: source.id.clone(),
            declaration: 1,
        },
        types: vec![],
        arguments: vec![],
    };
    let image = compile_function_with_conversions(
        &bundle,
        &source.id,
        &selection,
        &registry,
        &Limits::default(),
    )
    .unwrap();
    let ty = image.program().functions[image.program().entry.0 as usize].parameters[0];
    let base = Rc::strong_count(&provider);
    assert_eq!(base, 2);
    let mut cases = vec![];
    for mode in [
        Mode::Good,
        Mode::BeginError,
        Mode::StepError,
        Mode::Underdeclared,
        Mode::SwallowWork,
        Mode::SwallowCancel,
    ] {
        cases.push((
            format!("mode-{mode:?}"),
            mode,
            ExecutionLimits::default(),
            "valid",
        ));
    }
    for limit in 0..=40 {
        let work = ExecutionLimits {
            max_provider_work: limit,
            ..ExecutionLimits::default()
        };
        cases.push((format!("provider-work-{limit}"), Mode::Good, work, "valid"));
        let storage = ExecutionLimits {
            max_model_storage_bytes: limit,
            ..ExecutionLimits::default()
        };
        cases.push((
            format!("provider-storage-{limit}"),
            Mode::Good,
            storage,
            "valid",
        ));
    }
    for malformed in [
        "payload",
        "extra-header",
        "wrong-provider",
        "wrong-declaration",
    ] {
        cases.push((
            format!("malformed-{malformed}"),
            Mode::Good,
            ExecutionLimits::default(),
            malformed,
        ));
    }
    let mut rows = vec![];
    for (name, mode, limits, malformed) in cases {
        let converted = ConvertedValue::Converted {
            datatype: declaration.clone(),
            provider: provider.contract.provider.clone(),
            value: json!("abcd"),
        };
        let mut payload = serde_json::to_value(&converted).unwrap();
        match malformed {
            "payload" => payload["value"] = json!(["wrong"]),
            "extra-header" => payload["unexpected"] = json!(true),
            "wrong-provider" => payload["provider"]["version"] = json!("foreign"),
            "wrong-declaration" => payload["datatype"]["source"]["sha256"] = json!("foreign"),
            _ => {}
        }
        let arguments = vec![ValueWire {
            ty,
            data: DataWire::Datatype(payload),
        }];
        for owned in [false, true] {
            for quantum in [1, 17, 256] {
                provider.mode.set(mode);
                provider.cancel.set(false);
                provider.calls.borrow_mut().clear();
                let before_created = provider.created.get();
                let before_released = provider.released.get();
                let context = ExecutionContext {
                    bundle: &bundle,
                    conversions: Some(&registry),
                    references: &[],
                };
                let created = if owned {
                    EvaluationSession::new_owned(
                        image.clone(),
                        arguments.clone(),
                        bundle.clone(),
                        vec![],
                        Some(registry.clone()),
                        limits.clone(),
                    )
                } else {
                    EvaluationSession::new(&image, &arguments, &context, &limits)
                };
                let (outcome, polls, observations, stage) = match created {
                    Err(error) => (
                        serde_json::to_value(Err::<(), _>(error)).unwrap(),
                        0,
                        0,
                        "initialization",
                    ),
                    Ok(session) => {
                        let mut session = moved(session);
                        let (outcome, polls, observations) =
                            terminal(&mut session, &provider, NonZeroUsize::new(quantum).unwrap());
                        drop(session);
                        (outcome, polls, observations, "execution")
                    }
                };
                assert_eq!(provider.active.get(), 0);
                assert_eq!(
                    provider.created.get() - before_created,
                    provider.released.get() - before_released
                );
                assert_eq!(Rc::strong_count(&provider), base);
                let calls = provider.calls.borrow().clone();
                assert!(calls.iter().all(|c| c["observed"] == true));
                if mode == Mode::SwallowWork {
                    assert_eq!(outcome["Err"]["kind"], "ResourceExhausted")
                }
                if mode == Mode::SwallowCancel {
                    assert_eq!(outcome["Err"]["kind"], "Cancelled")
                }
                if mode == Mode::Underdeclared {
                    assert!(
                        outcome["Err"]
                            .to_string()
                            .contains("exceeds its declared state")
                    )
                }
                if name == "provider-storage-31" {
                    assert!(outcome.get("Err").is_some());
                    assert!(
                        calls.is_empty(),
                        "Factory ran before its cursor storage admission"
                    )
                }
                if name == "provider-storage-32" {
                    assert!(outcome.get("Ok").is_some())
                }
                rows.push(json!({"case":name,"mode":format!("{mode:?}"),"owned":owned,"quantum":quantum,"limits":limits,"stage":stage,"outcome":outcome,"calls":calls,"created":provider.created.get()-before_created,"released":provider.released.get()-before_released,"provider_refs":Rc::strong_count(&provider),"polls":polls,"control_observations":observations}));
            }
        }
    }
    provider.cancel.set(false);
    provider.mode.set(Mode::Good);
    json!({"source":bundle,"program":image.program(),"rows":rows,"layouts":{"root":EvaluationSession::retained_root_layouts(),"binding":EvaluationSession::retained_binding_root_layouts(),"phase":EvaluationSession::retained_phase_layouts()},"physical_cursor_bytes":std::mem::size_of::<Cursor<'_>>(),"logical_cursor_bytes":32,"scope":"Real Session provider factory/cursor failures, malformed metadata, understated state, caught work/cancellation failures, every work/storage limit0..40, borrowed/owned movement and current observation; exact before/after semantic rows. Invalid provider factories can allocate before declared-size rejection; no arbitrary provider physical preemption promise."})
}
