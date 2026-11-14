#![cfg(not(target_arch = "wasm32"))]
use super::Store;
use intersect_model::core::storage::ModelStore;
use intersect_model::sqlite::SqliteStore;

store_test! {
pub fn checkpoint_scope_is_enforced_between_memory_and_sqlite_instances() {
    let mut stores: Vec<Box<dyn ModelStore>> = vec![
        Box::new(crate::stores::store::<Store>(&crate::stores::empty_schema().unwrap()).unwrap()),
        Box::new(crate::stores::store::<Store>(&crate::stores::empty_schema().unwrap()).unwrap()),
        Box::new(SqliteStore::in_memory().unwrap()),
        Box::new(SqliteStore::in_memory().unwrap()),
    ];
    let tokens: Vec<_> = stores
        .iter_mut()
        .map(|store| {
            let token = store.begin().unwrap();
            store.set_revision("uncommitted").unwrap();
            token
        })
        .collect();
    for (owner, store) in stores.iter_mut().enumerate() {
        for (issuer, token) in tokens.iter().enumerate() {
            if owner != issuer {
                assert!(store.commit(token).is_err());
                assert!(store.rollback(token).is_err());
                assert!(store.is_transaction_active());
                assert_eq!(store.revision().unwrap(), "uncommitted");
            }
        }
        let child = store.begin().unwrap();
        store.set_revision("child").unwrap();
        assert!(store.commit(&tokens[owner]).is_err());
        assert!(store.rollback(&tokens[owner]).is_err());
        assert_eq!(store.revision().unwrap(), "child");
        store.commit(&child).unwrap();
        assert!(store.commit(&child).is_err());
        assert!(store.rollback(&child.clone()).is_err());
        store.rollback(&tokens[owner]).unwrap();
        assert!(store.commit(&tokens[owner]).is_err());
        assert!(store.rollback(&tokens[owner]).is_err());
        assert_eq!(store.revision().unwrap(), "");
        assert!(!store.is_transaction_active());
    }
}
}
