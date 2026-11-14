//! Return storage for one canonical runtime prediction. This shares immutable
//! prefixes, not input/recognition results, graph summaries or caller authority.
use super::{charge, live_callers};
use std::{borrow::Cow, cmp::Ordering, fmt::Debug, rc::Rc};

const OWNED_HEADER: usize = 64;
const RETURN_SLOT: usize = 8;

/// The rows one prediction closure has visited. Membership is all that is
/// read, so a storage whose stacks hash cheaply uses a hash set.
pub(super) trait VisitedRows<R>: Default {
    fn insert(&mut self, row: super::PredictionConfiguration<R>) -> bool;
    fn len(&self) -> usize;
}
impl<R: Ord> VisitedRows<R> for std::collections::BTreeSet<super::PredictionConfiguration<R>> {
    fn insert(&mut self, row: super::PredictionConfiguration<R>) -> bool {
        std::collections::BTreeSet::insert(self, row)
    }
    fn len(&self) -> usize {
        std::collections::BTreeSet::len(self)
    }
}
/// Owned return stacks hash as slices.
#[derive(Default)]
pub(super) struct HashedRows(rustc_hash::FxHashSet<super::PredictionConfiguration<Vec<usize>>>);
impl VisitedRows<Vec<usize>> for HashedRows {
    fn insert(&mut self, row: super::PredictionConfiguration<Vec<usize>>) -> bool {
        self.0.insert(row)
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

pub(super) trait Returns: Clone + Debug + Eq + Ord {
    const SHARED: bool;
    type Visited: VisitedRows<Self>;
    const MOVE_SINGLE_EDGE: bool = false;
    const MOVE_LAST_EDGE: bool = false;
    fn empty() -> Self;
    fn from_owned(value: Vec<usize>) -> Result<Self, String>;
    fn seed(
        value: &[usize],
        cached: &mut Option<Self>,
        work: &mut Option<usize>,
    ) -> Result<Self, String>;
    fn len(&self) -> usize;
    fn materialized(&self, work: &mut Option<usize>) -> Result<Cow<'_, [usize]>, String>;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn copy_fee(&self, work: &mut Option<usize>) -> Result<(), String>;
    fn popped(
        &self,
        work: &mut Option<usize>,
        metered: bool,
    ) -> Result<Option<(usize, Self)>, String>;
    fn push_return(&mut self, value: usize, work: &mut Option<usize>) -> Result<(), String>;
}

// The historical/default/eager implementation keeps its exact Vec operations
// and existing live-copy fees. The generic canonical body supplies no new
// allowance or charges when its live context is None.
impl Returns for Vec<usize> {
    type Visited = HashedRows;
    const SHARED: bool = false;
    fn empty() -> Self {
        Vec::new()
    }
    fn from_owned(value: Vec<usize>) -> Result<Self, String> {
        Ok(value)
    }
    fn seed(
        value: &[usize],
        _: &mut Option<Self>,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        live_callers::returns_copy(value.len(), work)?;
        Ok(value.to_vec())
    }
    fn len(&self) -> usize {
        Vec::len(self)
    }
    fn materialized(&self, _: &mut Option<usize>) -> Result<Cow<'_, [usize]>, String> {
        Ok(Cow::Borrowed(Vec::as_slice(self)))
    }
    fn copy_fee(&self, work: &mut Option<usize>) -> Result<(), String> {
        live_callers::returns_copy(self.len(), work)
    }
    fn popped(
        &self,
        work: &mut Option<usize>,
        metered: bool,
    ) -> Result<Option<(usize, Self)>, String> {
        match self.split_last() {
            Some((&target, rest)) => {
                if metered {
                    live_callers::returns_copy(rest.len(), work)?;
                }
                Ok(Some((target, rest.to_vec())))
            }
            None => Ok(None),
        }
    }
    fn push_return(&mut self, value: usize, _: &mut Option<usize>) -> Result<(), String> {
        self.push(value);
        Ok(())
    }
}

// Fixed portable retained node bound: actual parent descriptor + value/length
// and Rc counters must fit on native and Wasm. This is not allocator/RSS data.
const PUSH_NODE_BYTES: usize = 128;
const MATERIALIZED_HEADER_BYTES: usize = 64;
#[derive(Clone, Debug)]
pub(super) enum SharedReturns {
    Empty,
    Prefix { storage: Rc<Vec<usize>>, len: usize },
    Push { storage: Rc<SharedReturnPush> },
}
#[derive(Debug)]
pub(super) struct SharedReturnPush {
    previous: SharedReturns,
    value: usize,
    len: usize,
}
const _: () = {
    assert!(
        std::mem::size_of::<SharedReturnPush>() + 2 * std::mem::size_of::<usize>()
            <= PUSH_NODE_BYTES
    );
    assert!(std::mem::size_of::<Vec<usize>>() <= MATERIALIZED_HEADER_BYTES);
};

#[derive(Clone, Copy)]
enum ReturnView<'a> {
    Empty,
    Prefix(&'a [usize]),
    Push(&'a SharedReturnPush),
}
impl SharedReturns {
    fn view(&self, len: usize) -> ReturnView<'_> {
        let mut current = self;
        loop {
            match current {
                Self::Empty => return ReturnView::Empty,
                Self::Prefix { storage, .. } => return ReturnView::Prefix(&storage[..len]),
                Self::Push { storage } if storage.len == len => return ReturnView::Push(storage),
                Self::Push { storage } => current = &storage.previous,
            }
        }
    }
    // Existing BTreeSet full-key comparisons are atomic/unmetered CPU policy.
    // No new comparison allowance is claimed. This allocates nothing and walks
    // at most the admitted128 ancestors; pointer equality only proves equality
    // of an immutable prefix, never orders identities or substitutes hash keys.
    fn compare_prefix(a: ReturnView<'_>, b: ReturnView<'_>, len: usize) -> Ordering {
        if len == 0 {
            return Ordering::Equal;
        }
        match (a, b) {
            (ReturnView::Prefix(a), ReturnView::Prefix(b)) => a.cmp(b),
            (ReturnView::Push(a), ReturnView::Push(b)) => {
                if std::ptr::eq(a, b) {
                    return Ordering::Equal;
                }
                Self::compare_prefix(a.previous.view(len - 1), b.previous.view(len - 1), len - 1)
                    .then_with(|| a.value.cmp(&b.value))
            }
            (ReturnView::Push(a), ReturnView::Prefix(b)) => Self::compare_prefix(
                a.previous.view(len - 1),
                ReturnView::Prefix(&b[..len - 1]),
                len - 1,
            )
            .then_with(|| a.value.cmp(&b[len - 1])),
            (ReturnView::Prefix(a), ReturnView::Push(b)) => Self::compare_prefix(
                ReturnView::Prefix(&a[..len - 1]),
                b.previous.view(len - 1),
                len - 1,
            )
            .then_with(|| a[len - 1].cmp(&b.value)),
            (ReturnView::Empty, ReturnView::Empty) => Ordering::Equal,
            (ReturnView::Empty, _) => Ordering::Less,
            (_, ReturnView::Empty) => Ordering::Greater,
        }
    }
    #[cfg(test)]
    pub(super) fn has_shared_parent(&self, parent: &Self) -> bool {
        let Self::Push { storage } = self else {
            return false;
        };
        match (&storage.previous, parent) {
            (Self::Empty, Self::Empty) => true,
            (
                Self::Prefix {
                    storage: a,
                    len: a_len,
                },
                Self::Prefix {
                    storage: b,
                    len: b_len,
                },
            ) => a_len == b_len && Rc::ptr_eq(a, b),
            (Self::Push { storage: a }, Self::Push { storage: b }) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
    fn owned(
        values: &[usize],
        appended: Option<usize>,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        let len = values
            .len()
            .checked_add(usize::from(appended.is_some()))
            .ok_or("Shared return length overflow")?;
        if len == 0 {
            return Ok(Self::Empty);
        }
        if len > crate::grammar::MAX_CALL_DEPTH {
            return Err("Parser prediction call depth exceeded".into());
        }
        const {
            assert!(std::mem::size_of::<usize>() <= RETURN_SLOT);
            assert!(
                std::mem::size_of::<Vec<usize>>() + 2 * std::mem::size_of::<usize>()
                    <= OWNED_HEADER
            );
        }
        // Fixed portable policy units cover Rc counts + Vec header and every
        // retained return slot. They are not allocator/RSS measurements.
        charge(work)?;
        for _ in 0..OWNED_HEADER.div_ceil(64) {
            charge(work)?;
        }
        charge(work)?;
        let bytes = len
            .checked_mul(RETURN_SLOT)
            .ok_or("Shared return storage overflow")?;
        for _ in 0..bytes.div_ceil(64) {
            charge(work)?;
        }
        // Pay the complete copy/append before either allocation or retention.
        for _ in 0..len {
            charge(work)?;
        }
        let mut storage = Vec::new();
        storage
            .try_reserve_exact(len)
            .map_err(|_| "Shared return allocation denied")?;
        storage.extend_from_slice(values);
        if let Some(value) = appended {
            storage.push(value);
        }
        Ok(Self::Prefix {
            storage: Rc::new(storage),
            len,
        })
    }
}
impl PartialEq for SharedReturns {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for SharedReturns {}
impl PartialOrd for SharedReturns {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SharedReturns {
    fn cmp(&self, other: &Self) -> Ordering {
        let len = self.len().min(other.len());
        Self::compare_prefix(self.view(len), other.view(len), len)
            .then_with(|| self.len().cmp(&other.len()))
    }
}
impl Returns for SharedReturns {
    type Visited = std::collections::BTreeSet<super::PredictionConfiguration<Self>>;
    const SHARED: bool = true;
    const MOVE_SINGLE_EDGE: bool = true;
    const MOVE_LAST_EDGE: bool = true;
    fn empty() -> Self {
        Self::Empty
    }
    fn from_owned(_: Vec<usize>) -> Result<Self, String> {
        Err("Shared returns require the exact live caller context".into())
    }
    fn seed(
        value: &[usize],
        cached: &mut Option<Self>,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        if cached.is_none() {
            *cached = Some(Self::owned(value, None, work)?);
        }
        let source = cached.as_ref().ok_or("Missing paid shared return seed")?;
        source.copy_fee(work)?;
        Ok(source.clone())
    }
    fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Prefix { len, .. } => *len,
            Self::Push { storage } => storage.len,
        }
    }
    fn materialized(&self, work: &mut Option<usize>) -> Result<Cow<'_, [usize]>, String> {
        match self {
            Self::Empty => return Ok(Cow::Borrowed(&[])),
            Self::Prefix { storage, len } => return Ok(Cow::Borrowed(&storage[..*len])),
            Self::Push { .. } => {}
        }
        let len = self.len();
        if len > crate::grammar::MAX_CALL_DEPTH {
            return Err("Parser prediction call depth exceeded".into());
        }
        charge(work)?; // actual temporary-vector allocation
        for _ in 0..MATERIALIZED_HEADER_BYTES.div_ceil(64) {
            charge(work)?;
        }
        let bytes = len
            .checked_mul(RETURN_SLOT)
            .ok_or("Shared return materialization storage overflow")?;
        for _ in 0..bytes.div_ceil(64) {
            charge(work)?;
        }
        // Initialize every slot, traverse at mostlen nodes, and write each exact
        // value. All copying/retention is paid before allocation or traversal.
        let steps = len
            .checked_mul(3)
            .ok_or("Shared return materialization work overflow")?;
        for _ in 0..steps {
            charge(work)?;
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(len)
            .map_err(|_| "Shared return materialization allocation denied")?;
        values.resize(len, 0);
        let mut current = self;
        let mut end = len;
        loop {
            match current {
                Self::Empty => {
                    if end != 0 {
                        return Err("Invalid shared return materialization prefix".into());
                    }
                    break;
                }
                Self::Prefix { storage, len } => {
                    if *len != end {
                        return Err("Invalid shared return materialization seed".into());
                    }
                    values[..end].copy_from_slice(&storage[..*len]);
                    break;
                }
                Self::Push { storage } => {
                    if storage.len != end {
                        return Err("Invalid shared return materialization node".into());
                    }
                    end = end
                        .checked_sub(1)
                        .ok_or("Invalid shared return materialization depth")?;
                    values[end] = storage.value;
                    current = &storage.previous;
                }
            }
        }
        Ok(Cow::Owned(values))
    }
    fn copy_fee(&self, work: &mut Option<usize>) -> Result<(), String> {
        if !matches!(self, Self::Empty) {
            charge(work)?;
        }
        Ok(())
    }
    fn popped(&self, work: &mut Option<usize>, _: bool) -> Result<Option<(usize, Self)>, String> {
        match self {
            Self::Empty => Ok(None),
            Self::Prefix { storage, len } => {
                let (&target, rest) = storage[..*len]
                    .split_last()
                    .ok_or("Invalid shared return prefix")?;
                charge(work)?;
                let next = if rest.is_empty() {
                    Self::Empty
                } else {
                    self.copy_fee(work)?;
                    Self::Prefix {
                        storage: Rc::clone(storage),
                        len: rest.len(),
                    }
                };
                Ok(Some((target, next)))
            }
            Self::Push { storage } => {
                charge(work)?;
                storage.previous.copy_fee(work)?;
                Ok(Some((storage.value, storage.previous.clone())))
            }
        }
    }
    fn push_return(&mut self, value: usize, work: &mut Option<usize>) -> Result<(), String> {
        let len = self
            .len()
            .checked_add(1)
            .ok_or("Shared return length overflow")?;
        if len > crate::grammar::MAX_CALL_DEPTH {
            return Err("Parser prediction call depth exceeded".into());
        }
        charge(work)?; // allocation/placement, never a full-prefix copy
        for _ in 0..PUSH_NODE_BYTES.div_ceil(64) {
            charge(work)?;
        }
        charge(work)?; // immutable parent descriptor/link (including Rc clone)
        charge(work)?; // exact follow value
        charge(work)?; // cached length
        let previous = self.clone();
        *self = Self::Push {
            storage: Rc::new(SharedReturnPush {
                previous,
                value,
                len,
            }),
        };
        Ok(())
    }
}

#[cfg(test)]
#[path = "flat_shared_returns_reference.rs"]
pub(super) mod flat_reference;

#[path = "arena_returns.rs"]
pub(super) mod arena;
