//! Assertions for the private owned-frontier seam. No runtime award at authoring.
use super::super::{Decision, Edge, ParserPrediction};
use super::*;
use std::mem::{align_of, size_of};

fn row(state: usize, alternative: usize) -> AnalysisConfiguration {
    AnalysisConfiguration {
        state,
        alternative,
        calls: vec![(100 + alternative, 200 + alternative)],
    }
}
fn rows(count: usize) -> Rows {
    (0..count).map(|a| row(0, a)).collect()
}
fn token_map(rows: Rows) -> Tokens {
    BTreeMap::from([(PredictionSymbol::Keyword("x".into()), rows)])
}
fn place<'graph>(
    queue: &mut SeededQueue<'graph>,
    keys: &mut SeededKeys<'_>,
    value: AnalysisConfiguration,
    remaining: &mut usize,
) -> Result<(), String> {
    SeededKeys::schedule(
        value,
        queue,
        keys,
        remaining,
        (true, ScheduleSite::Successor, None),
        None,
    )
}
fn drain<'graph, Q: ClosureQueue<'graph>>(queue: &mut Q) -> Vec<AnalysisConfiguration> {
    let mut output = Vec::new();
    while let Some(row) = queue.pop_front() {
        output.push(row);
    }
    output
}

#[test]
fn paid_origin_and_pending_growth_are_precharged_before_publication() -> Result<(), String> {
    for grant in [0, 1] {
        let mut remaining = grant;
        assert!(AdoptFrontiers::initial(rows(2), &mut remaining).is_err());
        assert_eq!(remaining, 0);
    }
    let mut remaining = 9; // Already spent7 of the original16; never reset.
    let mut pending = AdoptFrontiers::initial(rows(2), &mut remaining)?;
    assert_eq!(remaining, 7); // Header1 then actual reserve1.
    let (initial, paid) = AdoptFrontiers::take(pending.pop_front().ok_or("Missing initial")?);
    assert!(!paid);
    let mut zero = 0;
    assert_eq!(
        AdoptFrontiers::uniform_state(paid, &initial, &mut zero)?,
        None
    );
    assert_eq!(zero, 0); // An initial set cannot silently acquire paid origin.
    AdoptFrontiers::extend(&mut pending, token_map(initial), &mut remaining)?;
    assert_eq!(remaining, 6); // Existing paid pending capacity, header only.
    let (complete, paid) = AdoptFrontiers::take(pending.pop_front().ok_or("Missing token")?);
    assert!(paid);
    assert_eq!(
        AdoptFrontiers::uniform_state(paid, &complete, &mut remaining)?,
        Some(0)
    );
    assert_eq!(remaining, 4);
    // Force an ACTUAL full buffer without assuming allocator-chosen capacity.
    let mut fixture_fill = pending.capacity() - pending.len();
    while pending.len() < pending.capacity() {
        AdoptFrontiers::publish(&mut pending, rows(2), Origin::PaidToken, &mut fixture_fill)?;
    }
    assert_eq!(fixture_fill, 0); // Separate full-buffer fixture; no guessed allocator capacity.
    let (len, cap) = (pending.len(), pending.capacity());
    for grant in [0, 1] {
        let mut short = grant;
        assert!(
            AdoptFrontiers::publish(&mut pending, rows(2), Origin::PaidToken, &mut short).is_err()
        );
        assert_eq!((short, pending.len(), pending.capacity()), (0, len, cap));
    }
    let mut exact = 2;
    AdoptFrontiers::publish(&mut pending, rows(2), Origin::PaidToken, &mut exact)?;
    assert_eq!((exact, pending.len()), (0, len + 1));
    assert!(pending.capacity() > cap);
    // An error before completing the token map cannot publish a returned proof.
    let mut zero = 0;
    assert!(AdoptFrontiers::extend(&mut pending, token_map(rows(2)), &mut zero).is_err());
    assert_eq!(pending.len(), len + 1);
    Ok(())
}

#[test]
fn uniform_proof_exact_binding_zero_short_preconsumed_and_recovery() -> Result<(), String> {
    let source = rows(4);
    for grant in [0, 1] {
        let mut remaining = grant;
        assert!(AdoptFrontiers::uniform_state(true, &source, &mut remaining).is_err());
        assert_eq!(remaining, 0);
    }
    let mut exact = 2;
    assert_eq!(
        AdoptFrontiers::uniform_state(true, &source, &mut exact)?,
        Some(0)
    );
    assert_eq!(exact, 0);
    let mixed = BTreeSet::from([row(0, 0), row(1, 1)]);
    let mut exact = 2;
    assert_eq!(
        AdoptFrontiers::uniform_state(true, &mixed, &mut exact)?,
        None
    );
    assert_eq!(exact, 0); // Fallback retains the eligibility debits.
    let mut one = 1;
    assert!(AdoptFrontiers::uniform_state(true, &Rows::new(), &mut one).is_err());
    assert_eq!(one, 0);
    let graph = vec![State::default()];
    let visited = BTreeSet::from([source.clone()]);
    for grant in [0, 1] {
        let mut remaining = grant;
        assert!(bind_adoption(source.clone(), &visited, &graph, 0, &mut remaining).is_err());
        assert_eq!(remaining, 0);
    }
    let mut remaining = 2;
    let (queue, keys) = bind_adoption(source.clone(), &visited, &graph, 0, &mut remaining)?;
    assert_eq!((remaining, queue.seeds.len(), keys.added.len()), (0, 4, 0));
    let mut foreign = source.clone();
    foreign.insert(AnalysisConfiguration {
        calls: vec![(999, 1000)],
        ..row(0, 0)
    });
    let mut remaining = 2;
    assert!(bind_adoption(foreign, &visited, &graph, 0, &mut remaining).is_err());
    assert_eq!(remaining, 1); // Exact full-key lookup fails before adoption fee.
    // One original10k classifier ledger, spent7: only the four new setup units debit it.
    let mut remaining = 10_000 - 7;
    let state = AdoptFrontiers::uniform_state(true, &source, &mut remaining)?.ok_or("No state")?;
    bind_adoption(source, &visited, &graph, state, &mut remaining)?;
    assert_eq!(remaining, 10_000 - 11);
    Ok(())
}

#[test]
fn adopted_seed_fifo_moves_whole_vectors_before_appends_and_keeps_full_keys() -> Result<(), String>
{
    let graph = vec![State::default(); 3];
    let input = rows(32);
    let expected = input.iter().cloned().collect::<Vec<_>>();
    let pointers = input
        .iter()
        .map(|r| r.calls.as_ptr() as usize)
        .collect::<Vec<_>>();
    let visited = BTreeSet::from([input.clone()]);
    let mut remaining = 2;
    let (mut queue, mut keys) = bind_adoption(input, &visited, &graph, 0, &mut remaining)?;
    assert_eq!(queue.appended.observed_shape(), (0, 0));
    assert!(keys.added.is_empty());
    let mut zero = 0;
    place(&mut queue, &mut keys, row(0, 0), &mut zero)?;
    assert_eq!((zero, queue.seeds.len()), (0, 32)); // Seed duplicate: no publication.
    let new = row(2, 99);
    let mut one = 1;
    place(&mut queue, &mut keys, new.clone(), &mut one)?;
    assert_eq!(one, 0);
    let output = drain(&mut queue);
    assert_eq!(&output[..32], expected.as_slice());
    assert_eq!(output[32], new);
    assert_eq!(
        output[..32]
            .iter()
            .map(|r| r.calls.as_ptr() as usize)
            .collect::<Vec<_>>(),
        pointers
    );
    assert_eq!(keys.added.len(), 1);
    Ok(())
}

#[test]
fn genuine_second_lookup_and_new_key_denials_never_publish_unpaid_rows() -> Result<(), String> {
    let graph = vec![State::default(); 2];
    let input = rows(2);
    let visited = BTreeSet::from([input.clone()]);
    let mut bind = 2;
    let (mut queue, mut keys) = bind_adoption(input, &visited, &graph, 0, &mut bind)?;
    let new = AnalysisConfiguration {
        calls: vec![(7, 8)],
        ..row(0, 0)
    };
    for grant in [0, 1] {
        let mut remaining = grant;
        assert!(place(&mut queue, &mut keys, new.clone(), &mut remaining).is_err());
        assert_eq!((remaining, keys.added.len(), queue.seeds.len()), (0, 0, 2));
        assert_eq!(queue.appended.observed_shape(), (0, 0));
    }
    let mut exact = 2; // Seed-set miss's second lookup1 + new key publication1.
    place(&mut queue, &mut keys, new.clone(), &mut exact)?;
    assert_eq!((exact, keys.added.len()), (0, 1));
    let mut zero = 0;
    assert!(place(&mut queue, &mut keys, new.clone(), &mut zero).is_err());
    let mut one = 1;
    place(&mut queue, &mut keys, new, &mut one)?;
    assert_eq!((one, keys.added.len()), (0, 1));
    // Different state cannot be in uniform seeds: one ordinary added lookup.
    let mut one = 1;
    place(&mut queue, &mut keys, row(1, 5), &mut one)?;
    assert_eq!(one, 0); // Different batch is inline, no overflow allocation.
    let mut zero = 0;
    place(&mut queue, &mut keys, row(1, 5), &mut zero)?;
    assert_eq!(zero, 0);
    Ok(())
}

#[test]
fn seeded_shape_reuses_one_worker_and_exact_siblings_with_zero_short_recovery() -> Result<(), String>
{
    for grant in [0, 3, 4] {
        let graph = vec![
            State {
                edges: vec![Edge::Epsilon(1)],
                ..State::default()
            },
            State::default(),
        ];
        let input = rows(2);
        let visited = BTreeSet::from([input.clone()]);
        let mut bind = 2;
        let (mut queue, mut keys) = bind_adoption(input, &visited, &graph, 0, &mut bind)?;
        let first = queue.pop_front().ok_or("Missing seed")?;
        let mut remaining = grant;
        let result = SeededKeys::share(first, &mut queue, &mut keys, &mut remaining, None, None);
        assert_eq!(remaining, 0);
        if grant == 4 {
            assert!(result?.is_none()); // Shape1 + two keys2 + actual overflow1.
            assert_eq!(keys.added.len(), 2);
            assert_eq!(drain(&mut queue), vec![row(1, 0), row(1, 1)]);
        } else {
            assert!(result.is_err());
            assert_eq!(keys.added.len(), usize::from(grant == 3));
        }
    }
    let graph = vec![State {
        edges: vec![Edge::Epsilon(0)],
        ..State::default()
    }];
    let input = rows(4);
    let visited = BTreeSet::from([input.clone()]);
    let mut bind = 2;
    let (mut queue, mut keys) = bind_adoption(input, &visited, &graph, 0, &mut bind)?;
    let first = queue.pop_front().ok_or("Missing seed")?;
    let mut one = 1;
    assert!(SeededKeys::share(first, &mut queue, &mut keys, &mut one, None, None)?.is_none());
    assert_eq!((one, keys.added.len()), (0, 0)); // Four genuine full-key seed duplicates.
    assert!(queue.pop_front().is_none());
    Ok(())
}

#[test]
fn seeded_token_fallback_does_not_reinspect_or_consume_new_appended_siblings() -> Result<(), String>
{
    let graph = vec![
        State {
            edges: vec![Edge::Token(PredictionSymbol::Keyword("x".into()), 1)],
            ..State::default()
        },
        State::default(),
    ];
    let input = rows(4);
    let visited = BTreeSet::from([input.clone()]);
    let mut bind = 2;
    let (mut queue, mut keys) = bind_adoption(input, &visited, &graph, 0, &mut bind)?;
    let mut output = Vec::new();
    let mut one = 1;
    while let Some(row) = queue.pop_front() {
        output.push(
            SeededKeys::share(row, &mut queue, &mut keys, &mut one, None, None)?
                .ok_or("Unexpected transparent Token")?,
        );
    }
    assert_eq!((one, keys.added.len()), (0, 0));
    assert_eq!(output, rows(4).into_iter().collect::<Vec<_>>());
    Ok(())
}

type AnalysisResult = (Result<bool, String>, usize, BTreeMap<(usize, usize), bool>);
fn analyze<F: FrontierPolicy>(
    p: &ParserPrediction,
    entries: &[usize],
    grant: usize,
    shared: bool,
) -> AnalysisResult {
    let mut remaining = grant;
    let mut witnesses = BTreeMap::new();
    let result = p
        .multiple_recursive_alternatives_with_frontiers::<GeometricAdjacentQueue<'_>, F, false>(
            &Decision {
                entries: entries.to_vec(),
                exit: None,
            },
            &mut remaining,
            shared,
            &mut witnesses,
            true,
            (true, None, None, None),
        );
    (result, remaining, witnesses)
}
fn program(states: Vec<State>) -> ParserPrediction {
    ParserPrediction {
        call_depth: crate::grammar::DEFAULT_CALL_DEPTH as u16,
        allow_empty_choices: false,
        states,
        rules: BTreeMap::new(),
        decisions: BTreeMap::new(),
        first_guards: BTreeMap::new(),
        recursive_decisions: BTreeSet::new(),
        runtime_requirement: None,
    }
}

#[test]
fn adopted_shared_classifier_returns_follows_cycles_and_witness_priority_match_legacy()
-> Result<(), String> {
    let token = |target| State {
        edges: vec![Edge::Token(PredictionSymbol::Keyword("x".into()), target)],
        ..State::default()
    };
    let programs = [
        program(vec![
            token(2),
            token(2),
            State {
                edges: vec![Edge::Epsilon(2)],
                ..State::default()
            },
        ]),
        program(vec![
            token(2),
            token(2),
            State {
                edges: vec![Edge::Call {
                    target: 3,
                    follow: 4,
                }],
                ..State::default()
            },
            State {
                rule_stop: true,
                ..State::default()
            },
            token(5),
            State::default(),
        ]),
        program(vec![
            token(2),
            token(2),
            State {
                rule_stop: true,
                follow: vec![3, 4],
                ..State::default()
            },
            token(5),
            token(5),
            State::default(),
        ]),
    ];
    for p in programs {
        let old = analyze::<LegacyFrontiers>(&p, &[0, 1], 1000, true);
        let new = analyze::<AdoptFrontiers>(&p, &[0, 1], 1000, true);
        assert_eq!(old.0, new.0);
        assert_eq!(old.2, new.2);
        assert_eq!(new.0, Ok(false));
        let exact = 1000 - new.1;
        assert!(
            analyze::<AdoptFrontiers>(&p, &[0, 1], exact - 1, true)
                .0
                .is_err()
        );
        assert_eq!(analyze::<AdoptFrontiers>(&p, &[0, 1], exact, true).0, new.0);
        assert_eq!(
            analyze::<AdoptFrontiers>(&p, &[0, 1], 0, true).0,
            Err("Grammar prediction budget exceeded".into())
        );
        assert_eq!(analyze::<AdoptFrontiers>(&p, &[0, 1], exact, true).0, new.0);
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
    let old = analyze::<LegacyFrontiers>(&p, &[0, 1, 999], 1000, true);
    let new = analyze::<AdoptFrontiers>(&p, &[0, 1, 999], 1000, true);
    assert_eq!((new.0, new.2), (old.0, old.2)); // Witness wins before later dangling state.
    let p = program(vec![State::default()]);
    assert_eq!(
        analyze::<AdoptFrontiers>(&p, &[0, 999], 100, false).0,
        Err("Dangling parser prediction state".into())
    );
    Ok(())
}

#[test]
fn adoption_components_remove_seed_publications_and_report_real_private_headers()
-> Result<(), String> {
    let graph = vec![State::default()];
    let input = rows(32);
    let visited = BTreeSet::from([input.clone()]);
    let mut old = GeometricAdjacentQueue::empty(&graph);
    let mut old_keys = Rows::new();
    let mut remaining = 1000;
    for value in input.clone() {
        Rows::schedule(
            value,
            &mut old,
            &mut old_keys,
            &mut remaining,
            (true, ScheduleSite::Seed, None),
            None,
        )?;
    }
    let old_cost = 1000 - remaining;
    let mut remaining = 1000;
    let state = AdoptFrontiers::uniform_state(true, &input, &mut remaining)?
        .ok_or("Missing uniform state")?;
    let (mut adopted, keys) = bind_adoption(input, &visited, &graph, state, &mut remaining)?;
    assert_eq!(1000 - remaining, 4); // Eligibility2 + exact bind1 + adoption1.
    assert!(keys.added.is_empty());
    assert_eq!(adopted.appended.observed_shape(), (0, 0));
    assert_eq!(drain(&mut old), drain(&mut adopted));
    assert!(old_cost > 32); // Actual reserve calls remain charged, never guessed.
    assert_eq!(
        size_of::<<LegacyFrontiers as FrontierPolicy>::Pending>(),
        size_of::<Rows>()
    );
    assert_eq!(
        align_of::<<LegacyFrontiers as FrontierPolicy>::Pending>(),
        align_of::<Rows>()
    );
    assert_eq!(
        size_of::<super::super::adjacent::AdjacentQueue<'_>>(),
        size_of::<GeometricAdjacentQueue<'_>>()
    );
    println!(
        "FRONTIER_ADOPTION_COMPONENT {{\"contexts\":32,\"old_seed_fee\":{old_cost},\"eligibility_bind_adopt_fee\":4,\"new_seed_publications\":0,\"new_seed_queue_reserves\":0,\"visited_clone_still_unmetered\":true,\"pending_publication_and_second_lookup_fees_excluded_from_this_component\":true}}"
    );
    println!(
        "FRONTIER_ADOPTION_LAYOUT {{\"legacy_pending_bytes\":{},\"tagged_pending_bytes\":{},\"seeded_queue_bytes\":{},\"borrowed_keys_bytes\":{},\"into_iter_bytes\":{},\"old_queue_bytes\":{},\"allocator_copy_bytes_measured\":false}}",
        size_of::<Rows>(),
        size_of::<PendingFrontier>(),
        size_of::<SeededQueue<'_>>(),
        size_of::<SeededKeys<'_>>(),
        size_of::<btree_set::IntoIter<AnalysisConfiguration>>(),
        size_of::<GeometricAdjacentQueue<'_>>()
    );
    Ok(())
}
