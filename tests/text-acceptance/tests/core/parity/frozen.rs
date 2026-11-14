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

#[path = "../changeable.rs"]
pub mod changeable;

#[path = "../computed_state.rs"]
pub mod computed_state;

#[path = "../creation_atomicity.rs"]
pub mod creation_atomicity;

#[path = "../reference_identity.rs"]
pub mod reference_identity;

#[path = "../storage_checkpoints.rs"]
pub mod storage_checkpoints;

#[path = "../unsettable_opposites.rs"]
pub mod unsettable_opposites;

#[path = "../is_set.rs"]
pub mod is_set;

#[path = "../feature_value_state.rs"]
pub mod feature_value_state;

#[path = "../prepared_validation.rs"]
pub mod prepared_validation;
