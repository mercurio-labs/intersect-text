//! Inventory-level routing control; semantic domain controls remain in their focused suites.
use intersect_model::conversion::with_catalog;
use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    compiler::{standard_datatype, standard_datatype_source},
    conversion::{ConversionContract, MemberContract, MemberType, ProviderId},
    ecore::{Document, ECORE, XSI},
    scalars::Builtin,
};
use intersect_text::{EvaluationLimits, Source, evaluate_with_conversions};
use std::collections::BTreeSet;

const E05: &[&str] = &[
    "EString",
    "EBoolean",
    "EBooleanObject",
    "EByte",
    "EByteObject",
    "EShort",
    "EShortObject",
    "EInt",
    "EIntegerObject",
    "ELong",
    "ELongObject",
    "EFloat",
    "EFloatObject",
    "EDouble",
    "EDoubleObject",
];
const E06: &[&str] = &[
    "EBigInteger",
    "EBigDecimal",
    "EByteArray",
    "EChar",
    "ECharacterObject",
    "EDate",
    "EJavaClass",
    "EJavaObject",
    "EDiagnosticChain",
    "EEList",
    "EEnumerator",
    "EFeatureMap",
    "EFeatureMapEntry",
    "EMap",
    "EResource",
    "EResourceSet",
    "ETreeIterator",
    "EInvocationTargetException",
];
fn provider(name: &str) -> &'static str {
    match name {
        "EJavaClass" => "ecore.class",
        "EJavaObject" => "example.rgb",
        "EDiagnosticChain" => "ecore.diagnostic-chain",
        "EEList" => "ecore.list",
        "EEnumerator" => "ecore.enumerator",
        "EFeatureMap" => "ecore.feature-map",
        "EFeatureMapEntry" => "ecore.feature-entry",
        "EMap" => "ecore.map",
        "EResource" => "ecore.definition-resource",
        "EResourceSet" => "ecore.definition-resource-set",
        "ETreeIterator" => "ecore.definition-tree-iterator",
        "EInvocationTargetException" => "ecore.invocation-error",
        _ => panic!("Unaccounted object datatype {name}"),
    }
}
#[test]
fn pinned_datatype_inventory_has_exhaustive_scalar_or_explicit_provider_consumers() {
    let pinned = standard_datatype_source();
    let document = Document::parse(pinned.clone()).unwrap();
    let mut actual = BTreeSet::new();
    let mut contracts = Vec::new();
    for (ordinal, node) in document.root.children.iter().enumerate() {
        let datatype = node.attributes.iter().any(|a| {
            a.name.namespace.as_deref() == Some(XSI)
                && a.name.local == "type"
                && a.type_name.as_ref().is_some_and(|t| {
                    t.namespace.as_deref() == Some(ECORE) && t.local == "EDataType"
                })
        });
        if !datatype {
            continue;
        }
        let attr = |name: &str| {
            node.attributes
                .iter()
                .find(|a| a.name.namespace.is_none() && a.name.local == name)
                .map(|a| a.value.as_str())
        };
        let name = attr("name").unwrap();
        assert!(actual.insert(name.to_owned()));
        if let Some(codec) = Builtin::named(name) {
            codec.validate(&codec.default_value()).unwrap();
            continue;
        }
        let imported = standard_datatype(name).unwrap();
        assert_eq!(imported.declaration().source, pinned.id);
        assert_eq!(imported.declaration().path, vec![ordinal]);
        assert_eq!(imported.instance_class_name(), attr("instanceClassName"));
        assert_eq!(
            imported.serializable(),
            attr("serializable") != Some("false")
        );
        assert!(imported.source_payload().unwrap().contains(name));
        let member = MemberContract {
            nullable: false,
            value: MemberType::Builtin {
                codec: Builtin::EString,
            },
        };
        let members = match name {
            "EEList" => vec![member],
            "EMap" => vec![member.clone(), member],
            _ => vec![],
        };
        contracts.push(ConversionContract {
            datatype: imported.declaration().clone(),
            provider: ProviderId {
                name: provider(name).into(),
                version: "1".into(),
            },
            serializable: imported.serializable(),
            declarations: vec![],
            members,
        });
    }
    let expected: BTreeSet<String> = E05.iter().chain(E06).map(|s| (*s).into()).collect();
    assert_eq!(E05.len(), 15);
    assert_eq!(E06.len(), 18);
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 33);
    assert_eq!(contracts.len(), 12);
    let features: String = actual.iter().map(|name| format!(r#"<eStructuralFeatures xsi:type="e:EAttribute" name="{name}" eType="e:EDataType http://www.eclipse.org/emf/2002/Ecore#//{name}"/>"#)).collect();
    let ecore = format!(
        r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="inventory" nsURI="urn:inventory"><eClassifiers xsi:type="e:EClass" name="Holder">{features}</eClassifiers></e:EPackage>"#
    );
    let xtext =
        "grammar inventory.Test import \"urn:inventory\" as m Holder returns m::Holder: 'holder';";
    let rcore_behavior = r#"rcore 1; package emptyRules version "1";"#;
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![DefinitionSource::new(
            "memory:ecore",
            SourceKind::Ecore,
            &ecore,
        )],
        namespaces: vec![],
    };
    with_catalog(&bundle, &contracts, |registry| {
        let package =
            intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
                &ecore,
                xtext,
                rcore_behavior,
                registry,
            )
            .unwrap();
        assert_eq!(package.schema.classes["Holder"].features.len(), 33);
        let result = evaluate_with_conversions(
            &package,
            &[Source {
                uri: "inventory.model".into(),
                text: "holder".into(),
            }],
            EvaluationLimits::native(),
            registry,
        );
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.model.elements.len(), 1);
        result
            .model
            .validate_with_conversions(&package.schema, registry)?;
        for name in &actual {
            let value = &result.model.elements[0].values[name];
            if let Some(codec) = Builtin::named(name) {
                assert_eq!(*value, codec.default_value());
            } else {
                assert!(value.is_null(), "{name}");
            }
        }
        Ok(())
    })
    .unwrap();
    // Every required object adapter must be selected even when every value is absent.
    for omitted in 0..contracts.len() {
        let mut selected = contracts.clone();
        let missing = selected.remove(omitted);
        with_catalog(&bundle, &selected, |registry| {
            assert!(
                intersect_text_acceptance::migration_fixture::compile_migrated_fixture_with_conversions(
                    &ecore,
                    xtext,
                    rcore_behavior,
                    registry
                )
                .is_err(),
                "missing {}",
                missing.provider.name
            );
            Ok(())
        })
        .unwrap();
    }
}
