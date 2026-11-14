//! Storage only. Nullability, visit order and validation stay in the one worker.
use super::Summary;
use std::collections::BTreeSet;

pub(in crate::grammar) trait Symbols: Clone + Default {
    const WORKER_FRAME_BYTES: usize;
    const MEMOIZE_ONE_STEP: bool;
    fn terminal(make: impl FnOnce() -> String) -> Self;
    fn append(&mut self, other: Self);
    fn each(&self, visit: impl FnMut(&str) -> Result<(), String>) -> Result<(), String>;
    fn symbol_count(&self) -> usize;
    fn full(summary: &Summary<Self>) -> Option<&Summary>;
}
impl Symbols for BTreeSet<String> {
    const WORKER_FRAME_BYTES: usize = 128;
    const MEMOIZE_ONE_STEP: bool = true;
    fn terminal(make: impl FnOnce() -> String) -> Self {
        [make()].into()
    }
    fn append(&mut self, other: Self) {
        self.extend(other);
    }
    fn each(&self, mut visit: impl FnMut(&str) -> Result<(), String>) -> Result<(), String> {
        self.iter().try_for_each(|value| visit(value))
    }
    fn symbol_count(&self) -> usize {
        BTreeSet::len(self)
    }
    fn full(summary: &Summary<Self>) -> Option<&Summary> {
        Some(summary)
    }
}
impl Symbols for () {
    const WORKER_FRAME_BYTES: usize = 64;
    const MEMOIZE_ONE_STEP: bool = false;
    fn terminal(_: impl FnOnce() -> String) -> Self {}
    fn append(&mut self, _: Self) {}
    fn each(&self, _: impl FnMut(&str) -> Result<(), String>) -> Result<(), String> {
        Ok(())
    }
    fn symbol_count(&self) -> usize {
        0
    }
    fn full(_: &Summary<Self>) -> Option<&Summary> {
        None
    }
}
