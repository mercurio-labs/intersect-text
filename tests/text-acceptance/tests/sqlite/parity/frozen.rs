//! The same case bodies with the frozen store selected at construction.
pub type Store = intersect_model::core::frozen::FrozenStore;
macro_rules! store_test {
    ($case:item) => {
        #[test]
        $case
    };
}
macro_rules! control_test {
    ($case:item) => {
        #[test]
        $case
    };
}

#[path = "../checkpoints.rs"]
pub mod checkpoints;

#[path = "../conversion.rs"]
pub mod conversion;

#[path = "../query.rs"]
pub mod query;

#[path = "../storage.rs"]
pub mod storage;
