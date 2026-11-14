//! Authored/unexecuted. Intended as a child of parser_prediction so every
//! debit below calls its existing analysis_charge, not a replacement ledger.
use super::analysis_charge;
use crate::grammar::admission_observation::pool_blocks::Block;
use crate::grammar::admission_observation::pool_blocks::Observation;
use crate::grammar::admission_observation::pool_blocks::scope;

fn component(work: &mut usize, observer: Option<&Observation>) -> Result<(), String> {
    scope(work, observer, Block::PoolRemainder, |work| {
        scope(work, observer, Block::GraphSourceBinding, |work| {
            analysis_charge(work)?;
            analysis_charge(work)
        })?;
        scope(work, observer, Block::ClassifierRemainder, |work| {
            analysis_charge(work)?;
            scope(work, observer, Block::QueueShareRemainder, |work| {
                for _ in 0..2 {
                    let result = analysis_charge(work);
                    if let Some(observer) = observer {
                        observer.actual_leaf_result(Block::PortKernel, result.is_ok());
                    }
                    result?;
                }
                let result = analysis_charge(work);
                if let Some(observer) = observer {
                    observer.actual_leaf_result(Block::PortDescriptor, result.is_ok());
                }
                result?;
                scope(work, observer, Block::PortSchedule, analysis_charge)
            })
        })
    })
}

#[test]
fn whole_pool_blocks_existing_debit_zero_exact_short_and_preconsumed() -> Result<(), String> {
    for allowance in [0, 4, 6, 7, 13] {
        let mut plain = allowance;
        let mut observed = allowance;
        let expected = component(&mut plain, None);
        let observer = Observation::new(observed);
        let actual = component(&mut observed, Some(&observer));
        observer.end(observed, actual.is_ok());
        let report = observer.snapshot();
        assert_eq!(actual, expected);
        assert_eq!(observed, plain);
        assert!(report.balanced);
        assert_eq!(report.initial, allowance);
        assert_eq!(
            report.counts.units.iter().sum::<u64>(),
            (allowance - plain) as u64
        );
        if allowance == 4 {
            assert_eq!(report.first_actual_denial, Some(Block::PortKernel));
            assert_eq!(
                report.counts.actual_denied_leaf_attempts[Block::PortKernel as usize],
                1
            );
        }
        if allowance == 6 {
            // Last schedule debit really fails, but only the Result boundary
            // was observed. Do not fabricate an actual leaf-denial category.
            assert_eq!(report.first_actual_denial, None);
            assert_eq!(report.first_error_exit, Some(Block::PortSchedule));
        }
        if allowance >= 7 {
            assert_eq!(report.counts.units[Block::GraphSourceBinding as usize], 2);
            assert_eq!(report.counts.units[Block::ClassifierRemainder as usize], 1);
            assert_eq!(report.counts.units[Block::PortKernel as usize], 2);
            assert_eq!(report.counts.units[Block::PortDescriptor as usize], 1);
            assert_eq!(report.counts.units[Block::PortSchedule as usize], 1);
        }
    }
    // A genuinely preconsumed original ledger, not another initial allowance.
    // No work is restored when the observer is attached at the current balance.
    let original_limit = 20;
    let mut plain = original_limit;
    let mut observed = original_limit;
    for _ in 0..7 {
        analysis_charge(&mut plain)?;
        analysis_charge(&mut observed)?;
    }
    assert_eq!(plain, 13);
    assert_eq!(observed, plain);
    let observer = Observation::new(observed);
    let expected = component(&mut plain, None);
    let actual = component(&mut observed, Some(&observer));
    observer.end(observed, actual.is_ok());
    assert_eq!(actual, expected);
    actual?;
    assert_eq!(observed, plain);
    assert_eq!(observed, 6);
    assert_eq!(original_limit - observed, 14);
    assert_eq!(observer.snapshot().initial, 13);
    assert_eq!(observer.snapshot().counts.units.iter().sum::<u64>(), 7);
    assert!(observer.snapshot().balanced);
    Ok(())
}

#[test]
fn whole_pool_blocks_capped_proof_reconciliation_is_counted_once() -> Result<(), String> {
    let mut work = 17;
    let observer = Observation::new(work);
    let result = scope(&mut work, Some(&observer), Block::PoolRemainder, |work| {
        let allowance = (*work).min(3);
        let mut proof = allowance;
        let result = scope(
            &mut proof,
            Some(&observer),
            Block::NullableGraphProof,
            |proof| {
                for _ in 0..4 {
                    analysis_charge(proof)?;
                }
                Ok::<(), String>(())
            },
        );
        *work -= allowance - proof;
        // Mirrors a caught early proof exhaustion without minting/refunding.
        assert!(result.is_err());
        analysis_charge(work)
    });
    observer.end(work, result.is_ok());
    let report = observer.snapshot();
    result?;
    assert_eq!(work, 13);
    assert!(report.balanced);
    assert_eq!(report.counts.units[Block::NullableGraphProof as usize], 3);
    assert_eq!(report.counts.units[Block::PoolRemainder as usize], 1);
    assert_eq!(report.first_actual_denial, None);
    Ok(())
}

#[test]
fn whole_pool_blocks_nonbudget_error_and_foreign_token_keep_authority_separate() {
    let mut work = 9;
    let observer = Observation::new(work);
    let result: Result<(), String> =
        scope(&mut work, Some(&observer), Block::PoolRemainder, |_| {
            Err("original callback cancellation".into())
        });
    observer.end(work, result.is_ok());
    assert_eq!(result, Err("original callback cancellation".into()));
    assert_eq!(work, 9);
    assert!(observer.snapshot().balanced);
    assert_eq!(observer.snapshot().first_actual_denial, None);

    let a = Observation::new(3);
    let b = Observation::new(3);
    let foreign = a.enter(Block::PoolRemainder, 3);
    let own = b.enter(Block::PoolRemainder, 3);
    b.leave(foreign, 3, true);
    b.leave(own, 3, true);
    b.end(3, true);
    assert!(b.snapshot().inconsistent);
    assert!(!b.snapshot().balanced);
}

#[test]
fn whole_pool_blocks_queue_type_totals_overlap_but_do_not_double_count() -> Result<(), String> {
    let mut work = 13;
    let observer = Observation::new(work);
    let token = observer.enter(Block::PoolRemainder, work);
    let before = work;
    scope(
        &mut work,
        Some(&observer),
        Block::QueueShareRemainder,
        |work| {
            analysis_charge(work)?;
            analysis_charge(work)
        },
    )?;
    observer.queue_span(0, before, work);
    let before = work;
    scope(
        &mut work,
        Some(&observer),
        Block::QueueShareRemainder,
        analysis_charge,
    )?;
    observer.queue_span(1, before, work);
    observer.leave(token, work, true);
    observer.end(work, true);
    let report = observer.snapshot();
    assert_eq!(report.counts.units.iter().sum::<u64>(), 3);
    assert_eq!(report.queue_share_inclusive, [2, 1, 0]);
    assert!(report.balanced);
    Ok(())
}

#[test]
fn whole_pool_blocks_post_end_and_inactive_mutators_invalidate_balance_immediately() {
    for mutation in 0..5 {
        let observer = Observation::new(3);
        let root = observer.enter(Block::PoolRemainder, 3);
        observer.leave(root, 3, true);
        observer.end(3, true);
        assert!(observer.snapshot().balanced);
        match mutation {
            0 => observer.queue_span(0, 1, 0),
            1 => {
                assert!(observer.enter(Block::PoolRemainder, 3).is_none());
            }
            2 => observer.actual_leaf_result(Block::PortKernel, false),
            3 => observer.leave(root, 3, true),
            _ => observer.end(3, true),
        }
        let report = observer.snapshot();
        assert!(report.ended);
        assert!(report.inconsistent);
        assert!(!report.balanced);
        assert_eq!(report.counts.units.iter().sum::<u64>(), 0);
        assert_eq!(report.queue_share_inclusive, [0; 3]);
    }
    let inactive = Observation::new(3);
    inactive.queue_span(0, 1, 0);
    assert!(inactive.snapshot().inconsistent);
    assert!(!inactive.snapshot().balanced);
    assert_eq!(inactive.snapshot().queue_share_inclusive, [0; 3]);

    let closed = Observation::new(3);
    let root = closed.enter(Block::PoolRemainder, 3);
    closed.leave(root, 3, true);
    // Depth zero between root leave and end is also not an active scope.
    closed.queue_span(0, 1, 0);
    assert!(closed.snapshot().inconsistent);
    assert!(!closed.snapshot().balanced);
}

#[test]
fn whole_pool_blocks_depth_overflow_stale_tokens_and_root_reuse_are_sticky() {
    let deep = Observation::new(9);
    assert!(deep.enter(Block::PoolRemainder, 9).is_some());
    for _ in 1..12 {
        assert!(deep.enter(Block::ClassifierRemainder, 9).is_some());
    }
    assert!(deep.enter(Block::Witness, 9).is_none());
    assert!(deep.snapshot().inconsistent);
    assert!(!deep.snapshot().balanced);
    let before = deep.snapshot();
    deep.actual_leaf_result(Block::PortKernel, true);
    assert_eq!(deep.snapshot().counts, before.counts);
    assert!(deep.enter(Block::Witness, 9).is_none());

    let stale = Observation::new(9);
    let root = stale.enter(Block::PoolRemainder, 9);
    let old = stale.enter(Block::Witness, 9);
    stale.leave(old, 9, true);
    let fresh = stale.enter(Block::Witness, 9);
    // Same depth and observer, but the copied old serial cannot close fresh.
    stale.leave(old, 9, true);
    assert!(stale.snapshot().inconsistent);
    assert!(!stale.snapshot().balanced);
    stale.leave(fresh, 9, true);
    stale.leave(root, 9, true);
    stale.end(9, true);
    assert!(!stale.snapshot().balanced);

    let one_root = Observation::new(9);
    let root = one_root.enter(Block::PoolRemainder, 9);
    one_root.leave(root, 9, true);
    assert!(one_root.enter(Block::PoolRemainder, 9).is_none());
    assert!(one_root.snapshot().inconsistent);
    assert!(!one_root.snapshot().balanced);
}
