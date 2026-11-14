//! Borrowed row handles for one invocation's existing paid arena.
//!
//! The bank is stack-only until the first canonical seed operation. It retains
//! one ordinary Rc-Arena root; rows borrow its immutable identity instead of
//! retaining/releasing an Rc on each configuration copy. The backing append,
//! full key and materialization algorithms are the SAME Arena methods.
use super::{Arena, ArenaReturns, FIRST_NODE, Nodes, Returns, State, WORK_LIMIT, charge};
use std::{borrow::Cow, cell::OnceCell, cmp::Ordering, fmt};

pub(in crate::grammar::parser_prediction) struct ArenaBank {
    root: OnceCell<ArenaReturns>,
}
impl ArenaBank {
    pub(in crate::grammar::parser_prediction) const fn new() -> Self {
        Self {
            root: OnceCell::new(),
        }
    }
    fn seeded(
        &self,
        value: &[usize],
        work: &mut Option<usize>,
    ) -> Result<(&Arena, usize, usize), String> {
        // A real bank lookup replaces the old per-initial-row Rc retain. Cold
        // initialization deliberately retains ALL old seed/allocation charges.
        charge(work)?;
        let root = if let Some(root) = self.root.get() {
            root
        } else {
            let root = ArenaReturns::seed(value, &mut None, work)?;
            charge(work)?; // actual complete root publication, before mutation
            self.root
                .set(root)
                .map_err(|_| "Invocation arena seed already installed")?;
            charge(work)?; // actual cold lookup after installing the complete root
            self.root
                .get()
                .ok_or("Missing installed invocation arena seed")?
        };
        let State::Bound { arena, head, len } = &root.0 else {
            return Err("Missing paid invocation arena root".into());
        };
        // Hot seeding returns exactly the bank's original paid root, just as
        // ArenaReturns::seed returns its cached root. No supplied hot value or
        // its length can mint another stack. The sole worker always passes the
        // same immutable LiveSeed returns for this invocation.
        Ok((arena, *head, *len))
    }
}

// Logical portable stack descriptor bound. The retained root's heap/header and
// seed charges remain the old Arena policy; this is not aggregate worker/RSS.
const _: () = {
    assert!(std::mem::size_of::<ArenaBank>() <= 64);
    assert!(std::mem::size_of::<BorrowedArenaReturns<'static>>() <= 64);
    assert!(
        std::mem::size_of::<
            crate::grammar::parser_prediction::PredictionConfiguration<
                BorrowedArenaReturns<'static>,
            >,
        >() <= 64
    );
};

#[derive(Clone, Copy)]
pub(in crate::grammar::parser_prediction) struct BorrowedArenaReturns<'arena>(Handle<'arena>);
#[derive(Clone, Copy)]
enum Handle<'arena> {
    Empty,
    Seed(&'arena ArenaBank),
    Bound {
        arena: &'arena Arena,
        head: usize,
        len: usize,
    },
}
impl<'arena> BorrowedArenaReturns<'arena> {
    pub(in crate::grammar::parser_prediction) fn unseeded(bank: &'arena ArenaBank) -> Self {
        Self(Handle::Seed(bank))
    }
    fn bound(&self) -> Result<(&'arena Arena, usize, usize), String> {
        let Handle::Bound { arena, head, len } = self.0 else {
            return Err("Missing paid borrowed arena return seed".into());
        };
        if !ArenaReturns::valid(arena, head, len) {
            return Err("Invalid private arena return head".into());
        }
        Ok((arena, head, len))
    }
}
impl fmt::Debug for BorrowedArenaReturns<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Handle::Empty => f.write_str("EmptyBorrowedArenaReturns"),
            Handle::Seed(_) => f.write_str("UnseededBorrowedArenaReturns"),
            Handle::Bound { head, len, .. } => f
                .debug_struct("BorrowedArenaReturns")
                .field("head", head)
                .field("len", len)
                .finish(),
        }
    }
}
impl PartialEq for BorrowedArenaReturns<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for BorrowedArenaReturns<'_> {}
impl PartialOrd for BorrowedArenaReturns<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BorrowedArenaReturns<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Exactly the old Arena full-value lex comparison, including unrelated
        // banks with equal values. Private arena/node identities never order keys.
        // Existing infallible comparison CPU remains unmetered, without a new
        // allowance, allocation, callback, trait object or fabricated fallback.
        let mut a = [0; crate::grammar::MAX_CALL_DEPTH];
        let mut b = [0; crate::grammar::MAX_CALL_DEPTH];
        match (self.0, other.0) {
            (
                Handle::Bound {
                    arena: left,
                    head: ah,
                    len: al,
                },
                Handle::Bound {
                    arena: right,
                    head: bh,
                    len: bl,
                },
            ) => {
                if std::ptr::eq(left, right) {
                    if ah == bh && al == bl {
                        return Ordering::Equal;
                    }
                    let nodes = Nodes::take(&left.nodes);
                    let an = left.key(ah, al, &nodes.data, &mut a);
                    let bn = right.key(bh, bl, &nodes.data, &mut b);
                    a[..an].cmp(&b[..bn])
                } else {
                    let left_nodes = Nodes::take(&left.nodes);
                    let right_nodes = Nodes::take(&right.nodes);
                    let an = left.key(ah, al, &left_nodes.data, &mut a);
                    let bn = right.key(bh, bl, &right_nodes.data, &mut b);
                    a[..an].cmp(&b[..bn])
                }
            }
            // Empty/Seed carry no return values and cannot enter the worker's
            // pending/visited/frontier until seed() produces a valid Bound row.
            _ => self.len().cmp(&other.len()),
        }
    }
}
impl Returns for BorrowedArenaReturns<'_> {
    type Visited = std::collections::BTreeSet<
        crate::grammar::parser_prediction::PredictionConfiguration<Self>,
    >;
    const SHARED: bool = true;
    const MOVE_SINGLE_EDGE: bool = true;
    const MOVE_LAST_EDGE: bool = true;
    fn empty() -> Self {
        Self(Handle::Empty)
    }
    fn from_owned(_: Vec<usize>) -> Result<Self, String> {
        Err("Borrowed arena returns require the exact invocation bank".into())
    }
    fn seed(
        value: &[usize],
        cached: &mut Option<Self>,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        if work.as_ref().is_none_or(|n| *n > WORK_LIMIT) {
            return Err(
                "Arena returns require the original finite at-most4000000000 work pool".into(),
            );
        }
        let Some(Self(Handle::Seed(bank))) = *cached else {
            return Err("Missing exact invocation arena bank".into());
        };
        let (arena, head, len) = bank.seeded(value, work)?;
        Ok(Self(Handle::Bound { arena, head, len }))
    }
    fn len(&self) -> usize {
        match self.0 {
            Handle::Bound { len, .. } => len,
            Handle::Empty | Handle::Seed(_) => 0,
        }
    }
    fn copy_fee(&self, _: &mut Option<usize>) -> Result<(), String> {
        self.bound()?;
        // A Copy handle performs no Rc retain/release, allocation, prefix copy
        // or node walk. Existing configuration payload/queue/visited charges
        // remain in the SAME worker; no performed operation is discounted.
        Ok(())
    }
    fn popped(&self, work: &mut Option<usize>, _: bool) -> Result<Option<(usize, Self)>, String> {
        let (arena, head, len) = self.bound()?;
        if len == 0 {
            return Ok(None);
        }
        charge(work)?; // same actual pop/selection, without an Rc retain
        let (value, parent) = if head < FIRST_NODE {
            (arena.seed[head - 1], head - 1)
        } else {
            let nodes = Nodes::take(&arena.nodes);
            let node = nodes.data[head - FIRST_NODE];
            (node.value, node.parent)
        };
        Ok(Some((
            value,
            Self(Handle::Bound {
                arena,
                head: parent,
                len: len - 1,
            }),
        )))
    }
    fn push_return(&mut self, value: usize, work: &mut Option<usize>) -> Result<(), String> {
        let Handle::Bound { arena, head, len } = &mut self.0 else {
            return Err("Missing paid borrowed arena return seed".into());
        };
        arena.append(head, len, value, work)
    }
    fn materialized(&self, work: &mut Option<usize>) -> Result<Cow<'_, [usize]>, String> {
        let (arena, head, len) = self.bound()?;
        arena.materialized(head, len, work)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeSet, rc::Rc};
    fn seed<'a>(
        bank: &'a ArenaBank,
        values: &[usize],
        work: &mut Option<usize>,
    ) -> BorrowedArenaReturns<'a> {
        BorrowedArenaReturns::seed(
            values,
            &mut Some(BorrowedArenaReturns::unseeded(bank)),
            work,
        )
        .unwrap()
    }
    fn root(bank: &ArenaBank) -> &ArenaReturns {
        bank.root.get().unwrap()
    }
    fn strong(bank: &ArenaBank) -> usize {
        let State::Bound { arena, .. } = &root(bank).0 else {
            panic!("paid root")
        };
        Rc::strong_count(arena)
    }
    #[allow(clippy::mutable_key_type)]
    #[test]
    fn shared_returns_borrowed_arena_real_growth_keys_pop_and_no_row_retains() {
        let bank = ArenaBank::new();
        let other_bank = ArenaBank::new();
        assert!(
            bank.root.get().is_none(),
            "stack-only construction does not seed"
        );
        let mut work = Some(100000);
        let base = seed(&bank, &[usize::MAX, 1], &mut work);
        let equal = seed(&other_bank, &[usize::MAX, 1], &mut work);
        assert_eq!(base, equal);
        assert_eq!(strong(&bank), 1);
        let hot = seed(&bank, &[9; 128], &mut work);
        assert_eq!(
            hot, base,
            "a hot argument cannot replace the bank's original paid stack"
        );
        assert_eq!(
            hot.materialized(&mut work).unwrap().as_ref(),
            &[usize::MAX, 1]
        );
        let mut old = ArenaReturns::seed(&[usize::MAX, 1], &mut None, &mut work).unwrap();
        let mut row = base;
        let mut kept = BTreeSet::new();
        for n in 0..257 {
            let before = work;
            row.copy_fee(&mut work).unwrap();
            assert_eq!(work, before, "Copy handle has no Rc retain to debit");
            let mut next = base;
            old.copy_fee(&mut work).unwrap();
            let mut reference = old.clone();
            let prior = work;
            next.push_return(n % 3, &mut work).unwrap();
            let actual_push = prior.unwrap() - work.unwrap();
            let prior = work;
            reference.push_return(n % 3, &mut work).unwrap();
            assert_eq!(
                prior.unwrap() - work.unwrap(),
                actual_push,
                "SAME actual append/growth tariff"
            );
            let prior = work;
            let actual = next.materialized(&mut work).unwrap().into_owned();
            let actual_material = prior.unwrap() - work.unwrap();
            let prior = work;
            assert_eq!(
                reference.materialized(&mut work).unwrap().as_ref(),
                actual.as_slice()
            );
            assert_eq!(prior.unwrap() - work.unwrap(), actual_material);
            assert_eq!(actual, vec![usize::MAX, 1, n % 3]);
            kept.insert(next);
            let prior = work;
            let (value, rest) = next.popped(&mut work, true).unwrap().unwrap();
            assert_eq!(prior.unwrap() - work.unwrap(), 1);
            assert_eq!(value, n % 3);
            assert_eq!(rest, base);
            row = next;
            // Both roots keep all paid dead nodes; no relocation/growth or peak
            // accounting is erased when a handle is popped or discarded.
            old = reference.popped(&mut work, true).unwrap().unwrap().1;
            assert_eq!(root(&bank).statistics(), old.statistics());
            assert_eq!(strong(&bank), 1, "rows do not retain the owner Rc");
        }
        let sorted = kept
            .iter()
            .map(|r| r.materialized(&mut work).unwrap().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            sorted,
            vec![
                vec![usize::MAX, 1, 0],
                vec![usize::MAX, 1, 1],
                vec![usize::MAX, 1, 2]
            ]
        );
        let mut unrelated = equal;
        unrelated.push_return(1, &mut work).unwrap();
        assert_eq!(kept.iter().nth(1).unwrap(), &unrelated);
        let (arena, _, _) = row.bound().unwrap();
        let before = arena.count.get();
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _nodes = Nodes::take(&arena.nodes);
                panic!("authored private loan unwind");
            }))
            .is_err()
        );
        assert_eq!(arena.count.get(), before);
        assert_eq!(
            row.materialized(&mut work).unwrap().as_ref(),
            &[usize::MAX, 1, 1]
        );
        println!(
            "BORROWED_ARENA_LAYOUT_RAW {}",
            serde_json::json!({
                "bank":std::mem::size_of::<ArenaBank>(),"borrowed_row":std::mem::size_of::<BorrowedArenaReturns<'static>>(),
                "rc_row":std::mem::size_of::<ArenaReturns>(),"node_logical_bytes":super::super::NODE_BYTES,
                "borrowed_configuration":std::mem::size_of::<crate::grammar::parser_prediction::PredictionConfiguration<BorrowedArenaReturns<'static>>>(),
                "rc_configuration":std::mem::size_of::<crate::grammar::parser_prediction::PredictionConfiguration<ArenaReturns>>(),
                "growth_statistics":root(&bank).statistics(),"root_rc_strong_count":strong(&bank),"aggregate_rss_bound":false
            })
        );
    }
    fn operation(grant: usize) -> (Result<Vec<usize>, String>, usize) {
        let bank = ArenaBank::new();
        let mut work = Some(grant);
        let result = (|| {
            let mut row = BorrowedArenaReturns::seed(
                &[7],
                &mut Some(BorrowedArenaReturns::unseeded(&bank)),
                &mut work,
            )?;
            for n in 0..9 {
                row.push_return(n, &mut work)?;
            }
            Ok(row.materialized(&mut work)?.into_owned())
        })();
        (result, grant - work.unwrap())
    }
    #[test]
    fn shared_returns_borrowed_arena_zero_exact_short_growth_denial_and_depth_ceiling() {
        let (whole, used) = operation(100000);
        assert_eq!(whole.as_ref().unwrap(), &vec![7, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
        for grant in [0, used - 1, used] {
            let (result, charged) = operation(grant);
            assert_eq!(charged, grant);
            if grant == used {
                assert_eq!(result, whole);
            } else {
                assert!(result.unwrap_err().contains("budget"));
            }
        }
        for pool in [None, Some(WORK_LIMIT + 1), Some(0)] {
            let bank = ArenaBank::new();
            let mut work = pool;
            let result = BorrowedArenaReturns::seed(
                &[],
                &mut Some(BorrowedArenaReturns::unseeded(&bank)),
                &mut work,
            );
            assert!(result.is_err());
            assert_eq!(work, pool);
            assert!(bank.root.get().is_none());
        }
        let bank = ArenaBank::new();
        let mut work = Some(100000);
        let mut row = seed(&bank, &[], &mut work);
        let before = work;
        row.copy_fee(&mut work).unwrap();
        assert_eq!(
            work, before,
            "empty borrowed rows also avoid actual Rc operations"
        );
        for n in 0..crate::grammar::MAX_CALL_DEPTH {
            row.push_return(n, &mut work).unwrap();
        }
        let before = work;
        let stats = root(&bank).statistics();
        assert!(
            row.push_return(crate::grammar::MAX_CALL_DEPTH + 1, &mut work)
                .unwrap_err()
                .contains("depth")
        );
        assert_eq!(work, before);
        assert_eq!(root(&bank).statistics(), stats);
        let (_, mut popped) = row.popped(&mut work, true).unwrap().unwrap();
        popped.push_return(usize::MAX, &mut work).unwrap();
        assert_eq!(
            row.materialized(&mut work).unwrap()[crate::grammar::MAX_CALL_DEPTH - 1],
            crate::grammar::MAX_CALL_DEPTH - 1
        );
        assert_eq!(
            popped.materialized(&mut work).unwrap()[crate::grammar::MAX_CALL_DEPTH - 1],
            usize::MAX
        );
        let other = ArenaBank::new();
        let mut seedwork = Some(10_000_000);
        assert!(
            BorrowedArenaReturns::seed(
                &[0; crate::grammar::MAX_CALL_DEPTH + 1],
                &mut Some(BorrowedArenaReturns::unseeded(&other)),
                &mut seedwork
            )
            .unwrap_err()
            .contains("depth")
        );
        assert!(other.root.get().is_none());
        let bank = ArenaBank::new();
        let mut denied = seed(&bank, &[5], &mut work);
        let before = root(&bank).statistics();
        let old = denied;
        while work.unwrap() > 5 {
            charge(&mut work).unwrap();
        }
        assert!(
            denied
                .push_return(9, &mut work)
                .unwrap_err()
                .contains("budget")
        );
        assert_eq!(work, Some(0));
        assert_eq!(denied, old);
        assert_eq!(root(&bank).statistics(), before);
        assert_eq!(strong(&bank), 1);
    }
}
