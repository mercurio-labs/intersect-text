//! Immutable prefixes for ordinary prediction. The canonical worker retains
//! its charges, traversal and input protocol; row copies share only stack data.
use super::shared_returns::{Returns, VisitedRows};
use std::{borrow::Cow, cmp::Ordering, hash::Hash, rc::Rc};

#[derive(Clone, Debug, Default)]
pub(super) struct OrdinaryReturns {
    storage: Option<Rc<Vec<usize>>>,
    len: usize,
}

impl OrdinaryReturns {
    fn values(&self) -> &[usize] {
        match &self.storage {
            Some(storage) => &storage[..self.len],
            None => &[],
        }
    }
}

impl PartialEq for OrdinaryReturns {
    fn eq(&self, other: &Self) -> bool {
        self.values() == other.values()
    }
}
impl Eq for OrdinaryReturns {}
impl PartialOrd for OrdinaryReturns {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for OrdinaryReturns {
    fn cmp(&self, other: &Self) -> Ordering {
        self.values().cmp(other.values())
    }
}
impl Hash for OrdinaryReturns {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.values().hash(state);
    }
}

#[derive(Default)]
pub(super) struct Rows(rustc_hash::FxHashSet<super::PredictionConfiguration<OrdinaryReturns>>);
impl VisitedRows<OrdinaryReturns> for Rows {
    fn insert(&mut self, row: super::PredictionConfiguration<OrdinaryReturns>) -> bool {
        self.0.insert(row)
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl Returns for OrdinaryReturns {
    // No live-return accounting or prepared-source authority is enabled.
    const SHARED: bool = false;
    type Visited = Rows;
    const MOVE_SINGLE_EDGE: bool = true;
    const MOVE_LAST_EDGE: bool = true;

    fn empty() -> Self {
        Self::default()
    }
    fn from_owned(value: Vec<usize>) -> Result<Self, String> {
        let len = value.len();
        Ok(Self {
            storage: (!value.is_empty()).then(|| Rc::new(value)),
            len,
        })
    }
    fn seed(
        value: &[usize],
        _: &mut Option<Self>,
        work: &mut Option<usize>,
    ) -> Result<Self, String> {
        super::live_callers::returns_copy(value.len(), work)?;
        Self::from_owned(value.to_vec())
    }
    fn len(&self) -> usize {
        self.len
    }
    fn materialized(&self, _: &mut Option<usize>) -> Result<Cow<'_, [usize]>, String> {
        Ok(Cow::Borrowed(self.values()))
    }
    fn copy_fee(&self, work: &mut Option<usize>) -> Result<(), String> {
        super::live_callers::returns_copy(self.len, work)
    }
    fn popped(
        &self,
        work: &mut Option<usize>,
        metered: bool,
    ) -> Result<Option<(usize, Self)>, String> {
        let Some((&target, rest)) = self.values().split_last() else {
            return Ok(None);
        };
        if metered {
            super::live_callers::returns_copy(rest.len(), work)?;
        }
        let prefix = if rest.is_empty() {
            Self::empty()
        } else {
            Self {
                storage: self.storage.clone(),
                len: rest.len(),
            }
        };
        Ok(Some((target, prefix)))
    }
    fn push_return(&mut self, value: usize, _: &mut Option<usize>) -> Result<(), String> {
        let len = self
            .len
            .checked_add(1)
            .ok_or("Prediction return length overflow")?;
        if let Some(storage) = self.storage.as_mut().and_then(Rc::get_mut) {
            storage.truncate(self.len);
            storage.push(value);
        } else {
            // Copy only the visible prefix, never the popped suffix. Visited
            // rows and siblings retain their immutable exact stack values.
            let mut storage = Vec::with_capacity(len);
            storage.extend_from_slice(self.values());
            storage.push(value);
            self.storage = Some(Rc::new(storage));
        }
        self.len = len;
        Ok(())
    }
}
