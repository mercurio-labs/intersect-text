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

#[path = "../direct_store.rs"]
pub mod direct_store;

#[path = "../runtime_lower_text.rs"]
pub mod runtime_lower_text;

#[path = "../unsettable_text.rs"]
pub mod unsettable_text;

#[path = "../eobject_grammar.rs"]
pub mod eobject_grammar;

#[path = "../xtext_actions.rs"]
pub mod xtext_actions;

#[path = "../xtext_initialization.rs"]
pub mod xtext_initialization;

#[path = "../query.rs"]
pub mod query;

#[path = "../rcore_behavior_navigation_assignment.rs"]
pub mod rcore_behavior_navigation_assignment;
