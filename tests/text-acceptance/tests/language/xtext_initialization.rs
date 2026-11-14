//! Regression controls for text construction before the linking phase.
use super::Store;
use intersect_text::grammar::Profile;
use intersect_text::{Source, evaluate};

const ECORE: &str = include_str!("../../../../fixtures/xtext/simple-actions/model.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/xtext/simple-actions/model.xtext");
const RCORE_BEHAVIOR: &str =
    include_str!("../../../../fixtures/xtext/simple-actions/model-behavior.rcore");

fn concrete_grammar() -> String {
    XTEXT
        .replace("{k::Model} ", "")
        .replace("returns k::Element: {k::Type}", "returns k::Type:")
        .replace("returns k::Element: {k::Feature}", "returns k::Feature:")
}

control_test! {
pub fn text_initializers_preserve_pending_references_and_authored_values() {
    let grammar = concrete_grammar();
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        &grammar,
        RCORE_BEHAVIOR,
        Profile::NativeV1,
    )
    .unwrap();
    // A constructor after a pending reference and a speculative constructor at
    // the closing brace must both work without dropping initialization checks.
    let source = Source {
        uri: "constructors.kerml".into(),
        text: "package Demo { type Wheel origin \"authored\"; feature wheel : Wheel; type Spare; }"
            .into(),
    };
    let result = evaluate(&package, std::slice::from_ref(&source));
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.model.elements.len(), 4);
    for (index, class) in [(0, "Model"), (1, "Type"), (2, "Feature"), (3, "Type")] {
        let element = &result.model.elements[index];
        assert_eq!(element.id, format!("constructors.kerml#{index}"));
        assert_eq!(element.class, class);
    }
    assert_eq!(result.model.elements[1].values["origin"], "authored");
    assert_eq!(result.model.elements[3].values["origin"], "constructor");
    for index in [1, 3] {
        let element = &result.model.elements[index];
        assert_eq!(element.initialization.as_ref().unwrap().object, element.id);
    }
    assert_eq!(
        result.model.elements[2].values["target"],
        "constructors.kerml#1"
    );
    assert_eq!(result.references.len(), 1);
    let reference = &result.references[0];
    assert_eq!(&source.text[reference.start..reference.end], "Wheel");
    assert_eq!(reference.target, "constructors.kerml#1");
    result.model.validate(&package.schema).unwrap();
}
}

control_test! {
pub fn unresolved_text_targets_and_late_syntax_still_reject() {
    let grammar = concrete_grammar();
    let package = intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_profile(
        ECORE,
        &grammar,
        RCORE_BEHAVIOR,
        Profile::NativeV1,
    )
    .unwrap();
    for text in [
        "package Demo { feature wheel : Missing; type Spare; }",
        "package Demo { type Wheel; feature wheel : Wheel; type Spare origin ; }",
    ] {
        let result = evaluate(
            &package,
            &[Source {
                uri: "rejected.kerml".into(),
                text: text.into(),
            }],
        );
        assert!(!result.diagnostics.is_empty(), "{text}");
        assert!(
            result
                .diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.message.contains("Reference must be an identity")),
            "construction must reach the intended rejection: {:?}",
            result.diagnostics
        );
        if text.contains("Missing") {
            assert!(
                result
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains("Unresolved")),
                "{:?}",
                result.diagnostics
            );
        } else {
            assert!(
                result.model.elements.is_empty(),
                "late syntax must roll back the resource"
            );
        }
    }
}
}
use intersect_model::core::{
    query::QueryBudget,
    storage::{
        self, ClassInitializer, Construction, DraftReferencePolicy, InitialValue,
        InitializationContext, ModelRead, ModelStore,
    },
};
use intersect_model::definitions::{
    bundle::{DefinitionSource, SourceKind},
    conversion::ProviderId,
    initialization::ClassInitializationContract,
};
use serde_json::json;

struct EmptyInitializer(ClassInitializationContract);
impl ClassInitializer for EmptyInitializer {
    fn contract(&self) -> &ClassInitializationContract {
        &self.0
    }
    fn plan(
        &self,
        _: &dyn ModelRead,
        _: &str,
        budget: &mut QueryBudget,
    ) -> Result<Vec<InitialValue>, String> {
        budget.charge()?;
        Ok(Vec::new())
    }
}

store_test! {
pub fn provisional_construction_relaxes_only_canonical_pending_noncontainment_references() {
    provisional_reference_cases(&[
        "pending",
        "pending-collision",
        "containment",
        "missing",
        "wrong-class",
        "attribute",
    ]);
}
}

store_test! {
fn provisional_construction_rejects_malformed_raw_reference() {
    provisional_reference_cases(&["json"]);
}
}

fn provisional_reference_cases(modes: &[&str]) {
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="draft" nsURI="urn:xtext:construction-draft">
<eClassifiers x:type="e:EClass" name="Item"/>
<eClassifiers x:type="e:EClass" name="Holder">
<eStructuralFeatures x:type="e:EReference" name="target" eType="#//Item"/>
<eStructuralFeatures x:type="e:EReference" name="child" containment="true" eType="#//Item"/>
<eStructuralFeatures x:type="e:EAttribute" name="count" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//EInt"/>
</eClassifiers></e:EPackage>"##).unwrap();
    let sources = vec![DefinitionSource::new(
        "memory:draft-behavior.rcore",
        SourceKind::RcoreBehavior,
        "{}",
    )];
    let provider = EmptyInitializer(ClassInitializationContract {
        version: 1,
        schema_revision: schema.revision.clone(),
        class: "Item".into(),
        class_identity: schema.identity("Item"),
        provider: ProviderId {
            name: "test.draft".into(),
            version: "1".into(),
        },
        sources: vec![sources[0].id.clone()],
        members: vec![],
    });
    let context = InitializationContext {
        schema: &schema,
        expected: &provider.0,
        sources: &sources,
        provider: Some(&provider),
        conversions: None,
    };
    for &mode in modes {
        let mut store = crate::stores::store::<Store>(&schema).unwrap();
        let setup = storage::atomic(&mut store, |store| {
            store.create_element(&schema, "holder", "Holder", "model", 0, 0)?;
            if mode == "pending-collision" {
                store.create_element(&schema, "?later", "Item", "model", 0, 0)?;
            }
            match mode {
                "containment" => {
                    store.set_unresolved_reference("holder", "child", "later", false)?
                }
                "missing" => store.resolve_reference("holder", "target", None, "absent")?,
                "wrong-class" => store.resolve_reference("holder", "target", None, "holder")?,
                "json" => store.set_value("holder", "target", Some(json!({"spelling":"later"})))?,
                _ => store.set_unresolved_reference("holder", "target", "later", false)?,
            }
            if mode == "attribute" {
                store.set_value("holder", "count", Some(json!("not an integer")))?;
            }
            Ok(())
        });
        if mode == "json" {
            assert!(setup.is_err());
            assert!(store.get("holder").unwrap().is_none());
            continue;
        }
        setup.unwrap();
        let before = storage::export(&store).unwrap();
        let request = || Construction {
            id: "created",
            resource: "model",
            authored: &[],
        };
        assert!(
            context
                .create(&mut store, request(), &mut QueryBudget::new(1000))
                .is_err(),
            "strict {mode}"
        );
        assert_eq!(storage::export(&store).unwrap(), before);
        let result = context.create_with_reference_policy(
            &mut store,
            request(),
            &mut QueryBudget::new(1000),
            DraftReferencePolicy::PendingNonContainment,
        );
        if matches!(mode, "pending" | "pending-collision") {
            let receipt = result.unwrap();
            assert_eq!(receipt.object, "created");
            assert!(storage::validate_draft(&store, &schema).is_err());
            assert!(storage::validate(&store, &schema).is_err());
            storage::atomic(&mut store, |store| {
                store.resolve_reference("holder", "target", None, "created")
            })
            .unwrap();
            storage::validate(&store, &schema).unwrap();
        } else {
            assert!(result.is_err(), "provisional {mode}");
            assert_eq!(storage::export(&store).unwrap(), before, "rollback {mode}");
        }
    }
}
