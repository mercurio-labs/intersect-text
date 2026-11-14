//! Iterative FIRST evaluation with independent syntax and rule-call bounds.
//! Each visited expression consumes the same work unit as recursive analysis.
use super::admission_observation::{
    FirstReuseCharge as ReuseCharge, FirstReuseEvent as ReuseEvent, PredictionObservation,
};
use super::{Expr, Grammar, Kind};
use std::collections::BTreeSet;
type Summary<S = BTreeSet<String>> = (S, bool);
pub(in crate::grammar) mod symbols;
use symbols::Symbols;
pub(in crate::grammar) mod completed_reuse;
#[cfg(test)]
mod completed_reuse_tests;
pub(super) mod prefix_capture;
mod resolved_call;
enum Frame<'a, S: Symbols = BTreeSet<String>> {
    Visit(&'a Expr, usize, usize),
    Cardinality(char),
    CompletedCallReturn {
        rule: &'a super::Rule,
    },
    SourceCallReturn {
        name: &'a str,
        rule: &'a super::Rule,
    },
    Items {
        items: &'a [Expr],
        next: usize,
        syntax: usize,
        calls: usize,
        alternatives: bool,
        tokens: S,
        nullable: bool,
    },
}
pub(super) fn analyze(
    expr: &Expr,
    grammar: &Grammar,
    depth: usize,
    remaining: usize,
) -> Result<Summary, String> {
    analyze_impl(expr, grammar, depth, remaining, None)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct OccurrenceFacts {
    pub body_nullable: Option<bool>,
    pub explicit_prefix_hidden_policy: bool,
}

pub(super) fn analyze_occurrence(
    expr: &Expr,
    grammar: &Grammar,
    depth: usize,
    remaining: usize,
) -> Result<(Summary, OccurrenceFacts), String> {
    let mut facts = OccurrenceFacts::default();
    let result = analyze_impl(expr, grammar, depth, remaining, Some(&mut facts))?;
    Ok((result, facts))
}

pub(super) fn analyze_with_remaining(
    expr: &Expr,
    grammar: &Grammar,
    depth: usize,
    remaining: &mut usize,
) -> Result<Summary, String> {
    analyze_shared(expr, grammar, depth, remaining, None, None)
}

#[cfg(test)]
pub(super) fn analyze_with_remaining_observed(
    expr: &Expr,
    grammar: &Grammar,
    depth: usize,
    remaining: &mut usize,
    observation: &PredictionObservation,
) -> Result<Summary, String> {
    analyze_shared_with_completed::<BTreeSet<String>>(
        expr,
        grammar,
        depth,
        remaining,
        None,
        None,
        (None, Some(observation)),
    )
}

#[cfg(test)]
pub(super) fn analyze_with_completed<'g>(
    expr: &Expr,
    grammar: &'g Grammar,
    depth: usize,
    remaining: &mut usize,
    completed: &mut completed_reuse::Completed<'g>,
) -> Result<Summary, String> {
    analyze_with_completed_observed(expr, grammar, depth, remaining, completed, None)
}

#[cfg(test)]
pub(super) fn analyze_with_completed_observed<'g>(
    expr: &Expr,
    grammar: &'g Grammar,
    depth: usize,
    remaining: &mut usize,
    completed: &mut completed_reuse::Completed<'g>,
    observation: Option<&PredictionObservation>,
) -> Result<Summary, String> {
    analyze_with_storage_observed(expr, grammar, depth, remaining, completed, observation)
}

pub(in crate::grammar) fn analyze_with_storage_observed<'g, S: Symbols>(
    expr: &Expr,
    grammar: &'g Grammar,
    depth: usize,
    remaining: &mut usize,
    completed: &mut completed_reuse::Storage<'g, S>,
    observation: Option<&PredictionObservation>,
) -> Result<Summary<S>, String> {
    let belongs_result = match observation {
        Some(observer) => completed.belongs_observed(grammar, remaining, observer),
        None => completed.belongs(grammar, remaining),
    };
    let belongs = match belongs_result {
        Ok(value) => value,
        Err(error) => {
            completed.fail();
            return Err(error);
        }
    };
    if !belongs {
        return match observation {
            Some(observer) => {
                observer.reuse_event(ReuseEvent::ForeignGraph, 1);
                analyze_remaining_storage::<S>(expr, grammar, depth, remaining, Some(observer))
            }
            None => analyze_remaining_storage::<S>(expr, grammar, depth, remaining, None),
        };
    }
    if let Err(error) = completed.ready() {
        completed.fail();
        return Err(error);
    }
    let result = analyze_shared_with_completed(
        expr,
        grammar,
        depth,
        remaining,
        None,
        None,
        (Some(&mut *completed), observation),
    );
    if result.is_err() {
        completed.fail();
    }
    result
}

pub(in crate::grammar) fn analyze_remaining_storage<S: Symbols>(
    expr: &Expr,
    grammar: &Grammar,
    depth: usize,
    remaining: &mut usize,
    observation: Option<&PredictionObservation>,
) -> Result<Summary<S>, String> {
    analyze_shared_with_completed::<S>(
        expr,
        grammar,
        depth,
        remaining,
        None,
        None,
        (None, observation),
    )
}

fn analyze_impl(
    expr: &Expr,
    grammar: &Grammar,
    depth: usize,
    mut remaining: usize,
    facts: Option<&mut OccurrenceFacts>,
) -> Result<Summary, String> {
    analyze_shared(expr, grammar, depth, &mut remaining, facts, None)
}

fn analyze_shared<'g>(
    expr: &'g Expr,
    grammar: &'g Grammar,
    depth: usize,
    remaining: &mut usize,
    facts: Option<&mut OccurrenceFacts>,
    capture: Option<&mut prefix_capture::Capture<'g>>,
) -> Result<Summary, String> {
    analyze_shared_with_completed::<BTreeSet<String>>(
        expr,
        grammar,
        depth,
        remaining,
        facts,
        capture,
        (None, None),
    )
}

fn analyze_shared_with_completed<'a, 'g: 'a, S: Symbols>(
    expr: &'a Expr,
    grammar: &'g Grammar,
    depth: usize,
    remaining: &mut usize,
    mut facts: Option<&mut OccurrenceFacts>,
    mut capture: Option<&mut prefix_capture::Capture<'a>>,
    (mut completed, observation): (
        Option<&mut completed_reuse::Storage<'g, S>>,
        Option<&PredictionObservation>,
    ),
) -> Result<Summary<S>, String> {
    let mut frames = vec![Frame::Visit(expr, depth, 0)];
    // This Vec is new for EACH query: a prior query's guarantee is unusable.
    let mut completed_frame_capacity = 0;
    let mut result: Option<Summary<S>> = None;
    while let Some(frame) = frames.pop() {
        match frame {
            Frame::Visit(expr, syntax, calls) => {
                if let Some(observer) = observation {
                    observer.reuse_visit(syntax, calls);
                }
                if syntax > 64 || calls > 64 || *remaining == 0 {
                    return Err("Grammar prediction budget exceeded".into());
                }
                *remaining -= 1;
                if let Some(observer) = observation {
                    observer.reuse_charge(ReuseCharge::CanonicalVisit);
                    observer.reuse_event(ReuseEvent::VisitSucceeded, 1);
                }
                if let Some(completed) = completed.as_deref_mut() {
                    match observation {
                        Some(observer) => {
                            completed.visit_observed(syntax, calls, remaining, observer)?
                        }
                        None => completed.visit(syntax, calls, remaining)?,
                    }
                }
                if let Some(capture) = capture.as_deref_mut() {
                    capture.visit(calls);
                }
                frames.push(Frame::Cardinality(expr.cardinality));
                match &expr.kind {
                    Kind::Action(_) | Kind::Capture { .. } => result = Some((S::default(), true)),
                    Kind::Literal(text) => {
                        result = Some((S::terminal(|| format!("literal:{text}")), false))
                    }
                    Kind::Reference(_) => {
                        result = Some((S::terminal(|| "terminal:ID".into()), false))
                    }
                    Kind::Call(name) => {
                        if let Some(resolved) = resolved_call::lookup(grammar, name) {
                            let rule = resolved.rule();
                            if let Some(facts) = facts.as_deref_mut() {
                                facts.explicit_prefix_hidden_policy |= rule.hidden.is_some();
                            }
                            if let Some(completed) = completed.as_deref_mut() {
                                match match observation {
                                    Some(observer) => completed
                                        .enter_observed(resolved, calls, remaining, observer),
                                    None => completed.enter(resolved, calls, remaining),
                                }? {
                                    completed_reuse::Call::Hit(summary) => {
                                        result = Some(summary);
                                        continue;
                                    }
                                    completed_reuse::Call::Entered => {
                                        match observation {
                                            Some(observer) => completed.return_frame_observed(
                                                &mut frames,
                                                &mut completed_frame_capacity,
                                                remaining,
                                                observer,
                                            )?,
                                            None => completed.return_frame(
                                                &mut frames,
                                                &mut completed_frame_capacity,
                                                remaining,
                                            )?,
                                        }
                                        frames.push(Frame::CompletedCallReturn { rule });
                                    }
                                    completed_reuse::Call::Uncached => {
                                        completed.uncached_frame(
                                            &mut frames,
                                            &mut completed_frame_capacity,
                                            remaining,
                                            observation,
                                        )?;
                                    }
                                    completed_reuse::Call::Fallback => {}
                                }
                            }
                            if capture.is_some() {
                                prefix_capture::retain_source_frame(&mut frames, remaining)?;
                                frames.push(Frame::SourceCallReturn { name, rule });
                            }
                            frames.push(Frame::Visit(&rule.body, 0, calls + 1));
                        } else {
                            result = Some((S::terminal(|| format!("terminal:{name}")), false));
                        }
                    }
                    Kind::ScalarReference { rule, .. } => {
                        let name = rule.as_str();
                        let resolved = resolved_call::lookup(grammar, rule)
                            .ok_or("Unknown datatype reference rule")?;
                        let rule = resolved.rule();
                        if let Some(facts) = facts.as_deref_mut() {
                            facts.explicit_prefix_hidden_policy |= rule.hidden.is_some();
                        }
                        if let Some(completed) = completed.as_deref_mut() {
                            match match observation {
                                Some(observer) => {
                                    completed.enter_observed(resolved, calls, remaining, observer)
                                }
                                None => completed.enter(resolved, calls, remaining),
                            }? {
                                completed_reuse::Call::Hit(summary) => {
                                    result = Some(summary);
                                    continue;
                                }
                                completed_reuse::Call::Entered => {
                                    match observation {
                                        Some(observer) => completed.return_frame_observed(
                                            &mut frames,
                                            &mut completed_frame_capacity,
                                            remaining,
                                            observer,
                                        )?,
                                        None => completed.return_frame(
                                            &mut frames,
                                            &mut completed_frame_capacity,
                                            remaining,
                                        )?,
                                    }
                                    frames.push(Frame::CompletedCallReturn { rule });
                                }
                                completed_reuse::Call::Uncached => {
                                    completed.uncached_frame(
                                        &mut frames,
                                        &mut completed_frame_capacity,
                                        remaining,
                                        observation,
                                    )?;
                                }
                                completed_reuse::Call::Fallback => {}
                            }
                        }
                        if capture.is_some() {
                            prefix_capture::retain_source_frame(&mut frames, remaining)?;
                            frames.push(Frame::SourceCallReturn { name, rule });
                        }
                        frames.push(Frame::Visit(&rule.body, 0, calls + 1));
                    }
                    Kind::Assign { value, .. }
                    | Kind::BooleanAssign { value, .. }
                    | Kind::SyntacticPredicate(value) => {
                        frames.push(Frame::Visit(value, syntax + 1, calls))
                    }
                    Kind::Sequence(items) | Kind::Alternative(items) => {
                        let alternatives = matches!(&expr.kind, Kind::Alternative(_));
                        if let Some(first) = items.first() {
                            frames.push(Frame::Items {
                                items,
                                next: 1,
                                syntax,
                                calls,
                                alternatives,
                                tokens: S::default(),
                                nullable: !alternatives,
                            });
                            frames.push(Frame::Visit(first, syntax + 1, calls));
                        } else {
                            result = Some((S::default(), !alternatives));
                        }
                    }
                }
            }
            Frame::CompletedCallReturn { rule } => {
                let summary = result
                    .as_ref()
                    .ok_or("Missing completed FIRST callee result")?;
                let completed = completed
                    .as_deref_mut()
                    .ok_or("Missing completed FIRST collection")?;
                match observation {
                    Some(observer) => {
                        completed.complete_observed(rule, summary, remaining, observer)?
                    }
                    None => completed.complete(rule, summary, remaining)?,
                }
            }
            Frame::SourceCallReturn { name, rule } => {
                let summary = result
                    .as_ref()
                    .ok_or("Missing captured FIRST callee result")?;
                let capture = capture
                    .as_deref_mut()
                    .ok_or("Missing FIRST capture owner")?;
                capture.complete_call(
                    name,
                    rule,
                    S::full(summary)
                        .ok_or("Nullability projection cannot capture full FIRST symbols")?,
                    remaining,
                )?;
            }
            Frame::Cardinality(cardinality) => {
                let (tokens, nullable) = result.take().ok_or("Missing FIRST result")?;
                if frames.is_empty()
                    && let Some(facts) = facts.as_deref_mut()
                {
                    facts.body_nullable = Some(nullable);
                }
                result = Some((tokens, nullable || matches!(cardinality, '?' | '*')));
            }
            Frame::Items {
                items,
                next,
                syntax,
                calls,
                alternatives,
                mut tokens,
                mut nullable,
            } => {
                let (head, empty) = result.take().ok_or("Missing FIRST child result")?;
                tokens.append(head);
                nullable = if alternatives {
                    nullable || empty
                } else {
                    nullable && empty
                };
                if (alternatives || empty) && next < items.len() {
                    frames.push(Frame::Items {
                        items,
                        next: next + 1,
                        syntax,
                        calls,
                        alternatives,
                        tokens,
                        nullable,
                    });
                    frames.push(Frame::Visit(&items[next], syntax + 1, calls));
                } else {
                    result = Some((tokens, nullable));
                }
            }
        }
    }
    result.ok_or_else(|| "Missing FIRST result".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn occurrence_facts_capture_only_the_root_pre_cardinality_result() {
        let mut grammar = grammar();
        let mut item = grammar.rules["Root"].body.clone();
        item.kind = Kind::Literal("x".into());
        item.cardinality = '*';
        for budget in [0, 1, 2, 64, 10_000] {
            let before = analyze(&item, &grammar, 0, budget);
            let after = analyze_occurrence(&item, &grammar, 0, budget);
            assert_eq!(
                before,
                after
                    .as_ref()
                    .map(|(summary, _)| summary.clone())
                    .map_err(Clone::clone)
            );
            if let Ok((summary, facts)) = after {
                assert!(summary.1);
                assert_eq!(facts.body_nullable, Some(false));
                assert!(!facts.explicit_prefix_hidden_policy);
            }
        }
        let mut root = item.clone();
        root.kind = Kind::Sequence(vec![item]);
        root.cardinality = '1';
        assert_eq!(
            analyze_occurrence(&root, &grammar, 0, 10_000)
                .unwrap()
                .1
                .body_nullable,
            Some(true)
        );
        grammar.rules.get_mut("Root").unwrap().body = root.clone();
        let mut recursive = root.clone();
        recursive.kind = Kind::Call("Root".into());
        grammar.rules.get_mut("Root").unwrap().body = recursive.clone();
        assert_eq!(
            analyze(&recursive, &grammar, 0, 10_000),
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(
            analyze_occurrence(&recursive, &grammar, 0, 10_000).err(),
            Some("Grammar prediction budget exceeded".into())
        );
    }
    #[test]
    fn occurrence_visibility_follows_only_the_existing_first_prefix_traversal() {
        let mut grammar = grammar();
        let mut leaf = grammar.rules["Root"].clone();
        leaf.hidden = Some(vec![]);
        grammar.rules.insert("Hidden".into(), leaf);
        let mut call = grammar.rules["Root"].body.clone();
        call.kind = Kind::Call("Hidden".into());
        assert!(
            analyze_occurrence(&call, &grammar, 0, 10_000)
                .unwrap()
                .1
                .explicit_prefix_hidden_policy
        );
        let mut prefixed = call.clone();
        prefixed.kind = Kind::Sequence(vec![grammar.rules["Root"].body.clone(), call]);
        assert!(
            !analyze_occurrence(&prefixed, &grammar, 0, 10_000)
                .unwrap()
                .1
                .explicit_prefix_hidden_policy
        );
    }
    #[test]
    fn occurrence_summary_keeps_the_same_independent_sixty_four_call_ceiling() {
        for count in [64, 65] {
            let mut grammar = grammar();
            let template = grammar.rules["Root"].clone();
            for index in 0..count {
                let mut rule = template.clone();
                if index + 1 < count {
                    rule.body.kind = Kind::Call(format!("R{}", index + 1));
                }
                grammar.rules.insert(format!("R{index}"), rule);
            }
            let mut root = template.body;
            root.kind = Kind::Call("R0".into());
            root.cardinality = '*';
            let old = analyze(&root, &grammar, 0, 10_000);
            let new = analyze_occurrence(&root, &grammar, 0, 10_000);
            assert_eq!(
                old,
                new.as_ref()
                    .map(|(summary, _)| summary.clone())
                    .map_err(Clone::clone)
            );
            assert_eq!(old.is_ok(), count == 64);
            if let Ok((_, facts)) = new {
                assert_eq!(facts.body_nullable, Some(false));
            }
        }
    }
    fn grammar() -> Grammar {
        let schema=crate::grammar::backend_test_support::inspect_ecore_schema(r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="t" nsURI="urn:t"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#).unwrap();
        super::super::compile_with_profile(
            r#"grammar t hidden() import "urn:t" as t Root returns t::Root: 'end';"#,
            &schema,
            super::super::Profile::NativeV22,
        )
        .unwrap()
    }
    #[test]
    fn matches_recursive_first_for_nullable_and_terminal_combinations() {
        let grammar = grammar();
        let leaf = grammar.rules["Root"].body.clone();
        let mut action = leaf.clone();
        action.kind = Kind::Action("Root".into());
        let mut terminal = leaf.clone();
        terminal.kind = Kind::Call("ID".into());
        for cardinality in ['1', '?', '*', '+'] {
            for kind in [
                Kind::Sequence(vec![action.clone(), terminal.clone(), leaf.clone()]),
                Kind::Alternative(vec![leaf.clone(), terminal.clone(), action.clone()]),
                Kind::Assign {
                    feature: "name".into(),
                    many: false,
                    value: Box::new(terminal.clone()),
                },
            ] {
                let mut expression = leaf.clone();
                expression.kind = kind;
                expression.cardinality = cardinality;
                assert_eq!(
                    analyze(&expression, &grammar, 0, 10000).unwrap(),
                    super::super::first_inner(&expression, &grammar, 0, &mut 10000).unwrap()
                );
            }
        }
    }
    #[test]
    fn preserves_independent_syntax_call_and_work_boundaries() {
        let mut grammar = grammar();
        let leaf = grammar.rules["Root"].body.clone();
        let mut nested = leaf.clone();
        for _ in 0..64 {
            let mut parent = leaf.clone();
            parent.kind = Kind::Sequence(vec![nested]);
            nested = parent;
        }
        assert!(analyze(&nested, &grammar, 0, 10000).is_ok());
        let mut parent = leaf.clone();
        parent.kind = Kind::Sequence(vec![nested]);
        assert!(analyze(&parent, &grammar, 0, 10000).is_err());
        let template = grammar.rules["Root"].clone();
        for length in [64, 65] {
            for index in (0..length).rev() {
                let mut rule = template.clone();
                if index + 1 < length {
                    rule.body.kind = Kind::Call(format!("R{}", index + 1));
                }
                grammar.rules.insert(format!("R{index}"), rule);
            }
            let mut entry = leaf.clone();
            entry.kind = Kind::Call("R0".into());
            assert_eq!(analyze(&entry, &grammar, 0, 10000).is_ok(), length == 64);
        }
        assert!(analyze(&leaf, &grammar, 0, 0).is_err());
        assert!(analyze(&leaf, &grammar, 0, 1).is_ok());
        let mut wide = leaf.clone();
        wide.kind = Kind::Alternative(vec![leaf; 10000]);
        assert!(analyze(&wide, &grammar, 0, 10000).is_err());
    }
}
