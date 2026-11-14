// Exact prior flat storage reference; same generic worker, tests only.
use super::*;
#[derive(Clone, Debug)]
pub(in crate::grammar::parser_prediction) enum FlatSharedReturns {
    Empty,
    Prefix { storage: Rc<Vec<usize>>, len: usize },
}
impl FlatSharedReturns {
    fn values(&self) -> &[usize] {
        match self {
            Self::Empty => &[],
            // Private constructors enforce 0<len<=storage.len(). No mutable
            // access to the Rc-owned Vec is exposed or used after publication.
            Self::Prefix { storage, len } => &storage[..*len],
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
impl PartialEq for FlatSharedReturns {
    fn eq(&self, other: &Self) -> bool {
        self.values() == other.values()
    }
}
impl Eq for FlatSharedReturns {}
impl PartialOrd for FlatSharedReturns {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FlatSharedReturns {
    fn cmp(&self, other: &Self) -> Ordering {
        self.values().cmp(other.values())
    }
}

impl Returns for FlatSharedReturns {
    type Visited = std::collections::BTreeSet<
        crate::grammar::parser_prediction::PredictionConfiguration<Self>,
    >;
    const SHARED: bool = true;
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
        self.values().len()
    }
    fn materialized(&self, _: &mut Option<usize>) -> Result<Cow<'_, [usize]>, String> {
        Ok(Cow::Borrowed(self.values()))
    }
    fn copy_fee(&self, work: &mut Option<usize>) -> Result<(), String> {
        // Empty is an allocation-free scalar. A nonempty clone increments one
        // Rc counter and copies the fixed prefix descriptor, without Vec clone.
        if matches!(self, Self::Prefix { .. }) {
            charge(work)?;
        }
        Ok(())
    }
    fn popped(&self, work: &mut Option<usize>, _: bool) -> Result<Option<(usize, Self)>, String> {
        let Some((&target, rest)) = self.values().split_last() else {
            return Ok(None);
        };
        charge(work)?;
        let next = if rest.is_empty() {
            Self::Empty
        } else {
            self.copy_fee(work)?;
            let Self::Prefix { storage, .. } = self else {
                return Err("Invalid shared return prefix".into());
            };
            Self::Prefix {
                storage: Rc::clone(storage),
                len: rest.len(),
            }
        };
        Ok(Some((target, next)))
    }
    fn push_return(&mut self, value: usize, work: &mut Option<usize>) -> Result<(), String> {
        *self = Self::owned(self.values(), Some(value), work)?;
        Ok(())
    }
}
