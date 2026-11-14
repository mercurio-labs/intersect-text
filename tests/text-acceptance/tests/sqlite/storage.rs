#![cfg(not(target_arch = "wasm32"))]
use super::Store;
use intersect_model::core::{
    frozen::FrozenStore,
    model::Element,
    storage::{self, ModelRead, ModelStore, Query},
};
use intersect_model::sqlite::SqliteStore;
use intersect_text::package::Package;
use intersect_text::{EvaluationLimits, Source, evaluate, evaluate_into};
use serde_json::json;

control_test! {
pub fn observed_sqlite_payloads_preflight_scalar_pages_and_whole_values_without_mutation() {
    use intersect_model::core::storage::InvocationContext;
    use intersect_model::definitions::conversion::{InvocationObserver, ProviderId};
    use std::{cell::Cell, rc::Rc};
    struct Budget {
        used: Cell<usize>,
        checks: Cell<usize>,
        limit: usize,
        stop: usize,
    }
    impl InvocationObserver for Budget {
        fn before_invoke(&self, _: &ProviderId) -> Result<(), String> {
            panic!("JSON provider")
        }
        fn checkpoint(&self) -> Result<(), String> {
            if self.checks.get() == self.stop {
                return Err("JSON cancelled".into());
            }
            self.checks.set(self.checks.get() + 1);
            Ok(())
        }
        fn retain_storage(&self, bytes: usize) -> Result<(), String> {
            let next = self
                .used
                .get()
                .checked_add(bytes)
                .filter(|n| *n <= self.limit)
                .ok_or("JSON storage denied")?;
            self.used.set(next);
            Ok(())
        }
    }
    fn budget(limit: usize, stop: usize) -> Rc<Budget> {
        Rc::new(Budget {
            used: Cell::new(0),
            checks: Cell::new(0),
            limit,
            stop,
        })
    }
    fn check<T: std::fmt::Debug + PartialEq>(
        bytes: usize,
        expected: T,
        read: impl Fn(&InvocationContext<'_>) -> Result<T, String>,
    ) {
        let baseline = budget(bytes, usize::MAX);
        assert_eq!(
            read(&InvocationContext::new(baseline.clone())).unwrap(),
            expected
        );
        assert_eq!(baseline.used.get(), bytes);
        for limit in [0, bytes - 1] {
            assert_eq!(
                read(&InvocationContext::new(budget(limit, usize::MAX))).unwrap_err(),
                "JSON storage denied"
            );
        }
        assert_eq!(
            read(&InvocationContext::new(budget(bytes + 1, usize::MAX))).unwrap(),
            expected
        );
        for stop in 0..baseline.checks.get() {
            assert_eq!(
                read(&InvocationContext::new(budget(usize::MAX, stop))).unwrap_err(),
                "JSON cancelled"
            );
        }
        assert_eq!(read(&InvocationContext::default()).unwrap(), expected);
    }
    let schema = package().schema;
    let mut store = SqliteStore::open(":memory:").unwrap();
    let frame = store.begin().unwrap();
    store.set_revision(&schema.revision).unwrap();
    for (id, class) in [("a", "Library"), ("é", "Member"), ("z", "Member")] {
        store.create_element(&schema, id, class, "r", 0, 0).unwrap();
        store.set_value(id, "name", Some(json!(id))).unwrap();
    }
    store.set_value("a", "name", Some(json!("é"))).unwrap();
    store
        .set_value("a", "members", Some(json!(["é", "z"])))
        .unwrap();
    storage::validate(&store, &schema).unwrap();
    store.commit(&frame).unwrap();
    let original = storage::export(&store).unwrap();
    // Scalar 132 plus two (64+4)-byte parser scratch bounds = 268.
    check(268, Some(json!("é")), |o| {
        store.value_observed("a", "name", o)
    });
    // Page slots add 32 each; "z" is 131 + 2*(64+3) = 265.
    check(300, vec![json!("é")], |o| {
        store.collection_page_observed("a", "members", 0, 1, o)
    });
    check(297, vec![json!("z")], |o| {
        store.collection_page_observed("a", "members", 1, 1, o)
    });
    check(597, vec![json!("é"), json!("z")], |o| {
        store.collection_page_observed("a", "members", 0, 2, o)
    });
    // An array root adds its 128-byte structural slot and two metadata bytes.
    check(727, Some(json!(["é", "z"])), |o| {
        store.value_observed("a", "members", o)
    });
    assert_eq!(
        store
            .value_observed("missing", "name", &InvocationContext::new(budget(0, 0)))
            .unwrap_err(),
        "JSON cancelled"
    );
    let empty = InvocationContext::new(budget(0, usize::MAX));
    assert_eq!(
        store.value_observed("missing", "name", &empty).unwrap(),
        None
    );
    assert!(
        store
            .collection_page_observed("a", "members", usize::MAX, 1, &empty)
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .collection_page_observed("a", "name", 0, 1, &InvocationContext::default())
            .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), original);
    let frame = store.begin().unwrap();
    store
        .set_value("a", "name", Some(json!("changed")))
        .unwrap();
    check(283, Some(json!("changed")), |o| {
        store.value_observed("a", "name", o)
    });
    store.rollback(&frame).unwrap();
    check(268, Some(json!("é")), |o| {
        store.value_observed("a", "name", o)
    });
    assert_eq!(storage::export(&store).unwrap(), original);
}
}

control_test! {
pub fn observed_sqlite_queries_guard_copies_and_recover_after_denial_and_rollback() {
    use intersect_model::core::storage::InvocationContext;
    use intersect_model::definitions::conversion::{InvocationObserver, ProviderId};
    use std::{cell::Cell, rc::Rc};
    struct Budget {
        used: Cell<usize>,
        limit: usize,
        cancel: bool,
    }
    impl InvocationObserver for Budget {
        fn before_invoke(&self, _: &ProviderId) -> Result<(), String> {
            panic!("query provider")
        }
        fn checkpoint(&self) -> Result<(), String> {
            if self.cancel {
                Err("query cancelled".into())
            } else {
                Ok(())
            }
        }
        fn retain_storage(&self, bytes: usize) -> Result<(), String> {
            let next = self
                .used
                .get()
                .checked_add(bytes)
                .filter(|n| *n <= self.limit)
                .ok_or("query storage denied")?;
            self.used.set(next);
            Ok(())
        }
    }
    let budget = |limit, cancel| {
        Rc::new(Budget {
            used: Cell::new(0),
            limit,
            cancel,
        })
    };
    let schema = package().schema;
    let mut store = SqliteStore::open(":memory:").unwrap();
    let frame = store.begin().unwrap();
    store.set_revision(&schema.revision).unwrap();
    for id in ["é", "z"] {
        store
            .create_element(&schema, id, "Library", "r", 0, 0)
            .unwrap();
    }
    store.commit(&frame).unwrap();
    let original = storage::export(&store).unwrap();
    // Two key slots 128 + "class" 5 + "Library" 7; first row 96 + UTF-8 2.
    let key_bytes = 140;
    for limit in [0, key_bytes + 97, key_bytes + 98, key_bytes + 99] {
        let observer = budget(limit, false);
        let result = store.query_candidates_observed(
            Query::Class("Library"),
            None,
            1,
            &InvocationContext::new(observer.clone()),
        );
        if limit < key_bytes + 98 {
            assert_eq!(result.unwrap_err(), "query storage denied");
        } else {
            let first = result.unwrap();
            assert_eq!(first.rows[0].1, "é");
            assert!(!first.exhausted);
            assert_eq!(observer.used.get(), 238);
            let second = store
                .query_candidates_observed(
                    Query::Class("Library"),
                    first.after,
                    1,
                    &InvocationContext::default(),
                )
                .unwrap();
            assert_eq!(second.rows[0].1, "z");
            assert!(second.exhausted);
        }
    }
    let observer = budget(98, false);
    assert_eq!(
        store
            .id_page_observed(None, 1, &InvocationContext::new(observer.clone()))
            .unwrap()[0]
            .1,
        "é"
    );
    assert_eq!(observer.used.get(), 98);
    assert_eq!(
        store
            .id_page_observed(None, 1, &InvocationContext::new(budget(97, false)))
            .unwrap_err(),
        "query storage denied"
    );
    let observer = budget(key_bytes, false);
    assert!(
        store
            .matches_observed(
                "é",
                Query::Class("Library"),
                &InvocationContext::new(observer.clone())
            )
            .unwrap()
    );
    assert_eq!(observer.used.get(), key_bytes);
    let cancelled = InvocationContext::new(budget(usize::MAX, true));
    assert_eq!(
        store
            .id_page_observed(Some(u64::MAX), 1, &cancelled)
            .unwrap_err(),
        "query cancelled"
    );
    assert_eq!(
        store
            .matches_observed("missing", Query::Class("Library"), &cancelled)
            .unwrap_err(),
        "query cancelled"
    );
    assert_eq!(
        store
            .query_candidates_observed(Query::Class("Library"), None, 1, &cancelled)
            .unwrap_err(),
        "query cancelled"
    );
    assert_eq!(storage::export(&store).unwrap(), original);
    let frame = store.begin().unwrap();
    store.remove("é").unwrap();
    assert_eq!(
        store
            .query_candidates_observed(
                Query::Class("Library"),
                None,
                1,
                &InvocationContext::default()
            )
            .unwrap()
            .rows[0]
            .1,
        "z"
    );
    store.rollback(&frame).unwrap();
    assert_eq!(storage::export(&store).unwrap(), original);
}
}

fn package() -> Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        include_str!("../../../../fixtures/library/library.xtext"),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap()
}

control_test! {
pub fn borrowed_class_callback_tracks_transaction_visibility_and_callback_errors() {
    let schema = package().schema;
    let mut store = SqliteStore::open(":memory:").unwrap();
    let revision = |store: &SqliteStore, expected: &str| {
        let mut calls = 0;
        store
            .inspect_revision(&mut |revision| {
                calls += 1;
                assert_eq!(revision, expected);
                Ok(())
            })
            .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(
            store
                .inspect_revision(&mut |_| Err("revision callback rejected".into()))
                .unwrap_err(),
            "revision callback rejected"
        );
    };
    revision(&store, "");
    let check = |store: &SqliteStore, expected: Option<&str>| {
        let mut calls = 0;
        assert_eq!(
            store
                .inspect_class("a", &mut |class| {
                    calls += 1;
                    assert_eq!(Some(class), expected);
                    Ok(())
                })
                .unwrap(),
            expected.is_some()
        );
        assert_eq!(calls, usize::from(expected.is_some()));
    };
    check(&store, None);
    let frame = store.begin().unwrap();
    store.set_revision(&schema.revision).unwrap();
    store
        .put(Element::new(&schema, "a", "Library", "r", 0, 0).unwrap())
        .unwrap();
    check(&store, Some("Library"));
    revision(&store, &schema.revision);
    assert_eq!(
        store
            .inspect_class("a", &mut |_| Err("callback rejected".into()))
            .unwrap_err(),
        "callback rejected"
    );
    check(&store, Some("Library"));
    store.commit(&frame).unwrap();
    let frame = store.begin().unwrap();
    store.set_revision("é revision").unwrap();
    revision(&store, "é revision");
    store.remove("a").unwrap();
    check(&store, None);
    store.rollback(&frame).unwrap();
    revision(&store, &schema.revision);
    check(&store, Some("Library"));
}
}

control_test! {
pub fn enum_aliases_and_exact_longs_persist_and_bad_values_roll_back_in_sqlite() {
    let text = "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='test' nsURI='urn:test'><eClassifiers xsi:type='e:EEnum' name='Mode'><eLiterals name='Primary' value='7'/><eLiterals name='Alias' value='7'/></eClassifiers><eClassifiers xsi:type='e:EClass' name='Item'><eStructuralFeatures xsi:type='e:EAttribute' name='mode' eType='#//Mode'/></eClassifiers></e:EPackage>";
    let text = text.replace("</eClassifiers></e:EPackage>", "<eStructuralFeatures xsi:type='e:EAttribute' name='longValue' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//ELong' defaultValueLiteral='9007199254740993'/></eClassifiers></e:EPackage>");
    let text = text.replace("</eClassifiers></e:EPackage>", "<eStructuralFeatures xsi:type='e:EAttribute' name='negativeZero' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EFloat' defaultValueLiteral='-0'/><eStructuralFeatures xsi:type='e:EAttribute' name='notANumber' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EDouble' defaultValueLiteral='NaN'/></eClassifiers></e:EPackage>");
    let text = text.replace("</eClassifiers></e:EPackage>", "<eStructuralFeatures xsi:type='e:EAttribute' name='declared' eType='#//Counter' defaultValueLiteral='9007199254740993'/></eClassifiers><eClassifiers xsi:type='e:EDataType' name='Counter' instanceClassName='long'/></e:EPackage>");
    let text = text.replace("name='notANumber'", "name='huge' defaultValueLiteral='184467440737095516160000000000000000000' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBigInteger'/><eStructuralFeatures xsi:type='e:EAttribute' name='notANumber'");
    let text = text.replace("name='notANumber'", "name='decimal' defaultValueLiteral='18446744073709551616.00' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBigDecimal'/><eStructuralFeatures xsi:type='e:EAttribute' name='notANumber'");
    let text = text.replace("name='notANumber'", "name='bytes' defaultValueLiteral='ABC' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EByteArray'/><eStructuralFeatures xsi:type='e:EAttribute' name='notANumber'");
    let text = text.replace("name='notANumber'", "name='character' defaultValueLiteral='55296' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EChar'/><eStructuralFeatures xsi:type='e:EAttribute' name='notANumber'");
    let text = text.replace("name='notANumber'", "name='boxedCharacter' defaultValueLiteral='65535' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//ECharacterObject'/><eStructuralFeatures xsi:type='e:EAttribute' name='notANumber'");
    let text = text.replace("name='notANumber'", "name='date' defaultValueLiteral='1969-12-31T23:59:59.999+0000' eType='e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EDate'/><eStructuralFeatures xsi:type='e:EAttribute' name='notANumber'");
    let schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(&text).unwrap();
    let long = intersect_model::definitions::scalars::Builtin::ELong
        .parse("9007199254740993")
        .unwrap();
    let alias = serde_json::to_value(schema.enumerations["Mode"].parse("Alias").unwrap()).unwrap();
    let path = std::env::temp_dir().join(format!(
        "intersect-enum-storage-{}.sqlite",
        std::process::id()
    ));
    let expected;
    {
        let mut store = SqliteStore::open(&path).unwrap();
        let checkpoint = store.begin().unwrap();
        storage::insert(
            &mut store,
            &schema,
            Element::new(&schema, "item", "Item", "api", 0, 0).unwrap(),
        )
        .unwrap();
        store.commit(&checkpoint).unwrap();
        storage::edit(
            &mut store,
            &schema,
            &[("item".into(), "mode".into(), alias.clone())],
        )
        .unwrap();
        expected = storage::export(&store).unwrap();
        assert!(
            storage::edit(
                &mut store,
                &schema,
                &[("item".into(), "declared".into(), long.clone())]
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
        assert!(
            storage::edit(
                &mut store,
                &schema,
                &[("item".into(), "mode".into(), json!(7))]
            )
            .is_err()
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        storage::validate(&store, &schema).unwrap();
        for (field, expected_value) in [
            ("date", json!({"kind":"date","milliseconds":"-1"})),
            ("bytes", json!({"kind":"bytes","hex":"AB0C"})),
            ("character", json!({"kind":"char16","unit":55296})),
            ("boxedCharacter", json!({"kind":"char16","unit":65535})),
        ] {
            assert_eq!(
                store.query(Query::Value(field, &expected_value)).unwrap(),
                ["item"]
            );
            assert_eq!(
                store.get("item").unwrap().unwrap().values[field],
                expected_value
            );
        }

        let decimal = intersect_model::definitions::scalars::Builtin::EBigDecimal
            .parse("18446744073709551616.00")
            .unwrap();
        assert_eq!(
            store.query(Query::Value("decimal", &decimal)).unwrap(),
            ["item"]
        );
        assert_eq!(
            store.get("item").unwrap().unwrap().values["decimal"],
            decimal
        );
        let other_scale = intersect_model::definitions::scalars::Builtin::EBigDecimal
            .parse("18446744073709551616.0")
            .unwrap();
        assert!(
            store
                .query(Query::Value("decimal", &other_scale))
                .unwrap()
                .is_empty()
        );
        let huge = intersect_model::definitions::scalars::Builtin::EBigInteger
            .parse("184467440737095516160000000000000000000")
            .unwrap();
        assert_eq!(store.query(Query::Value("huge", &huge)).unwrap(), ["item"]);
        assert_eq!(store.get("item").unwrap().unwrap().values["huge"], huge);
        let declared = serde_json::to_value(
            schema.datatypes["Counter"]
                .parse("9007199254740993")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            store.query(Query::Value("declared", &declared)).unwrap(),
            ["item"]
        );
        assert_eq!(
            store.get("item").unwrap().unwrap().values["declared"],
            declared
        );
        assert_eq!(storage::export(&store).unwrap(), expected);
        assert_eq!(store.query(Query::Value("mode", &alias)).unwrap(), ["item"]);
        assert_eq!(
            store.query(Query::Value("longValue", &long)).unwrap(),
            ["item"]
        );
        assert_eq!(
            store.get("item").unwrap().unwrap().values["longValue"],
            long
        );
        let negative = intersect_model::definitions::scalars::Builtin::EFloat
            .parse("-0")
            .unwrap();
        assert_eq!(
            store
                .query(Query::Value("negativeZero", &negative))
                .unwrap(),
            ["item"]
        );
        assert_eq!(
            store.get("item").unwrap().unwrap().values["notANumber"],
            intersect_model::definitions::scalars::Builtin::EDouble
                .parse("NaN")
                .unwrap()
        );
    }
    std::fs::remove_file(path).unwrap();
}
}
fn sources() -> Vec<Source> {
    vec![
        Source {
            uri: "catalog.library".into(),
            text: include_str!("../../../../fixtures/library/catalog.library").into(),
        },
        Source {
            uri: "loans.library".into(),
            text: include_str!("../../../../fixtures/library/loans.library").into(),
        },
    ]
}
fn exercise(store: &mut impl ModelStore) {
    let p = package();
    let expected = evaluate(&p, &sources());
    let parsed = evaluate_into(&p, &sources(), EvaluationLimits::native(), store).unwrap();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(storage::export(store).unwrap(), expected.model);
    assert_eq!(parsed.derived, expected.derived);
    assert_eq!(
        serde_json::to_value(parsed.references).unwrap(),
        serde_json::to_value(expected.references).unwrap()
    );
    assert_eq!(
        store.query(Query::Value("name", &json!("dune"))).unwrap(),
        vec!["catalog.library#1"]
    );
    assert_eq!(
        store
            .query(Query::Value("copies", &json!("catalog.library#1")))
            .unwrap(),
        vec!["catalog.library#0"]
    );
    assert_eq!(
        store.query(Query::Class("Loan")).unwrap(),
        vec!["loans.library#1"]
    );
    assert_eq!(
        store.query(Query::Resource("loans.library")).unwrap(),
        vec!["loans.library#0", "loans.library#1"]
    );
    assert!(
        store
            .matches(
                "catalog.library#0",
                Query::Value("copies", &json!("catalog.library#1"))
            )
            .unwrap()
    );
    assert!(
        !store
            .matches(
                "loans.library#0",
                Query::Value("copies", &json!("catalog.library#1"))
            )
            .unwrap()
    );
    assert!(!store.matches("missing", Query::Class("Loan")).unwrap());
    storage::validate(store, &p.schema).unwrap();
    let before = storage::export(store).unwrap();
    for edits in [
        vec![("catalog.library#1".into(), "edition".into(), json!("bad"))],
        vec![("loans.library#1".into(), "copy".into(), json!("missing"))],
        vec![("catalog.library#1".into(), "available".into(), json!(true))],
        vec![(
            "loans.library#0".into(),
            "copies".into(),
            json!(["catalog.library#1", "missing"]),
        )],
    ] {
        assert!(storage::edit(store, &p.schema, &edits).is_err());
        assert_eq!(storage::export(store).unwrap(), before);
    }
    let edits = vec![
        (
            "catalog.library#0".into(),
            "copies".into(),
            json!(["catalog.library#2"]),
        ),
        (
            "loans.library#0".into(),
            "copies".into(),
            json!(["catalog.library#1"]),
        ),
    ];
    let mut api = before.clone();
    api.edit(&p.schema, &edits).unwrap();
    storage::edit(store, &p.schema, &edits).unwrap();
    assert_eq!(storage::export(store).unwrap(), api);
    assert_eq!(
        store
            .query(Query::Value("copies", &json!("catalog.library#1")))
            .unwrap(),
        vec!["loans.library#0"]
    );
    assert_eq!(
        store.get("catalog.library#1").unwrap().unwrap().values["library"],
        json!("loans.library#0")
    );
    for text in [
        "library Broken { member",
        "library Broken { loan missing copy unknown to nobody; }",
        "library Broken { loan second copy dune to ada; }",
    ] {
        let mut input = sources();
        input.push(Source {
            uri: "broken.library".into(),
            text: text.into(),
        });
        let result = evaluate_into(&p, &input, EvaluationLimits::native(), store).unwrap();
        assert!(!result.diagnostics.is_empty());
        let legacy = evaluate(&p, &input);
        assert_eq!(result.diagnostics, legacy.diagnostics);
        assert_eq!(storage::export(store).unwrap(), api);
    }
    let mut other = p.clone();
    other.schema.revision = "different".into();
    assert!(evaluate_into(&other, &sources(), EvaluationLimits::native(), store).is_err());
    assert_eq!(storage::export(store).unwrap(), api);
    let outer = store.begin().unwrap();
    let inner = store.begin().unwrap();
    assert!(store.commit(&outer).is_err());
    storage::edit(
        store,
        &p.schema,
        &[("catalog.library#1".into(), "name".into(), json!("changed"))],
    )
    .unwrap();
    store.commit(&inner).unwrap();
    assert_eq!(
        store
            .query(Query::Value("name", &json!("changed")))
            .unwrap(),
        vec!["catalog.library#1"]
    );
    store.rollback(&outer).unwrap();
    assert_eq!(storage::export(store).unwrap(), api);
    assert!(
        store
            .query(Query::Value("name", &json!("changed")))
            .unwrap()
            .is_empty()
    );
    assert!(store.rollback(&outer).is_err());
    let token = store.begin().unwrap();
    store.remove("catalog.library#1").unwrap();
    assert!(store.get("catalog.library#1").unwrap().is_none());
    assert!(
        store
            .query(Query::Value("name", &json!("dune")))
            .unwrap()
            .is_empty()
    );
    store.rollback(&token).unwrap();
    assert_eq!(storage::export(store).unwrap(), api);
    assert!(store.remove("catalog.library#1").is_err());
    // A delete that leaves a dangling reference is rejected at validation.
    assert!(
        storage::atomic(store, |s| {
            s.remove("catalog.library#1")?;
            storage::validate(s, &p.schema)
        })
        .is_err()
    );
    assert_eq!(storage::export(store).unwrap(), api);
    let token = store.begin().unwrap();
    let new = Element::new(&p.schema, "new", "Member", "api", 4, 9).unwrap();
    storage::insert(store, &p.schema, new.clone()).unwrap();
    assert!(storage::insert(store, &p.schema, new).is_err());
    store.rollback(&token).unwrap();
    assert_eq!(storage::export(store).unwrap(), api);
    // Empty replacement is persisted with an explicit schema revision.
    assert!(
        evaluate_into(&p, &[], EvaluationLimits::native(), store)
            .unwrap()
            .diagnostics
            .is_empty()
    );
    assert!(store.page(None, 1).unwrap().is_empty());
    assert_eq!(store.revision().unwrap(), p.schema.revision);
}
store_test! {
pub fn memory_and_sqlite_share_parsing_editing_validation_and_rollback() {
    exercise(&mut crate::stores::store::<Store>(&package().schema).unwrap());
    exercise(&mut SqliteStore::in_memory().unwrap());
}
}

control_test! {
pub fn reopen_preserves_identity_provenance_and_discards_uncommitted_changes() {
    let path =
        std::env::temp_dir().join(format!("intersect-storage-{}.sqlite", std::process::id()));
    let p = package();
    let expected;
    {
        let mut store = SqliteStore::open(&path).unwrap();
        evaluate_into(&p, &sources(), EvaluationLimits::native(), &mut store).unwrap();
        expected = storage::export(&store).unwrap();
        store.begin().unwrap();
        store.remove("catalog.library#1").unwrap();
    }
    {
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(storage::export(&store).unwrap(), expected);
        storage::validate(&store, &p.schema).unwrap();
        assert_eq!(
            store.query(Query::Value("name", &json!("dune"))).unwrap(),
            vec!["catalog.library#1"]
        );
    }
    std::fs::remove_file(path).unwrap();
}
}
control_test! {
pub fn pages_and_rollback_restore_order_across_deletes() {
    fn check(store: &mut impl ModelStore) {
        let p = package();
        storage::atomic(store, |s| {
            for n in 0..260 {
                let mut e =
                    Element::new(&p.schema, &format!("member-{n}"), "Member", "api", n, n + 1)?;
                e.values.insert("name".into(), json!(format!("m{n}")));
                storage::insert(s, &p.schema, e)?;
            }
            storage::validate(s, &p.schema)
        })
        .unwrap();
        let before = storage::export(store).unwrap();
        let checkpoint = store.begin().unwrap();
        store.remove("member-0").unwrap();
        let mut e = before.elements[0].clone();
        e.values.insert("name".into(), json!("replaced"));
        store.put(e).unwrap();
        assert_eq!(
            storage::export(store).unwrap().elements.last().unwrap().id,
            "member-0"
        );
        store.rollback(&checkpoint).unwrap();
        assert_eq!(storage::export(store).unwrap(), before);
        assert!(store.page(None, 0).unwrap().is_empty());
        assert!(store.page(Some(u64::MAX), 1).unwrap().is_empty());
    }
    check(&mut FrozenStore::for_schema(&package().schema).unwrap());
    check(&mut SqliteStore::in_memory().unwrap());
}
}
control_test! {
pub fn unwinding_rolls_back_outer_and_committed_inner_changes() {
    fn check(store: &mut impl ModelStore) {
        let p = package();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<(), String> = storage::atomic(store, |s| {
                storage::atomic(s, |s| {
                    storage::insert(
                        s,
                        &p.schema,
                        Element::new(&p.schema, "a", "Member", "api", 0, 0)?,
                    )
                })?;
                panic!("host callback failed")
            });
        }));
        assert!(panic.is_err());
        assert!(store.page(None, 1).unwrap().is_empty());
        assert_eq!(store.revision().unwrap(), "");
    }
    check(&mut FrozenStore::for_schema(&package().schema).unwrap());
    check(&mut SqliteStore::in_memory().unwrap());
}
}
control_test! {
pub fn sqlite_failed_index_write_restores_payload_and_outer_transaction() {
    let path = std::env::temp_dir().join(format!(
        "intersect-storage-fault-{}.sqlite",
        std::process::id()
    ));
    let p = package();
    {
        let mut store = SqliteStore::open(&path).unwrap();
        evaluate_into(&p, &sources(), EvaluationLimits::native(), &mut store).unwrap();
        let before = storage::export(&store).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TRIGGER IF NOT EXISTS fail_index BEFORE INSERT ON intersect_indexes WHEN NEW.value='\"fail\"' BEGIN SELECT RAISE(ABORT,'injected index failure'); END;").unwrap();
        let token = store.begin().unwrap();
        let mut element = store.get("catalog.library#1").unwrap().unwrap();
        element.values.insert("name".into(), json!("fail"));
        assert!(
            store
                .put(element)
                .unwrap_err()
                .contains("injected index failure")
        );
        assert_eq!(storage::export(&store).unwrap(), before);
        assert_eq!(
            store.query(Query::Value("name", &json!("dune"))).unwrap(),
            vec!["catalog.library#1"]
        );
        store.commit(&token).unwrap();
        assert_eq!(storage::export(&store).unwrap(), before);
    }
    std::fs::remove_file(path).unwrap();
}
}

store_test! {
pub fn schema_validation_negatives_agree_between_adapters() {
    let p = package();
    let original = evaluate(&p, &sources()).model;
    for case in 0..8 {
        let mut broken = original.clone();
        match case {
            0 => broken.elements[1].type_identity = "foreign#BookCopy".into(),
            1 => {
                broken.elements[1]
                    .values
                    .insert("edition".into(), json!(2147483648i64));
            }
            2 => {
                broken.elements[0].values.insert(
                    "copies".into(),
                    json!(["catalog.library#1", "catalog.library#1"]),
                );
            }
            3 => {
                broken.elements[4]
                    .values
                    .insert("members".into(), json!(["catalog.library#3"]));
            }
            4 => {
                broken.elements[5]
                    .values
                    .insert("copy".into(), json!("catalog.library#3"));
            }
            5 => {
                broken.elements[5].values.insert("copy".into(), json!(null));
            }
            6 => {
                broken.elements[1]
                    .values
                    .insert("library".into(), json!(null));
            }
            _ => {
                broken.elements[1]
                    .values
                    .insert("extra".into(), json!(true));
            }
        }

        let mut sqlite = SqliteStore::in_memory().unwrap();
        storage::atomic(&mut sqlite, |s| {
            s.set_revision(&broken.schema_revision)?;
            for element in &broken.elements {
                s.put(element.clone())?;
            }
            Ok(())
        })
        .unwrap();
        let wire_error = broken.validate(&p.schema).unwrap_err();
        assert_eq!(storage::validate(&sqlite, &p.schema).unwrap_err(), wire_error);
        if let Ok(frozen) = crate::stores::from_model::<Store>(&p.schema, &broken) {
            assert_eq!(storage::validate(&frozen, &p.schema).unwrap_err(), wire_error);
        }
    }
}
}

store_test! {
pub fn speculative_parser_failures_do_not_reach_either_store() {
    let p = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        include_str!("../../../../fixtures/library/library.ecore"),
        &include_str!("../../../../fixtures/library/library.xtext").replace(
            "'library' name=ID",
            "'library' ('optional'? 'marker'?) name=ID",
        ),
        include_str!("../../../../fixtures/library/library-behavior.rcore"),
    )
    .unwrap();
    let mut memory = crate::stores::store::<Store>(&p.schema).unwrap();
    let mut sqlite = SqliteStore::in_memory().unwrap();
    let a = evaluate_into(&p, &sources(), EvaluationLimits::native(), &mut memory).unwrap();
    let b = evaluate_into(&p, &sources(), EvaluationLimits::native(), &mut sqlite).unwrap();
    assert!(a.diagnostics.is_empty());
    assert!(b.diagnostics.is_empty());
    assert_eq!(
        semantic_model(storage::export(&memory).unwrap()),
        semantic_model(storage::export(&sqlite).unwrap())
    );
    let before_memory = storage::export(&memory).unwrap();
    let before = storage::export(&sqlite).unwrap();
    let limited = EvaluationLimits {
        elements_per_resource: Some(1),
        ..EvaluationLimits::native()
    };
    let a = evaluate_into(&p, &sources(), limited, &mut memory).unwrap();
    let b = evaluate_into(&p, &sources(), limited, &mut sqlite).unwrap();
    assert!(!a.diagnostics.is_empty());
    assert_eq!(a.diagnostics, b.diagnostics);
    assert_eq!(storage::export(&memory).unwrap(), before_memory);
    assert_eq!(storage::export(&sqlite).unwrap(), before);
}
}

// Resource content revisions are adapter-local deterministic tokens, not a
// portable edit counter. Preserve every other field in semantic comparisons.
fn semantic_model(
    mut model: intersect_model::core::model::Model,
) -> intersect_model::core::model::Model {
    for record in model.resources.values_mut() {
        record.content_revision = 0;
    }
    model
}
