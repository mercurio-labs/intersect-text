use super::*;

#[test]
fn evaluation_indexes_declared_query_dependencies_and_opposite_endpoints() {
    let ecore = include_str!("../assets/fixtures/library/library.ecore")
        .replace("copies", "inventory")
        .replace("BookCopy/library", "BookCopy/catalog")
        .replace("name=\"library\" eType", "name=\"catalog\" eType");
    let grammar =
        include_str!("../assets/fixtures/library/library.xtext").replace("copies", "inventory");
    // Count reads a collection directly; it must not cause every contained
    // member to receive a secondary index entry. Incoming validation does query.
    let semantics = include_str!("../assets/fixtures/library/library-behavior.rcore")
        .replace("count(`copies`)", "count(`members`)")
        .replace("self.`copy`, \"copy\"", "self.`member`, \"member\"");
    let package = compile_migrated_fixture(&ecore, &grammar, &semantics).unwrap();
    assert_eq!(
        evaluation_index_policy(&package),
        storage::IndexPolicy::selected(["name", "copy", "member", "inventory", "catalog"])
    );
}
