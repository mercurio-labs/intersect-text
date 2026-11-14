//! Direct, unassigned parser dispatch uses one real rule frame repeatedly.
//! All syntax/model execution still uses the canonical Machine methods. Pending
//! document/checkpoint/live-return continuations are explicit, paid and bounded;
//! they do not grant a deeper recursive rule or runtime-prediction allowance.
use super::*;

type DirectDispatchTarget<'a> = (&'a Expr, &'a str, Option<&'a [String]>, grammar::Profile);

struct LeadingSequenceTarget<'a> {
    rule: &'a grammar::Rule,
    grammar: &'a grammar::Grammar,
    first: &'a Expr,
    suffix: &'a [Expr],
    target: &'a str,
}
enum DispatchTarget<'a> {
    Direct(DirectDispatchTarget<'a>),
    Sequence(LeadingSequenceTarget<'a>),
}
// A suspended cardinality-one expr owns its original checkpoint and both
// canonical once/live envelopes. The outer Sequence and first Call are distinct.
struct SuspendedExpr<'a> {
    expression: &'a Expr,
    checkpoint: Checkpoint,
    previous_current: Option<Current>,
    document: Option<OnceDocumentPhase<'a>>,
    live: bool,
}
struct LeadingSequenceFrame<'a> {
    outer: SuspendedExpr<'a>,
    first: SuspendedExpr<'a>,
    suffix: &'a [Expr],
    current: Option<Current>,
    rule_phase: Option<CurrentRulePhase<'a>>,
    previous_hidden: Option<&'a [String]>,
    previous_generic: bool,
    physical_depth: usize,
    profile: grammar::Profile,
}
// A recovered entry can already complete the rule before a child is suspended.
// This temporary uses the same explicitly bounded inline frame, not a new Box.
#[allow(clippy::large_enum_variant)]
enum SequenceStart<'a> {
    Suspended(LeadingSequenceFrame<'a>),
    Finished(Value),
}
// Keep the paid bounded contiguous slot rather than adding an unobserved Box
// allocation for the larger Sequence variant. Both variants fit FRAME_BYTES.
#[allow(clippy::large_enum_variant)]
enum RuleContinuation<'a> {
    Direct(DirectDispatchFrame<'a>),
    Sequence(LeadingSequenceFrame<'a>),
}

struct DirectDispatchFrame<'a> {
    expression: &'a Expr,
    checkpoint: Checkpoint,
    document: Option<OnceDocumentPhase<'a>>,
    previous_hidden: Option<&'a [String]>,
    previous_generic: bool,
    physical_depth: usize,
    profile: grammar::Profile,
}
// Portable logical slots, including the actual checkpoint/document descriptor.
const FRAME_BYTES: usize = 1024;
const VEC_BYTES: usize = 32;
const _: () = assert!(std::mem::size_of::<RuleContinuation<'_>>() <= FRAME_BYTES);
const _: () = assert!(std::mem::size_of::<Vec<RuleContinuation<'_>>>() <= VEC_BYTES);

impl<'a, S: ModelStore, C: draft::ParseContext<S>> Machine<'a, '_, S, C> {
    fn direct_lookup_work(&mut self, rows: usize, name: &str) -> Result<(), ParseError> {
        let units = rows
            .max(1)
            .checked_mul(1 + name.len().div_ceil(64))
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| ParseError::Budget("Direct dispatch lookup work overflow".into()))?;
        for _ in 0..units {
            self.step()?;
        }
        Ok(())
    }
    fn direct_dispatch_target(
        &mut self,
        name: &str,
    ) -> Result<Option<DispatchTarget<'a>>, ParseError> {
        let package = self.package;
        let Some(grammar) = package.grammar() else {
            return Ok(None);
        };
        if !grammar.profile.has_constant_guards() {
            return Ok(None);
        }
        self.direct_lookup_work(grammar.rules.len(), name)?;
        let Some(rule) = grammar.rules.get(name) else {
            return Ok(None);
        };
        for _ in 0..5 {
            self.step()?;
        }
        if rule.fragment || rule.datatype.is_some() || rule.body.cardinality != '1' {
            return Ok(None);
        }
        let sequence = if let Kind::Sequence(items) = &rule.body.kind {
            self.step()?;
            let Some((first, suffix)) = items.split_first() else {
                return Ok(None);
            };
            if suffix.is_empty() || first.cardinality != '1' {
                return Ok(None);
            }
            Some((first, suffix))
        } else {
            None
        };
        let called = sequence.map_or(&rule.body, |(first, _)| first);
        let Kind::Call(target) = &called.kind else {
            return Ok(None);
        };
        self.direct_lookup_work(grammar.rules.len(), target)?;
        let Some(child) = grammar.rules.get(target) else {
            return Ok(None);
        };
        self.direct_lookup_work(grammar.terminals.len(), target)?;
        for _ in 0..5 + 3 * (1 + target.len().div_ceil(64)) {
            self.step()?;
        }
        if child.fragment
            || child.datatype.is_some()
            || grammar.terminals.contains_key(target)
            || ["ID", "STRING", "INT"].contains(&target.as_str())
        {
            return Ok(None);
        }
        Ok(Some(match sequence {
            Some((first, suffix)) => DispatchTarget::Sequence(LeadingSequenceTarget {
                rule,
                grammar,
                first,
                suffix,
                target,
            }),
            None => DispatchTarget::Direct((
                &rule.body,
                target,
                rule.hidden.as_deref(),
                grammar.profile,
            )),
        }))
    }

    fn admit_direct_frame(
        &mut self,
        frames: &mut Vec<RuleContinuation<'a>>,
    ) -> Result<(), ParseError> {
        // This bounds explicit continuations, not a new recursive depth grant.
        // Nontransparent recursion still runs rule_body's literal64 check;
        // actual live caller returns retain their separate literal128 check.
        if frames.len() >= self.limits.nesting {
            return Err(ParseError::Budget("Model nesting budget exceeded".into()));
        }
        let capacity = frames
            .len()
            .checked_add(1)
            .ok_or_else(|| ParseError::Budget("Rule continuation count overflow".into()))?;
        let bytes = capacity
            .checked_mul(FRAME_BYTES)
            .and_then(|n| n.checked_add(VEC_BYTES))
            .ok_or_else(|| ParseError::Budget("Rule continuation size overflow".into()))?;
        let moved = frames
            .len()
            .checked_mul(FRAME_BYTES)
            .and_then(|n| n.checked_add(bytes))
            .ok_or_else(|| ParseError::Budget("Rule continuation move overflow".into()))?;
        // Pay fresh allocation/moves/drop descriptors before reservation.
        for _ in 0..4usize
            .checked_add(moved.div_ceil(64))
            .ok_or_else(|| ParseError::Budget("Rule continuation work overflow".into()))?
        {
            self.step()?;
        }
        if let Some(document) = self.document.as_mut() {
            document
                .admit_rule_continuation(bytes)
                .map_err(ParseError::Storage)?;
        }
        frames
            .try_reserve_exact(1)
            .map_err(|_| ParseError::Storage("Rule continuation allocation failed".into()))?;
        self.step()?;
        if frames.capacity() != capacity {
            return Err(ParseError::Storage(
                "Rule continuation capacity differs from paid slots".into(),
            ));
        }
        Ok(())
    }

    fn begin_direct_frame(
        &mut self,
        expression: &'a Expr,
        policy: Option<&'a [String]>,
        profile: grammar::Profile,
    ) -> Result<DirectDispatchFrame<'a>, ParseError> {
        let previous_hidden = self.enter_hidden(policy)?;
        let physical_depth = self.depth;
        let result = (|| {
            self.depth = self
                .depth
                .checked_add(1)
                .ok_or_else(|| ParseError::Budget("Model nesting budget exceeded".into()))?;
            if self.depth > self.limits.nesting {
                return Err(ParseError::Budget("Model nesting budget exceeded".into()));
            }
            self.skip_hidden()?; // canonical rule_body prelude
            self.step()?; // canonical cardinality1 expr entry
            let checkpoint = self.checkpoint()?;
            let mut owner = None;
            let mut live = false;
            let mut document = None;
            let entered = (|| {
                if C::LIVE_CALLERS {
                    self.live_call_event(expression, true)?;
                    live = true;
                }
                document = self.begin_once_document(expression, &mut owner)?;
                self.skip_hidden()?; // canonical once_body prelude
                Ok(())
            })();
            if let Err(error) = entered {
                let mut failed =
                    self.finish_once_document(expression, &mut owner, document, Err(error));
                if live {
                    let closed = self.live_call_event(expression, false);
                    if failed.is_ok()
                        && let Err(error) = closed
                    {
                        failed = Err(error);
                    }
                }
                return match self
                    .finish_expr_attempt(expression, &mut owner, checkpoint, None, 0, false, failed)
                {
                    Err(error) => Err(error),
                    Ok(_) => Err(ParseError::Storage(
                        "Invalid direct dispatch entry completion".into(),
                    )),
                };
            }
            let previous_generic = self.generic_fragment;
            self.generic_fragment = false;
            Ok(DirectDispatchFrame {
                expression,
                checkpoint,
                document,
                previous_hidden,
                previous_generic,
                physical_depth,
                profile,
            })
        })();
        if result.is_err() {
            self.restore_hidden(previous_hidden)?;
        }
        result
    }

    fn finish_direct_frame(
        &mut self,
        frame: DirectDispatchFrame<'a>,
        result: Result<Value, ParseError>,
    ) -> Result<Value, ParseError> {
        // Re-enter the one physical frame for completion, not for execution.
        self.depth = frame
            .physical_depth
            .checked_add(1)
            .ok_or_else(|| ParseError::Budget("Model nesting budget exceeded".into()))?;
        self.generic_fragment = frame.previous_generic;
        let mut owner = None;
        let result = result.and_then(|value| {
            if !value.is_null() {
                owner = Some(Current::called(self.current_index(&value)?));
            }
            Ok(value)
        });
        let result =
            self.finish_once_document(frame.expression, &mut owner, frame.document, result);
        let result = if C::LIVE_CALLERS {
            let closed = self.live_call_event(frame.expression, false);
            match (result, closed) {
                (Err(original), _) => Err(original),
                (_, Err(error)) => Err(error),
                (Ok(value), Ok(())) => Ok(value),
            }
        } else {
            result
        };
        let result = self
            .finish_expr_attempt(
                frame.expression,
                &mut owner,
                frame.checkpoint,
                None,
                0,
                false,
                result,
            )
            .and_then(|value| {
                value.ok_or_else(|| ParseError::Storage("Missing direct dispatch result".into()))
            });
        let result = self.finish_forwarding_rule(owner, result, frame.profile);
        if result.is_ok() {
            self.depth -= 1;
        }
        self.restore_hidden(frame.previous_hidden)?;
        result
    }

    fn begin_suspended_expr(
        &mut self,
        expression: &'a Expr,
        owner: &mut Option<Current>,
        live: bool,
    ) -> Result<SuspendedExpr<'a>, ParseError> {
        // Literal expr cardinality-one prelude, including its own savepoint.
        self.step()?;
        let checkpoint = self.checkpoint()?;
        let previous_current = *owner;
        let mut document = None;
        let mut document_started = false;
        let mut live_started = false;
        let entered = (|| {
            if live {
                self.live_call_event(expression, true)?;
                live_started = true;
            }
            document = self.begin_once_document(expression, owner)?;
            document_started = true;
            self.skip_hidden()?; // SAME once_body prelude
            Ok(())
        })();
        if let Err(error) = entered {
            let mut failed = if document_started {
                self.finish_once_document(expression, owner, document, Err(error))
            } else {
                Err(error)
            };
            if live_started {
                let closed = self.live_call_event(expression, false);
                if failed.is_ok()
                    && let Err(error) = closed
                {
                    failed = Err(error);
                }
            }
            return match self.finish_expr_attempt(
                expression,
                owner,
                checkpoint,
                previous_current,
                0,
                false,
                failed,
            ) {
                Err(error) => Err(error),
                Ok(_) => Err(ParseError::Storage(
                    "Invalid leading Sequence expression entry completion".into(),
                )),
            };
        }
        Ok(SuspendedExpr {
            expression,
            checkpoint,
            previous_current,
            document,
            live,
        })
    }

    fn finish_suspended_expr(
        &mut self,
        phase: SuspendedExpr<'a>,
        owner: &mut Option<Current>,
        result: Result<Value, ParseError>,
    ) -> Result<Value, ParseError> {
        let result = self.finish_once_document(phase.expression, owner, phase.document, result);
        let result = if phase.live {
            let closed = self.live_call_event(phase.expression, false);
            match (result, closed) {
                (Err(original), _) => Err(original),
                (_, Err(error)) => Err(error),
                (Ok(value), Ok(())) => Ok(value),
            }
        } else {
            result
        };
        self.finish_expr_attempt(
            phase.expression,
            owner,
            phase.checkpoint,
            phase.previous_current,
            0,
            false,
            result,
        )?
        .ok_or_else(|| ParseError::Storage("Missing leading Sequence expression result".into()))
    }

    fn finish_sequence_rule(
        &mut self,
        phase: Option<CurrentRulePhase<'a>>,
        current: Option<Current>,
        result: Result<Value, ParseError>,
        profile: grammar::Profile,
    ) -> Result<Value, ParseError> {
        if let Some(mut phase) = phase {
            phase.current = current;
            self.finish_current_rule_phase(phase, result)
        } else {
            let result = self.finish_forwarding_rule(current, result, profile);
            if result.is_ok() {
                self.depth -= 1;
            }
            result
        }
    }

    fn begin_sequence_frame(
        &mut self,
        target: LeadingSequenceTarget<'a>,
    ) -> Result<SequenceStart<'a>, ParseError> {
        let previous_hidden = self.enter_hidden(target.rule.hidden.as_deref())?;
        let physical_depth = self.depth;
        let undo_start = self.undo.len();
        let mut rule_phase = None;
        let mut current = None;
        let mut outer = None;
        let mut completion_ready = false;
        let result = (|| {
            self.depth = self
                .depth
                .checked_add(1)
                .ok_or_else(|| ParseError::Budget("Model nesting budget exceeded".into()))?;
            if self.depth > self.limits.nesting {
                return Err(ParseError::Budget("Model nesting budget exceeded".into()));
            }
            self.skip_hidden()?; // SAME rule_body prelude
            if !target.rule.is_dispatch(target.grammar) {
                let phase =
                    self.begin_current_rule_phase(target.rule, target.grammar, undo_start)?;
                current = phase.current;
                rule_phase = Some(phase);
            }
            completion_ready = true;
            outer = Some(self.begin_suspended_expr(&target.rule.body, &mut current, false)?);
            let first = self.begin_suspended_expr(target.first, &mut current, C::LIVE_CALLERS)?;
            Ok(first)
        })();
        match result {
            Ok(first) => {
                let outer = outer.ok_or_else(|| {
                    ParseError::Storage("Missing leading Sequence outer expression".into())
                })?;
                let previous_generic = self.generic_fragment;
                self.generic_fragment = false; // SAME ordinary Call prelude
                Ok(SequenceStart::Suspended(LeadingSequenceFrame {
                    outer,
                    first,
                    suffix: target.suffix,
                    current,
                    rule_phase,
                    previous_hidden,
                    previous_generic,
                    physical_depth,
                    profile: target.grammar.profile,
                }))
            }
            Err(error) => {
                let mut failed = Err(error);
                if let Some(outer) = outer {
                    failed = self.finish_suspended_expr(outer, &mut current, failed);
                }
                if completion_ready {
                    failed = self.finish_sequence_rule(
                        rule_phase,
                        current,
                        failed,
                        target.grammar.profile,
                    );
                }
                self.restore_hidden(previous_hidden)?;
                failed.map(SequenceStart::Finished)
            }
        }
    }

    fn finish_sequence_frame(
        &mut self,
        mut frame: LeadingSequenceFrame<'a>,
        result: Result<Value, ParseError>,
    ) -> Result<Value, ParseError> {
        self.depth = frame
            .physical_depth
            .checked_add(1)
            .ok_or_else(|| ParseError::Budget("Model nesting budget exceeded".into()))?;
        self.generic_fragment = frame.previous_generic;
        // Ordinary Call adopts the actual child, before closing that Call's
        // document/live envelope. A suffix Capture may then replace this owner.
        let result = result.and_then(|value| {
            if !value.is_null() {
                frame.current = Some(Current::called(self.current_index(&value)?));
            }
            Ok(value)
        });
        let result = self.finish_suspended_expr(frame.first, &mut frame.current, result);
        let result = result.and_then(|_| {
            for expression in frame.suffix {
                self.expr(expression, &mut frame.current)?;
            }
            // Literal once_body(Sequence) returns Null, not the child result.
            Ok(Value::Null)
        });
        let result = self.finish_suspended_expr(frame.outer, &mut frame.current, result);
        let result =
            self.finish_sequence_rule(frame.rule_phase, frame.current, result, frame.profile);
        self.restore_hidden(frame.previous_hidden)?;
        result
    }

    pub(super) fn rule_with_direct_dispatch(&mut self, name: &str) -> Result<Value, ParseError> {
        let mut frames = Vec::new();
        let mut selected = name;
        let mut result = loop {
            let target = match self.direct_dispatch_target(selected) {
                Ok(Some(target)) => target,
                Ok(None) => break self.rule_literal(selected),
                Err(error) => break Err(error),
            };
            if let Err(error) = self.admit_direct_frame(&mut frames) {
                break Err(error);
            }
            let frame = match target {
                DispatchTarget::Direct((expression, target, policy, profile)) => {
                    selected = target;
                    self.begin_direct_frame(expression, policy, profile)
                        .map(RuleContinuation::Direct)
                }
                DispatchTarget::Sequence(target) => {
                    selected = target.target;
                    match self.begin_sequence_frame(target) {
                        Ok(SequenceStart::Suspended(frame)) => {
                            Ok(RuleContinuation::Sequence(frame))
                        }
                        Ok(SequenceStart::Finished(value)) => break Ok(value),
                        Err(error) => Err(error),
                    }
                }
            };
            match frame {
                Ok(frame) => {
                    // Actual call replacement: retain the real continuation;
                    // do not recurse into rule() for its leading child.
                    self.depth = match &frame {
                        RuleContinuation::Direct(frame) => frame.physical_depth,
                        RuleContinuation::Sequence(frame) => frame.physical_depth,
                    };
                    frames.push(frame);
                }
                Err(error) => break Err(error),
            }
        };
        while let Some(frame) = frames.pop() {
            result = match frame {
                RuleContinuation::Direct(frame) => self.finish_direct_frame(frame, result),
                RuleContinuation::Sequence(frame) => self.finish_sequence_frame(frame, result),
            };
        }
        result
    }
}
