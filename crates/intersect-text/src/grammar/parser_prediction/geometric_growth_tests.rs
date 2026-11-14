//! Source-authored component controls. No platform/capacity result is awarded
//! until these assertions execute on that target.
use super::*;
use std::mem::{align_of, size_of};

#[derive(Debug)]
struct ReserveEvent {
    len: usize,
    before: usize,
    requested: usize,
    after: usize,
}

fn row(state: usize, alternative: usize) -> AnalysisConfiguration {
    AnalysisConfiguration {
        state,
        alternative,
        calls: vec![(100 + alternative, 200 + alternative)],
    }
}

fn place<const GEOMETRIC: bool>(
    queue: &mut GrowthQueue<'_, GEOMETRIC>,
    keys: &mut BTreeSet<AnalysisConfiguration>,
    remaining: &mut usize,
    value: AnalysisConfiguration,
) -> Result<(), String> {
    GrowthQueue::schedule(
        value,
        queue,
        keys,
        remaining,
        (true, ScheduleSite::Seed, None),
    )
}

fn drain<const GEOMETRIC: bool>(
    queue: &mut GrowthQueue<'_, GEOMETRIC>,
) -> Vec<AnalysisConfiguration> {
    let mut rows = Vec::new();
    while let Some(row) = queue.pop_front() {
        rows.push(row);
    }
    rows
}

fn reserve_probe<const GEOMETRIC: bool>(count: usize) -> Result<Vec<ReserveEvent>, String> {
    let graph = vec![State::default()];
    let mut queue = GrowthQueue::<GEOMETRIC>::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut remaining = 1000;
    let mut events = Vec::new();
    let expected: Vec<_> = (0..count).map(|a| row(0, a)).collect();
    let addresses: Vec<_> = expected.iter().map(|r| r.calls.as_ptr() as usize).collect();
    for value in expected {
        let old = queue
            .batches
            .back()
            .map(|batch| (batch.overflow.len(), batch.overflow.capacity()));
        let initial = remaining;
        place(&mut queue, &mut keys, &mut remaining, value)?;
        if let Some((len, before)) = old
            && len == before
        {
            let requested = if GEOMETRIC { before.max(4) } else { 4 };
            let after = queue
                .batches
                .back()
                .ok_or("Missing paid batch")?
                .overflow
                .capacity();
            assert!(after >= len + requested);
            assert_eq!(initial - remaining, 2); // Same key1 and actual reserve1.
            events.push(ReserveEvent {
                len,
                before,
                requested,
                after,
            });
        } else {
            assert_eq!(initial - remaining, 1);
        }
    }
    assert_eq!(1000 - remaining, count + events.len());
    assert_eq!(keys.len(), count);
    let output = drain(&mut queue);
    assert_eq!(output, (0..count).map(|a| row(0, a)).collect::<Vec<_>>());
    assert_eq!(
        output
            .iter()
            .map(|r| r.calls.as_ptr() as usize)
            .collect::<Vec<_>>(),
        addresses
    );
    Ok(events)
}

#[test]
fn geometric_actual_reserves_capacity_and_memory_bounds_keep_old_fees() -> Result<(), String> {
    let fixed = reserve_probe::<false>(32)?;
    let geometric = reserve_probe::<true>(32)?;
    // These are explicit target assertions, not allocator promises. A platform
    // with a different observed capacity must qualify separately.
    assert_eq!(
        fixed.iter().map(|e| e.requested).collect::<Vec<_>>(),
        vec![4; 8]
    );
    assert_eq!(
        fixed.iter().map(|e| e.after).collect::<Vec<_>>(),
        vec![4, 8, 12, 16, 20, 24, 28, 32]
    );
    assert_eq!(
        geometric.iter().map(|e| e.requested).collect::<Vec<_>>(),
        vec![4, 4, 8, 16]
    );
    assert_eq!(
        geometric.iter().map(|e| e.after).collect::<Vec<_>>(),
        vec![4, 8, 16, 32]
    );
    assert!(fixed.iter().chain(&geometric).all(|e| e.len == e.before));
    let fixed_headers: usize = fixed.iter().map(|e| e.len).sum();
    let geometric_headers: usize = geometric.iter().map(|e| e.len).sum();
    assert_eq!((fixed_headers, geometric_headers), (112, 28));
    let fixed_nonpower = reserve_probe::<false>(11)?;
    let geometric_nonpower = reserve_probe::<true>(11)?;
    assert_eq!(fixed_nonpower.last().ok_or("No fixed capacity")?.after, 12);
    assert_eq!(
        geometric_nonpower
            .last()
            .ok_or("No geometric capacity")?
            .after,
        16
    );
    println!(
        "GEOMETRIC_OVERFLOW_COMPONENT {{\"contexts\":32,\"fixed_reserves\":8,\"geometric_reserves\":4,\"fixed_fee\":40,\"geometric_fee\":36,\"context_header_bytes\":{},\"fixed_occupied_headers_at_reserve\":112,\"geometric_occupied_headers_at_reserve\":28,\"max_capacity\":32,\"nonpower_additional_spare_slots\":4,\"allocator_copy_bytes_measured\":false}}",
        size_of::<Context>()
    );
    Ok(())
}

#[test]
fn geometric_zero_exact_short_preconsumed_duplicate_and_recovery() -> Result<(), String> {
    for (grant, expected_keys, complete) in [
        (0, 0, false),
        (35, 31, false),
        (36, 32, true),
        (36, 32, true),
    ] {
        let graph = vec![State::default()];
        let mut queue = GeometricAdjacentQueue::empty(&graph);
        let mut keys = BTreeSet::new();
        let mut remaining = grant;
        let result = (|| {
            for a in 0..32 {
                place(&mut queue, &mut keys, &mut remaining, row(0, a))?;
            }
            Ok::<_, String>(())
        })();
        assert_eq!(result.is_ok(), complete);
        if !complete {
            assert_eq!(result, Err("Grammar prediction budget exceeded".into()));
        }
        assert_eq!(remaining, 0);
        assert_eq!(keys.len(), expected_keys);
        if complete {
            place(&mut queue, &mut keys, &mut remaining, row(0, 31))?;
            assert_eq!(remaining, 0); // Original duplicate-at-zero behavior.
        }
        assert_eq!(drain(&mut queue).len(), expected_keys);
    }
    let graph = vec![State::default()];
    let mut queue = GeometricAdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let original = 43;
    let already_spent = 7;
    let mut remaining = original - already_spent;
    for a in 0..32 {
        place(&mut queue, &mut keys, &mut remaining, row(0, a))?;
    }
    assert_eq!(original - remaining, 43);
    assert_eq!(remaining, 0);
    Ok(())
}

#[test]
fn geometric_denied_reserve_and_capacity_error_precede_publication() -> Result<(), String> {
    let graph = vec![State::default()];
    let mut queue = GeometricAdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut remaining = 2;
    place(&mut queue, &mut keys, &mut remaining, row(0, 0))?;
    assert_eq!(
        place(&mut queue, &mut keys, &mut remaining, row(0, 1)),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(remaining, 0); // The denied reserve retains the prior key debit.
    assert_eq!(keys, BTreeSet::from([row(0, 0)]));
    assert_eq!(
        queue
            .batches
            .back()
            .ok_or("Missing batch")?
            .overflow
            .capacity(),
        0
    );
    assert_eq!(drain(&mut queue), vec![row(0, 0)]);
    // Deterministic capacity-overflow rejection in the SAME charged reserve
    // helper; this does not pretend to exhaust the operating-system allocator.
    let mut overflow: VecDeque<Context> = VecDeque::new();
    let mut fee = 1;
    assert_eq!(
        reserve_overflow(&mut overflow, usize::MAX, &mut fee, None),
        Err("Adjacent closure overflow allocation unavailable".into())
    );
    assert_eq!((fee, overflow.len(), overflow.capacity()), (0, 0, 0));
    let mut zero = 0;
    assert_eq!(
        reserve_overflow(&mut overflow, 4, &mut zero, None),
        Err("Grammar prediction budget exceeded".into())
    );
    assert_eq!(overflow.capacity(), 0);
    Ok(())
}

#[test]
fn geometric_fifo_full_keys_transparent_cycles_fallback_and_sibling_scope() -> Result<(), String> {
    let graph = vec![State {
        edges: vec![Edge::Epsilon(0)],
        ..State::default()
    }];
    let mut queue = GeometricAdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut remaining = 37;
    for a in 0..32 {
        place(&mut queue, &mut keys, &mut remaining, row(0, a))?;
    }
    let first = queue.pop_front().ok_or("Missing original context")?;
    assert!(
        queue
            .share_transparent(first, &mut keys, &mut remaining, None)?
            .is_none()
    );
    assert_eq!((remaining, keys.len()), (0, 32));
    assert!(queue.pop_front().is_none());
    let graph = vec![State {
        edges: vec![Edge::Token(
            super::super::PredictionSymbol::Keyword("x".into()),
            0,
        )],
        ..State::default()
    }];
    let mut queue = GeometricAdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut remaining = 37;
    for a in 0..32 {
        place(&mut queue, &mut keys, &mut remaining, row(0, a))?;
    }
    let mut output = Vec::new();
    while let Some(next) = queue.pop_front() {
        output.push(
            queue
                .share_transparent(next, &mut keys, &mut remaining, None)?
                .ok_or("Unexpected transparent token")?,
        );
    }
    assert_eq!(remaining, 0); // One paid fallback shape, no later reinspection.
    assert_eq!(output, (0..32).map(|a| row(0, a)).collect::<Vec<_>>());
    let graph = vec![State::default(); 2];
    let mut queue = GeometricAdjacentQueue::empty(&graph);
    let mut keys = BTreeSet::new();
    let mut remaining = 10;
    let rows = vec![
        row(0, 0),
        row(1, 1),
        row(0, 2),
        AnalysisConfiguration {
            calls: vec![(9, 9)],
            ..row(0, 2)
        },
    ];
    for value in rows.clone() {
        place(&mut queue, &mut keys, &mut remaining, value)?;
    }
    assert_eq!(drain(&mut queue), rows);
    assert_eq!(keys.len(), 4); // State, alternative and entire call stack matter.
    Ok(())
}

#[test]
fn geometric_shared_classifier_witness_depth_errors_and_holder_layout() -> Result<(), String> {
    use super::super::{Decision, ParserPrediction};
    type AnalysisResult = (
        Result<bool, String>,
        usize,
        std::collections::BTreeMap<(usize, usize), bool>,
    );
    fn analyze<const GEOMETRIC: bool>(
        p: &ParserPrediction,
        entries: &[usize],
        grant: usize,
        shared: bool,
    ) -> AnalysisResult {
        let mut remaining = grant;
        let mut witnesses = std::collections::BTreeMap::new();
        let result = p.multiple_recursive_alternatives_with_queue::<GrowthQueue<'_, GEOMETRIC>>(
            &Decision {
                entries: entries.to_vec(),
                exit: None,
            },
            &mut remaining,
            shared,
            &mut witnesses,
            true,
            (true, None, None),
        );
        (result, remaining, witnesses)
    }
    fn program(states: Vec<State>) -> ParserPrediction {
        ParserPrediction {
            call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
            allow_empty_choices: false,
            states,
            rules: Default::default(),
            decisions: Default::default(),
            first_guards: Default::default(),
            recursive_decisions: Default::default(),
            runtime_requirement: None,
        }
    }
    let p = program(vec![
        State {
            edges: vec![Edge::Call {
                target: 2,
                follow: 3,
            }],
            ..State::default()
        },
        State {
            edges: vec![Edge::Call {
                target: 2,
                follow: 3,
            }],
            ..State::default()
        },
        State {
            edges: vec![Edge::Call {
                target: 2,
                follow: 3,
            }],
            ..State::default()
        },
        State::default(),
    ]);
    let fixed = analyze::<false>(&p, &[0, 1, 999], 100, true);
    let geometric = analyze::<true>(&p, &[0, 1, 999], 100, true);
    assert_eq!(fixed, geometric);
    assert_eq!(geometric.0, Ok(true));
    assert_eq!(
        geometric.2,
        std::collections::BTreeMap::from([((2, 128), true)])
    );
    let exact = 100 - geometric.1;
    for grant in [0, exact - 1, exact, exact] {
        assert_eq!(
            analyze::<false>(&p, &[0, 1, 999], grant, true),
            analyze::<true>(&p, &[0, 1, 999], grant, true)
        );
    }
    for count in [128, 129] {
        let mut states = vec![State::default(); count + 3];
        for (i, state) in states.iter_mut().enumerate().take(count) {
            state.edges.push(Edge::Call {
                target: i + 1,
                follow: count + 1,
            });
        }
        states[count].edges.push(Edge::Token(
            super::super::PredictionSymbol::Keyword("x".into()),
            count + 2,
        ));
        let p = program(states);
        let a = analyze::<false>(&p, &[0, 0], 10000, false);
        let b = analyze::<true>(&p, &[0, 0], 10000, false);
        assert_eq!(a, b);
        if count == 129 {
            assert_eq!(b.0, Err("Parser prediction call depth exceeded".into()));
        }
    }
    assert!(
        GeometricAdjacentQueue::unreserved(BTreeSet::<AnalysisConfiguration>::new(), &[]).is_err()
    );
    assert_eq!(
        size_of::<AdjacentQueue<'_>>(),
        size_of::<GeometricAdjacentQueue<'_>>()
    );
    assert_eq!(
        align_of::<AdjacentQueue<'_>>(),
        align_of::<GeometricAdjacentQueue<'_>>()
    );
    println!(
        "GEOMETRIC_LAYOUT {{\"queue_bytes\":{},\"batch_bytes\":{},\"context_bytes\":{},\"policy_state_bytes\":0}}",
        size_of::<AdjacentQueue<'_>>(),
        size_of::<Batch>(),
        size_of::<Context>()
    );
    Ok(())
}
