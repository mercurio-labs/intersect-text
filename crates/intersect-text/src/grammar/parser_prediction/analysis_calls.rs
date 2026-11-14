//! Physical full (callsite,follow) storage for the SAME compiler classifier.
//! This module is private preparation, not an enabled compiler/read authority.
use super::analysis_charge;
use std::{cmp::Ordering, fmt::Debug, ops::Deref, rc::Rc};

pub(super) type Pair = (usize, usize);
pub(super) type VecCalls = Vec<Pair>;
const OWNED_HEADER: usize = 64;
const PAIR_BYTES: usize = 16;
const MAX_LOGICAL_BYTES: usize = 1_048_576;

pub(super) trait Calls: Clone + Debug + Eq + Ord + Deref<Target = [Pair]> {
    type Scope;
    const SHARED: bool;
    fn scope(work: &mut usize) -> Result<Self::Scope, String>;
    fn empty() -> Self;
    fn pushed(
        &mut self,
        pair: Pair,
        scope: &mut Self::Scope,
        work: &mut usize,
    ) -> Result<(), String>;
    fn popped(
        &mut self,
        scope: &mut Self::Scope,
        work: &mut usize,
    ) -> Result<Option<(Pair, Self)>, String>;
    fn copied_pairs(&self) -> usize;
    fn snapshot_copy_fee(&self, scope: &mut Self::Scope, work: &mut usize) -> Result<(), String>;
}
impl Calls for VecCalls {
    type Scope = ();
    const SHARED: bool = false;
    fn scope(_: &mut usize) -> Result<(), String> {
        Ok(())
    }
    fn empty() -> Self {
        Vec::new()
    }
    fn pushed(&mut self, pair: Pair, _: &mut (), _: &mut usize) -> Result<(), String> {
        self.push(pair);
        Ok(())
    }
    fn popped(&mut self, _: &mut (), _: &mut usize) -> Result<Option<(Pair, Self)>, String> {
        Ok(self.split_last().map(|(&pair, rest)| (pair, rest.to_vec())))
    }
    fn copied_pairs(&self) -> usize {
        self.len()
    }
    fn snapshot_copy_fee(&self, _: &mut (), _: &mut usize) -> Result<(), String> {
        Ok(())
    }
}

pub(super) struct SharedScope {
    logical_bytes: usize,
    failed: bool,
}
#[derive(Clone, Debug)]
pub(super) enum SharedCalls {
    Empty,
    Prefix { storage: Rc<VecCalls>, len: usize },
}
impl Deref for SharedCalls {
    type Target = [Pair];
    fn deref(&self) -> &[Pair] {
        match self {
            Self::Empty => &[],
            Self::Prefix { storage, len } => &storage[..*len],
        }
    }
}
impl PartialEq for SharedCalls {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl Eq for SharedCalls {}
impl PartialOrd for SharedCalls {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SharedCalls {
    fn cmp(&self, other: &Self) -> Ordering {
        (**self).cmp(&**other)
    }
}
impl SharedCalls {
    fn owned(
        values: &[Pair],
        pair: Pair,
        scope: &mut SharedScope,
        work: &mut usize,
    ) -> Result<Self, String> {
        if scope.failed {
            return Err("Shared compiler call storage is poisoned".into());
        }
        let result = (|| {
            let len = values
                .len()
                .checked_add(1)
                .filter(|n| *n <= crate::grammar::DEFAULT_CALL_DEPTH)
                .ok_or("Parser prediction call depth exceeded")?;
            const {
                assert!(std::mem::size_of::<Pair>() <= PAIR_BYTES);
                assert!(
                    std::mem::size_of::<VecCalls>() + 2 * std::mem::size_of::<usize>()
                        <= OWNED_HEADER
                );
            }
            let bytes = len
                .checked_mul(PAIR_BYTES)
                .and_then(|n| n.checked_add(OWNED_HEADER))
                .ok_or("Shared compiler call storage size overflow")?;
            let logical = scope
                .logical_bytes
                .checked_add(bytes)
                .filter(|n| *n <= MAX_LOGICAL_BYTES)
                .ok_or("Shared compiler call storage retention bound exceeded")?;
            analysis_charge(work)?;
            for _ in 0..bytes.div_ceil(64) {
                analysis_charge(work)?;
            }
            for _ in 0..len {
                analysis_charge(work)?;
            }
            scope.logical_bytes = logical; // cumulative, never refunded on failure/drop
            let mut storage = Vec::new();
            storage
                .try_reserve_exact(len)
                .map_err(|_| "Shared compiler call allocation denied")?;
            storage.extend_from_slice(values);
            storage.push(pair);
            Ok(Self::Prefix {
                storage: Rc::new(storage),
                len,
            })
        })();
        if result.is_err() {
            scope.failed = true;
        }
        result
    }
}
impl Calls for SharedCalls {
    type Scope = SharedScope;
    const SHARED: bool = true;
    fn scope(work: &mut usize) -> Result<SharedScope, String> {
        const {
            assert!(std::mem::size_of::<SharedScope>() <= 16);
        }
        analysis_charge(work)?; // selected invocation holder admission event
        analysis_charge(work)?; // fixed portable16 bytes, not target-sized output
        Ok(SharedScope {
            logical_bytes: 16,
            failed: false,
        })
    }
    fn empty() -> Self {
        Self::Empty
    }
    fn pushed(
        &mut self,
        pair: Pair,
        scope: &mut SharedScope,
        work: &mut usize,
    ) -> Result<(), String> {
        *self = Self::owned(self, pair, scope, work)?;
        Ok(())
    }
    fn popped(
        &mut self,
        scope: &mut SharedScope,
        _: &mut usize,
    ) -> Result<Option<(Pair, Self)>, String> {
        if scope.failed {
            return Err("Shared compiler call storage is poisoned".into());
        }
        // The canonical row owns this prefix. Move the Rc into the returned
        // prefix; no counter clone, pair copy or allocation occurs at return.
        let Self::Prefix { storage, len } = self else {
            return Ok(None);
        };
        let rest = len
            .checked_sub(1)
            .ok_or("Invalid shared compiler call prefix")?;
        let pair = *storage
            .get(rest)
            .ok_or("Invalid shared compiler call prefix")?;
        let owned = std::mem::replace(self, Self::Empty);
        let next = if rest == 0 {
            Self::Empty
        } else {
            let Self::Prefix { storage, .. } = owned else {
                return Err("Invalid shared compiler call prefix".into());
            };
            Self::Prefix { storage, len: rest }
        };
        Ok(Some((pair, next)))
    }
    // A configuration clone copies this fixed descriptor, not any pair slots.
    // The independent Rc-owned vector was prepaid at its actual Call boundary.
    fn copied_pairs(&self) -> usize {
        0
    }
    fn snapshot_copy_fee(&self, scope: &mut SharedScope, work: &mut usize) -> Result<(), String> {
        if scope.failed {
            return Err("Shared compiler call storage is poisoned".into());
        }
        // This selected pass inspects every row before the existing snapshot.
        // Pay that new inspection even for Empty; Prefix additionally performs
        // its fixed descriptor/Rc clone under the same per-row event.
        let result = analysis_charge(work);
        if result.is_err() {
            scope.failed = true;
        }
        result
    }
}

#[cfg(test)]
impl SharedScope {
    pub(super) fn at_retention_boundary_for_test(&mut self) {
        self.logical_bytes = MAX_LOGICAL_BYTES - 1;
    }
    pub(super) fn failed_for_test(&self) -> bool {
        self.failed
    }
}
