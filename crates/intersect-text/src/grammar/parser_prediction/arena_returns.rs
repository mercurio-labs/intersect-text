//! Invocation-local append-only returns. No node is a graph/source/input fact.
use super::{Returns, SharedReturns, charge};
use std::{borrow::Cow, cell::Cell, cmp::Ordering, fmt, rc::Rc};

const NODE_BYTES: usize = 16;
const ARENA_HEADER_BYTES: usize = 192;
// Seed prefix codes cover every length up to the structural ceiling.
const FIRST_NODE: usize = crate::grammar::MAX_CALL_DEPTH + 1;
const MAX_NODES: usize = 32768;
// Finite allowed maximum only; each caller retains its own original grant.
// 4b and its first rejected value fit wasm32 usize. Node/depth/storage bounds
// remain independent and every existing allocation/copy/debit remains paid.
pub(in crate::grammar::parser_prediction) const WORK_LIMIT: usize = 4_000_000_000;
const MATERIALIZED_HEADER: usize = 64;

#[derive(Clone, Copy)]
struct Node {
    value: usize,
    parent: usize,
}
struct Arena {
    seed: Rc<Vec<usize>>,
    nodes: Cell<Vec<Node>>,
    count: Cell<usize>,
    capacity: Cell<usize>,
    cumulative_bytes: Cell<usize>,
    peak_bytes: Cell<usize>,
    growths: Cell<usize>,
}
const _: () = {
    assert!(std::mem::size_of::<Node>() <= NODE_BYTES);
    assert!(std::mem::size_of::<Arena>() + 2 * std::mem::size_of::<usize>() <= ARENA_HEADER_BYTES);
};

// A header move only. No call to input, observation, Debug, trait-object code,
// comparison or fallible budget admission occurs while a slot is absent.
// Even allocation error/unwind restores the original complete vector.
struct Nodes<'a> {
    slot: &'a Cell<Vec<Node>>,
    data: Vec<Node>,
}
impl<'a> Nodes<'a> {
    fn take(slot: &'a Cell<Vec<Node>>) -> Self {
        Self {
            slot,
            data: slot.take(),
        }
    }
}
impl Drop for Nodes<'_> {
    fn drop(&mut self) {
        self.slot.set(std::mem::take(&mut self.data));
    }
}

#[derive(Clone)]
pub(in crate::grammar::parser_prediction) struct ArenaReturns(State);
const _: () = {
    assert!(std::mem::size_of::<ArenaReturns>() <= 64);
};
#[derive(Clone)]
enum State {
    // Required only by the trait; the same Shared worker guard rejects a
    // completion/non-live invocation before constructing such configurations.
    Unseeded,
    Bound {
        arena: Rc<Arena>,
        head: usize,
        len: usize,
    },
}
impl fmt::Debug for ArenaReturns {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            State::Unseeded => f.write_str("UnseededArenaReturns"),
            State::Bound { head, len, .. } => f
                .debug_struct("ArenaReturns")
                .field("head", head)
                .field("len", len)
                .finish(),
        }
    }
}
fn debit(work: &mut Option<usize>, units: usize) -> Result<(), String> {
    for _ in 0..units {
        charge(work)?;
    }
    Ok(())
}
impl ArenaReturns {
    // All heads are private. Seed prefix codes0..128 are exact prefix lengths;
    // node codes129+i are minted only after complete append i. Parents always
    // refer to the prior valid head. Pop/clone preserve that invariant. Following
    // at most len<=128 links therefore yields exactly the original Vec key.
    fn key(&self, nodes: &[Node], out: &mut [usize]) -> usize {
        let State::Bound { arena, head, len } = &self.0 else {
            return 0;
        };
        arena.key(*head, *len, nodes, out)
    }
    fn valid(arena: &Arena, head: usize, len: usize) -> bool {
        len <= crate::grammar::MAX_CALL_DEPTH
            && if head < FIRST_NODE {
                head <= arena.seed.len() && len == head
            } else {
                len != 0 && head - FIRST_NODE < arena.count.get()
            }
    }
    #[cfg(test)]
    pub(in crate::grammar::parser_prediction) fn statistics(&self) -> (usize, usize, usize, usize) {
        match &self.0 {
            State::Bound { arena, .. } => (
                arena.count.get(),
                arena.growths.get(),
                arena.cumulative_bytes.get(),
                arena.peak_bytes.get(),
            ),
            State::Unseeded => (0, 0, 0, 0),
        }
    }
}
impl PartialEq for ArenaReturns {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for ArenaReturns {}
impl PartialOrd for ArenaReturns {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ArenaReturns {
    fn cmp(&self, other: &Self) -> Ordering {
        // Existing full-key comparisons are atomic/unmetered CPU policy. This
        // adds no comparator allowance, allocation or callbacks. Two fixed
        // 128-slot stack keys are not retained per configuration.
        let mut a = [0; crate::grammar::MAX_CALL_DEPTH];
        let mut b = [0; crate::grammar::MAX_CALL_DEPTH];
        match (&self.0, &other.0) {
            (State::Unseeded, _) => 0.cmp(&other.len()),
            (_, State::Unseeded) => self.len().cmp(&0),
            (
                State::Bound {
                    arena: left,
                    head: ah,
                    len: al,
                },
                State::Bound {
                    arena: right,
                    head: bh,
                    len: bl,
                },
            ) => {
                // Detect alias BEFORE taking. Different arenas own different
                // Cell slots; no duplicate take or fabricated ordering fallback.
                if Rc::ptr_eq(left, right) {
                    if ah == bh && al == bl {
                        return Ordering::Equal;
                    }
                    let nodes = Nodes::take(&left.nodes);
                    let an = self.key(&nodes.data, &mut a);
                    let bn = other.key(&nodes.data, &mut b);
                    a[..an].cmp(&b[..bn])
                } else {
                    let left_nodes = Nodes::take(&left.nodes);
                    let right_nodes = Nodes::take(&right.nodes);
                    let an = self.key(&left_nodes.data, &mut a);
                    let bn = other.key(&right_nodes.data, &mut b);
                    a[..an].cmp(&b[..bn])
                }
            }
        }
    }
}
impl Returns for ArenaReturns {
    type Visited = std::collections::BTreeSet<
        crate::grammar::parser_prediction::PredictionConfiguration<Self>,
    >;
    const SHARED: bool = true;
    const MOVE_SINGLE_EDGE: bool = true;
    const MOVE_LAST_EDGE: bool = true;
    fn empty() -> Self {
        Self(State::Unseeded)
    }
    fn from_owned(_: Vec<usize>) -> Result<Self, String> {
        Err("Arena returns require the exact live caller context".into())
    }
    fn seed(
        value: &[usize],
        cached: &mut Option<Self>,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        if work
            .as_ref()
            .is_none_or(|remaining| *remaining > WORK_LIMIT)
        {
            return Err(
                "Arena returns require the original finite at-most4000000000 work pool".into(),
            );
        }
        if cached.is_none() {
            // Preserve the exact existing paid seed implementation and owned
            // source values. No per-Call seed/prefix copy is introduced.
            let paid = SharedReturns::owned(value, None, work)?;
            let seed = match paid {
                SharedReturns::Prefix { storage, .. } => storage,
                SharedReturns::Empty => {
                    debit(work, 2)?; // actual empty Rc<Vec> allocation +64-byte header
                    Rc::new(Vec::new())
                }
                SharedReturns::Push { .. } => {
                    return Err("Unexpected persistent return seed".into());
                }
            };
            debit(work, 1 + ARENA_HEADER_BYTES.div_ceil(64))?;
            let seed_bytes = 64usize
                .checked_add(
                    value
                        .len()
                        .checked_mul(8)
                        .ok_or("Arena seed storage overflow")?,
                )
                .ok_or("Arena seed storage overflow")?;
            let header = ARENA_HEADER_BYTES
                .checked_add(seed_bytes)
                .ok_or("Arena header storage overflow")?;
            *cached = Some(Self(State::Bound {
                arena: Rc::new(Arena {
                    seed,
                    nodes: Cell::new(Vec::new()),
                    count: Cell::new(0),
                    capacity: Cell::new(0),
                    cumulative_bytes: Cell::new(header),
                    peak_bytes: Cell::new(header),
                    growths: Cell::new(0),
                }),
                head: value.len(),
                len: value.len(),
            }));
        }
        let source = cached.as_ref().ok_or("Missing paid arena return seed")?;
        source.copy_fee(work)?; // includes len0: the arena Rc is really cloned
        Ok(source.clone())
    }
    fn len(&self) -> usize {
        match &self.0 {
            State::Unseeded => 0,
            State::Bound { len, .. } => *len,
        }
    }
    fn copy_fee(&self, work: &mut Option<usize>) -> Result<(), String> {
        let State::Bound { arena, head, len } = &self.0 else {
            return Err("Missing paid arena return seed".into());
        };
        if !Self::valid(arena, *head, *len) {
            return Err("Invalid private arena return head".into());
        }
        charge(work) // every actual arena Rc clone, including empty returns
    }
    fn popped(&self, work: &mut Option<usize>, _: bool) -> Result<Option<(usize, Self)>, String> {
        let State::Bound { arena, head, len } = &self.0 else {
            return Err("Missing paid arena return seed".into());
        };
        if !Self::valid(arena, *head, *len) {
            return Err("Invalid private arena return head".into());
        }
        if *len == 0 {
            return Ok(None);
        }
        charge(work)?; // actual pop/selection
        self.copy_fee(work)?; // returned row owns one Rc, even when it becomes empty
        let (value, parent) = if *head < FIRST_NODE {
            (arena.seed[*head - 1], *head - 1)
        } else {
            let nodes = Nodes::take(&arena.nodes);
            let node = nodes.data[*head - FIRST_NODE];
            (node.value, node.parent)
        };
        Ok(Some((
            value,
            Self(State::Bound {
                arena: Rc::clone(arena),
                head: parent,
                len: len - 1,
            }),
        )))
    }
    fn push_return(&mut self, value: usize, work: &mut Option<usize>) -> Result<(), String> {
        let State::Bound { arena, head, len } = &mut self.0 else {
            return Err("Missing paid arena return seed".into());
        };
        arena.append(head, len, value, work)
    }
    fn materialized(&self, work: &mut Option<usize>) -> Result<Cow<'_, [usize]>, String> {
        let State::Bound { arena, head, len } = &self.0 else {
            return Err("Missing paid arena return seed".into());
        };
        arena.materialized(*head, *len, work)
    }
}

// Both carriers use these exact node/key/materialization operations.
impl Arena {
    fn key(&self, head: usize, len: usize, nodes: &[Node], out: &mut [usize]) -> usize {
        let mut head = head;
        let mut end = len;
        while head >= FIRST_NODE {
            let node = nodes[head - FIRST_NODE];
            end -= 1;
            out[end] = node.value;
            head = node.parent;
        }
        out[..end].copy_from_slice(&self.seed[..head]);
        len
    }
    fn append(
        &self,
        head: &mut usize,
        len: &mut usize,
        value: usize,
        work: &mut Option<usize>,
    ) -> Result<(), String> {
        if !ArenaReturns::valid(self, *head, *len) {
            return Err("Invalid private arena return head".into());
        }
        let new_len = len.checked_add(1).ok_or("Arena return length overflow")?;
        if new_len > crate::grammar::MAX_CALL_DEPTH {
            return Err("Parser prediction call depth exceeded".into());
        }
        let count = self.count.get();
        if count >= MAX_NODES {
            return Err("Invocation arena node storage bound exceeded".into());
        }
        let capacity = self.capacity.get();
        let growing = count == capacity;
        let new_capacity = if growing {
            if capacity == 0 {
                1
            } else {
                capacity.checked_mul(2).ok_or("Arena capacity overflow")?
            }
        } else {
            capacity
        };
        if new_capacity > MAX_NODES {
            return Err("Invocation arena capacity bound exceeded".into());
        }
        let new_bytes = new_capacity
            .checked_mul(NODE_BYTES)
            .ok_or("Arena growth storage overflow")?;
        let old_bytes = count
            .checked_mul(NODE_BYTES)
            .ok_or("Arena relocation storage overflow")?;
        let cumulative = if growing {
            self.cumulative_bytes
                .get()
                .checked_add(new_bytes)
                .ok_or("Arena cumulative storage overflow")?
        } else {
            self.cumulative_bytes.get()
        };
        let peak = if growing {
            ARENA_HEADER_BYTES
                .checked_add(64 + self.seed.len() * 8)
                .and_then(|n| n.checked_add(old_bytes))
                .and_then(|n| n.checked_add(new_bytes))
                .ok_or("Arena peak storage overflow")?
        } else {
            self.peak_bytes.get()
        };
        let growths = self
            .growths
            .get()
            .checked_add(usize::from(growing))
            .ok_or("Arena growth count overflow")?;
        // Preserve payload/parent/length work; add the actual Vec append/length
        // update. Only per-node Rc allocation/header is replaced by paid growth.
        debit(work, 4)?;
        if growing {
            debit(work, 1 + new_bytes.div_ceil(64))?; // actual new buffer allocation/admission
            if count != 0 {
                debit(work, 1 + old_bytes.div_ceil(64))?;
            } // actual bulk relocation, existing64-byte Vec-copy tariff
        }
        // All admissions precede taking. This scope invokes only concrete Copy
        // node operations and fallible Vec allocation; no external callbacks.
        let mut nodes = Nodes::take(&self.nodes);
        if nodes.data.len() != count {
            return Err("Arena append count differs from private head ledger".into());
        }
        if growing {
            let mut fresh = Vec::new();
            fresh
                .try_reserve_exact(new_capacity)
                .map_err(|_| "Arena return allocation denied")?;
            fresh.extend_from_slice(&nodes.data);
            nodes.data = fresh;
        }
        let new_head = FIRST_NODE.checked_add(count).ok_or("Arena head overflow")?;
        nodes.data.push(Node {
            value,
            parent: *head,
        });
        self.count.set(count + 1);
        self.capacity.set(new_capacity);
        self.cumulative_bytes.set(cumulative);
        self.peak_bytes.set(self.peak_bytes.get().max(peak));
        self.growths.set(growths);
        drop(nodes); // restore complete vector BEFORE publishing the new row head
        *head = new_head;
        *len = new_len;
        Ok(())
    }
    fn materialized(
        &self,
        head: usize,
        len: usize,
        work: &mut Option<usize>,
    ) -> Result<Cow<'_, [usize]>, String> {
        if !ArenaReturns::valid(self, head, len) {
            return Err("Invalid private arena return head".into());
        }
        if head < FIRST_NODE {
            return Ok(Cow::Borrowed(&self.seed[..head]));
        }
        let bytes = len
            .checked_mul(8)
            .ok_or("Arena materialization storage overflow")?;
        debit(
            work,
            1 + MATERIALIZED_HEADER.div_ceil(64) + bytes.div_ceil(64),
        )?;
        debit(
            work,
            len.checked_mul(3)
                .ok_or("Arena materialization work overflow")?,
        )?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(len)
            .map_err(|_| "Arena materialization allocation denied")?;
        values.resize(len, 0);
        {
            let nodes = Nodes::take(&self.nodes);
            self.key(head, len, &nodes.data, &mut values);
        }
        Ok(Cow::Owned(values))
    }
}

#[path = "arena_borrowed_returns.rs"]
pub(in crate::grammar::parser_prediction) mod borrowed;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    fn seeded(values: &[usize], work: &mut Option<usize>) -> ArenaReturns {
        ArenaReturns::seed(values, &mut None, work).unwrap()
    }
    // Growth preserves each key's seed, head, length and old node values.
    // This test intentionally checks real BTreeSet ordering while storage grows.
    #[allow(clippy::mutable_key_type)]
    #[test]
    fn shared_returns_arena_growth_pop_push_full_order_and_restore_are_real() {
        let mut work = Some(100000);
        let base = seeded(&[usize::MAX, 1], &mut work);
        let equal = seeded(&[usize::MAX, 1], &mut work);
        assert_eq!(base, equal);
        let mut kept = BTreeSet::new();
        for n in 0..257 {
            base.copy_fee(&mut work).unwrap();
            let mut row = base.clone();
            row.push_return(n % 3, &mut work).unwrap();
            assert_eq!(
                row.materialized(&mut work).unwrap().as_ref(),
                &[usize::MAX, 1, n % 3]
            );
            row.copy_fee(&mut work).unwrap();
            kept.insert(row.clone());
            let (value, rest) = row.popped(&mut work, true).unwrap().unwrap();
            assert_eq!(value, n % 3);
            assert_eq!(rest, base);
            assert_eq!(kept.len(), (n + 1).min(3));
        }
        let (count, growths, cumulative, peak) = base.statistics();
        assert_eq!((count, growths), (257, 10));
        assert_eq!(cumulative, ARENA_HEADER_BYTES + 64 + 16 + 1023 * NODE_BYTES);
        assert_eq!(
            peak,
            ARENA_HEADER_BYTES + 64 + 16 + (256 + 512) * NODE_BYTES
        );
        let sorted: Vec<_> = kept
            .iter()
            .map(|r| r.materialized(&mut work).unwrap().into_owned())
            .collect();
        assert_eq!(
            sorted,
            vec![
                vec![usize::MAX, 1, 0],
                vec![usize::MAX, 1, 1],
                vec![usize::MAX, 1, 2]
            ]
        );
        // Growth never alters old keys, even across two different arenas whose
        // node indices and allocation identities have no ordering authority.
        let mut other = equal;
        other.push_return(1, &mut work).unwrap();
        assert_eq!(kept.iter().nth(1).unwrap(), &other);
        let State::Bound { arena, .. } = &base.0 else {
            panic!("seed")
        };
        let before = arena.count.get();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = Nodes::take(&arena.nodes);
            panic!("authored private guard unwind");
        }));
        assert!(panic.is_err());
        assert_eq!(arena.count.get(), before);
        assert_eq!(base, seeded(&[usize::MAX, 1], &mut work));
        let restored_len = {
            let guard = Nodes::take(&arena.nodes);
            guard.data.len()
        };
        assert_eq!(restored_len, before);
    }
    fn operation(grant: usize) -> (Result<Vec<usize>, String>, usize) {
        let mut work = Some(grant);
        let result = (|| {
            let mut row = ArenaReturns::seed(&[7], &mut None, &mut work)?;
            for n in 0..9 {
                row.push_return(n, &mut work)?;
            }
            Ok(row.materialized(&mut work)?.into_owned())
        })();
        (result, grant - work.unwrap())
    }
    #[test]
    fn shared_returns_arena_growth_admission_exact_short_empty_clone_and_depth_ceiling() {
        let (whole, used) = operation(100000);
        assert_eq!(whole.as_ref().unwrap(), &vec![7, 0, 1, 2, 3, 4, 5, 6, 7, 8]);
        for grant in [0, used - 1, used] {
            let (actual, charged) = operation(grant);
            assert_eq!(charged, grant);
            if grant == used {
                assert_eq!(actual, whole);
            } else {
                assert!(actual.unwrap_err().contains("budget"));
            }
        }
        let mut work = Some(100000);
        let mut row = seeded(&[], &mut work);
        let before = work;
        row.copy_fee(&mut work).unwrap();
        assert_eq!(
            before.unwrap() - work.unwrap(),
            1,
            "empty carrier really owns arena Rc"
        );
        let before_first = work;
        row.push_return(0, &mut work).unwrap();
        assert_eq!(before_first.unwrap() - work.unwrap(), 6);
        let before_growth = work;
        row.push_return(1, &mut work).unwrap();
        assert_eq!(
            before_growth.unwrap() - work.unwrap(),
            8,
            "cold relocation is really paid, not a universal saving"
        );
        for n in 2..crate::grammar::MAX_CALL_DEPTH {
            row.push_return(n, &mut work).unwrap();
        }
        let state = row.statistics();
        let remaining = work;
        assert!(
            row.push_return(999, &mut work)
                .unwrap_err()
                .contains("depth")
        );
        assert_eq!(work, remaining);
        assert_eq!(row.statistics(), state);
        let (value, mut popped) = row.popped(&mut work, true).unwrap().unwrap();
        assert_eq!(value, crate::grammar::MAX_CALL_DEPTH - 1);
        popped.push_return(usize::MAX, &mut work).unwrap();
        assert_eq!(popped.len(), crate::grammar::MAX_CALL_DEPTH);
        assert_eq!(
            row.materialized(&mut work).unwrap()[crate::grammar::MAX_CALL_DEPTH - 1],
            crate::grammar::MAX_CALL_DEPTH - 1
        );
        assert_eq!(
            popped.materialized(&mut work).unwrap()[crate::grammar::MAX_CALL_DEPTH - 1],
            usize::MAX
        );
        // Genuine prior debit on the same original pool denies growth before
        // taking/mutating any vector or publishing a new head.
        let mut deny = Some(100000);
        let mut denied = seeded(&[5], &mut deny);
        let stats = denied.statistics();
        while deny.unwrap() > 5 {
            charge(&mut deny).unwrap();
        }
        assert!(
            denied
                .push_return(9, &mut deny)
                .unwrap_err()
                .contains("budget")
        );
        assert_eq!(deny, Some(0));
        assert_eq!(denied.statistics(), stats);
        assert_eq!(denied.len(), 1);
        assert!(ArenaReturns::seed(&[], &mut None, &mut Some(WORK_LIMIT + 1)).is_err());
    }
}
