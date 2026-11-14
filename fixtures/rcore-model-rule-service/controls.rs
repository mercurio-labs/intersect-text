//! Literal prepared-rule service fixture; not a new source-activation route.
use intersect_model::definitions::schema::Package;
use intersect_model::{
    core::{
        model::Model,
        query::{DerivedValues, QueryBudget},
        storage::ModelRead,
    },
    definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        compiler::compile_structure_bundle,
        schema::{Derivation, Diagnostic, Expression, Semantics, Validation},
    },
    query::PackageDerived,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap};

pub type Derived = BTreeMap<String, BTreeMap<String, Value>>;
const STRUCTURE: &str = r#"rcore 1; package rules version "1";
metamodel "urn:prepared:model-rules" prefix rules {
 class Library { ref copies: Copy [0,*] containment; attr copyCount: Int32 [1,1] derived; }
 class Copy { attr available: Bool [1,1] derived; }
 class SpecialCopy extends Copy {}
 class Loan { ref copies: Copy [1,*] nonunique; }
 class SpecialLoan extends Loan {}
}"#;

pub fn fixture() -> (Package, Model) {
    let source = DefinitionSource::new("memory:prepared-rules.rcore", SourceKind::Rcore, STRUCTURE);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let schema = compile_structure_bundle(&bundle, &source.id, "urn:prepared:model-rules").unwrap();
    // These literal prepared rules deliberately avoid the grammar-gated rule
    // frontend. Separate existing source-compiler tests retain that authority.
    let package = Package {
        version: 3,
        schema,
        semantics: Semantics {
            scope_bindings: vec![],
            rule_conversions: vec![],
            version: 1,
            scope: "unique_names".into(),
            derived: vec![
                Derivation {
                    class: "Library".into(),
                    feature: "copyCount".into(),
                    expression: Expression::Count {
                        feature: "copies".into(),
                    },
                },
                Derivation {
                    class: "Copy".into(),
                    feature: "available".into(),
                    expression: Expression::NoIncoming {
                        class: "Loan".into(),
                        feature: "copies".into(),
                    },
                },
            ],
            initializers: vec![],
            validations: vec![
                Validation {
                    class: "Loan".into(),
                    feature: "copies".into(),
                    max_incoming: 1,
                    message: "first-rule".into(),
                },
                Validation {
                    class: "Loan".into(),
                    feature: "copies".into(),
                    max_incoming: 2,
                    message: "second-rule".into(),
                },
            ],
        },
        sources: BTreeMap::from([("rcore".into(), STRUCTURE.into())]),
        compilation: None,
    };
    let mut model = Model::default();
    for (id, class, uri, start, end) in [
        ("library", "Library", "memory:catalog", 0, 1),
        ("copy-a", "SpecialCopy", "memory:catalog", 2, 3),
        ("copy-b", "Copy", "memory:catalog", 4, 5),
        ("loan-a", "Loan", "memory:loans", 10, 15),
        ("loan-b", "SpecialLoan", "memory:loans", 20, 25),
    ] {
        model
            .create(&package.schema, id, class, uri, start, end)
            .unwrap();
    }
    // Checked edits keep containment and logical resource roots consistent.
    model
        .edit(
            &package.schema,
            &[
                (
                    "library".into(),
                    "copies".into(),
                    json!(["copy-a", "copy-b"]),
                ),
                (
                    "loan-a".into(),
                    "copies".into(),
                    json!(["copy-a", "copy-a"]),
                ),
                ("loan-b".into(), "copies".into(), json!(["copy-a"])),
            ],
        )
        .unwrap();
    model.validate(&package.schema).unwrap();
    intersect_model::resources::validate_records(
        &intersect_model::storage::ModelView::new(&model).unwrap(),
        &package.schema,
    )
    .unwrap();
    (package, model)
}

pub fn seed() -> (Derived, Vec<Diagnostic>) {
    (
        BTreeMap::from([(
            "stale".into(),
            BTreeMap::from([("value".into(), json!(999))]),
        )]),
        vec![Diagnostic::new(
            "memory:previous",
            7,
            8,
            "retained diagnostic",
        )],
    )
}

pub struct Recording<'a> {
    package: &'a Package,
    reject_copy_b: bool,
    pub calls: RefCell<Vec<(String, String, usize, usize)>>,
}
impl<'a> Recording<'a> {
    pub fn new(package: &'a Package, reject_copy_b: bool) -> Self {
        Self {
            package,
            reject_copy_b,
            calls: RefCell::new(vec![]),
        }
    }
}
impl DerivedValues for Recording<'_> {
    fn supports(&self, class: &str, feature: &str) -> bool {
        PackageDerived(self.package).supports(class, feature)
    }
    fn read(
        &self,
        store: &dyn ModelRead,
        id: &str,
        feature: &str,
        budget: &mut QueryBudget,
    ) -> Result<Value, String> {
        let before = budget.reads;
        let result = if self.reject_copy_b && id == "copy-b" {
            Err("selected provider rejected copy-b".into())
        } else {
            PackageDerived(self.package).read(store, id, feature, budget)
        };
        self.calls
            .borrow_mut()
            .push((id.into(), feature.into(), before, budget.reads));
        result
    }
}
pub fn snapshot(
    outcome: Result<(), String>,
    values: &Derived,
    diagnostics: &[Diagnostic],
    provider: &Recording<'_>,
) -> Value {
    let outcome = match outcome {
        Ok(()) => json!({"ok":true}),
        Err(error) => json!({"error":error}),
    };
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.origin.is_none())
    );
    json!({"outcome":outcome,"derived":values,"diagnostics":diagnostics,"calls":*provider.calls.borrow()})
}
pub fn expected(rejected: bool) -> Value {
    let mut diagnostics =
        vec![json!({"source":"memory:previous","start":7,"end":8,"message":"retained diagnostic"})];
    if !rejected {
        for message in ["first-rule", "second-rule"] {
            for (start, end) in [(10, 15), (10, 15), (20, 25)] {
                diagnostics.push(
                    json!({"source":"memory:loans","start":start,"end":end,"message":message}),
                );
            }
        }
    }
    let mut derived = json!({"library":{"copyCount":2},"copy-a":{"available":false}});
    if !rejected {
        derived
            .as_object_mut()
            .unwrap()
            .insert("copy-b".into(), json!({"available":true}));
    }
    json!({
        "outcome":if rejected {json!({"error":"selected provider rejected copy-b"})} else {json!({"ok":true})},
        "derived":derived,"diagnostics":diagnostics,
        "calls":[["library","copyCount",0,2],["copy-a","available",2,6],["copy-b","available",6,if rejected {6} else {9}]],
    })
}
