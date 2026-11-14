//! The same case bodies with the frozen store selected at construction.
pub type Store = intersect_model::core::frozen::FrozenStore;
macro_rules! control_module {
    ($module:item) => {
        $module
    };
}
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

#[path = "../interchange.rs"]
pub mod interchange;

#[path = "../structural.rs"]
pub mod structural;
