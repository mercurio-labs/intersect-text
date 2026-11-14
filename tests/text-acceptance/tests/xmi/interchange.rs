use super::Store;
use intersect_model::core::{
    interchange::{FormatRegistry, ModelFormat, Resource},
    model::Model,
    storage::{self},
};
use intersect_model::definitions::schema::Schema;
use intersect_model::xmi::XmiFormat;
use intersect_text::{Source, evaluate};
use serde_json::json;

const ECORE: &str = include_str!("../../../../fixtures/library/library.ecore");
const XTEXT: &str = include_str!("../../../../fixtures/library/library.xtext");
const RCORE_BEHAVIOR: &str = include_str!("../../../../fixtures/library/library-behavior.rcore");

control_test! {
pub fn cached_emf_output_imports_without_intersect_extensions() {
    let p = package();
    let resources = [
        resource(
            "resource-0000.xmi",
            include_str!("../../../../fixtures/xmi/emf/resource-0000.xmi"),
        ),
        resource(
            "resource-0001.xmi",
            include_str!("../../../../fixtures/xmi/emf/resource-0001.xmi"),
        ),
    ];
    let model = registry().import("xmi", &p.schema, &resources).unwrap();
    assert_eq!(model.elements.len(), 6);
    assert_eq!(model.elements[1].values["edition"], json!(1));
    assert_eq!(
        model.elements[1].values["library"],
        json!("resource-0000.xmi#e0")
    );
    assert_eq!(
        model.elements[5].values["copy"],
        json!("resource-0000.xmi#e1")
    );
    assert_eq!(
        model.elements[5].values["member"],
        json!("resource-0000.xmi#e3")
    );
    let mut result = intersect_text::Evaluation {
        model,
        diagnostics: vec![],
        references: vec![],
        derived: Default::default(),
    };
    intersect_text::apply_semantics(&p, &mut result);
    assert!(result.diagnostics.is_empty());
    assert_eq!(
        result.derived["resource-0000.xmi#e1"]["available"],
        json!(false)
    );
}
}

control_test! {
pub fn incomplete_resources_provenance_and_lossy_export_are_rejected() {
    let r = registry();
    let schema = package().schema;
    let model = library();
    let exported = r.export("xmi", &schema, &model).unwrap();
    assert!(r.import("xmi", &schema, &exported[1..]).is_err());
    for (from, to) in [
        ("xmiId=\"e0\"", "xmiId=\"missing\""),
        ("order=\"1\"", "order=\"0\""),
    ] {
        let mut broken = exported.clone();
        broken[0].bytes = String::from_utf8(broken[0].bytes.clone())
            .unwrap()
            .replace(from, to)
            .into_bytes();
        assert!(r.import("xmi", &schema, &broken).is_err());
    }
    let mut sparse = model.clone();
    sparse.elements[1].values.remove("edition");
    assert!(r.export("xmi", &schema, &sparse).is_err());
    let mut control = model;
    control.elements[1]
        .values
        .insert("title".into(), json!("invalid\u{0000}"));
    assert!(r.export("xmi", &schema, &control).is_err());
    let malformed = doc("<!-- bad -- comment --><l:Library name=\"L\"/>");
    assert!(
        r.import("xmi", &schema, &[resource("a.xmi", &malformed)])
            .is_err()
    );
}
}
fn package() -> intersect_text::package::Package {
    intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        ECORE,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap()
}
fn registry() -> FormatRegistry {
    let mut r = FormatRegistry::default();
    r.register(XmiFormat).unwrap();
    r
}
fn resource(uri: &str, text: &str) -> Resource {
    Resource {
        uri: uri.into(),
        bytes: text.as_bytes().to_vec(),
    }
}
fn library() -> Model {
    let result = evaluate(
        &package(),
        &[
            Source {
                uri: "file:///models/catalog.library".into(),
                text: include_str!("../../../../fixtures/library/catalog.library").into(),
            },
            Source {
                uri: "file:///models/loans.library".into(),
                text: include_str!("../../../../fixtures/library/loans.library").into(),
            },
        ],
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.model
}
fn doc(body: &str) -> String {
    format!(
        r#"<xmi:XMI xmlns:xmi="http://www.omg.org/XMI" xmlns:l="urn:intersect:example:library:1" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmi:version="2.0">{body}</xmi:XMI>"#
    )
}

control_test! {
pub fn library_round_trip_preserves_identity_provenance_order_and_semantics() {
    let p = package();
    let r = registry();
    let mut model = library();
    model.elements[1]
        .values
        .insert("title".into(), json!("Café & <雪> \"quoted\"\n\t\r"));
    model.elements.swap(0, 3);
    let exported = r.export("xmi", &p.schema, &model).unwrap();
    assert_eq!(exported.len(), 2);
    assert_eq!(exported, r.export("xmi", &p.schema, &model).unwrap());
    let mut reversed = exported.clone();
    reversed.reverse();
    let restored = r.import("xmi", &p.schema, &reversed).unwrap();
    assert_eq!(restored, model);
    let mut evaluation = intersect_text::Evaluation {
        model: restored,
        diagnostics: vec![],
        references: vec![],
        derived: Default::default(),
    };
    intersect_text::apply_semantics(&p, &mut evaluation);
    assert!(evaluation.diagnostics.is_empty());
    assert_eq!(
        evaluation.derived["file:///models/catalog.library#1"]["available"],
        json!(false)
    );
}
}
control_test! {
pub fn emf_style_paths_defaults_opposites_and_relative_forward_references() {
    let p = package();
    let catalog = r#"<lib:Library xmlns:lib="urn:intersect:example:library:1" name="Main"><copies name="Dune" title="Dune"/><members name="Alice"/></lib:Library>"#;
    let loans = doc(
        r##"<l:Library name="Loans"><loans name="L"><copy href="../catalog.xmi#//@copies.0"/><member href="../catalog.xmi#//@members.0"/></loans></l:Library>"##,
    );
    let result = registry()
        .import(
            "xmi",
            &p.schema,
            &[
                resource("file:///models/sub/loans.xmi", &loans),
                resource("file:///models/catalog.xmi", catalog),
            ],
        )
        .unwrap();
    let copy = result
        .elements
        .iter()
        .find(|e| e.class == "BookCopy")
        .unwrap();
    assert_eq!(copy.values["edition"], json!(1));
    assert_eq!(
        copy.values["library"],
        json!("file:///models/catalog.xmi#/")
    );
    let loan = result.elements.iter().find(|e| e.class == "Loan").unwrap();
    assert_eq!(loan.values["copy"], json!(copy.id));
    assert_eq!(copy.resource, "file:///models/catalog.xmi");
    assert!(copy.end > copy.start);
}
}
control_test! {
pub fn aliases_ids_multiple_roots_inheritance_and_reference_attributes() {
    let text = r##"<a:XMI xmlns:a="http://www.omg.org/XMI" xmlns:z="urn:intersect:example:library:1" xmlns:t="http://www.w3.org/2001/XMLSchema-instance"><z:Library a:id="lib" name="Library"><copies t:type="z:BookCopy" a:id="book" name="Book" edition="-2147483648"/></z:Library><z:Member a:id="member" name="Reader"/><z:Loan a:id="loan" name="Loan" copy="book" member="#member"/></a:XMI>"##;
    let model = registry()
        .import("xmi", &package().schema, &[resource("a.xmi", text)])
        .unwrap();
    assert_eq!(model.elements[1].values["edition"], json!(i32::MIN));
    assert_eq!(model.elements[3].values["copy"], json!("a.xmi#book"));
}
}
control_test! {
pub fn malformed_unsupported_and_inconsistent_inputs_fail() {
    let valid =
        doc(r#"<l:Library xmi:id="lib" name="L"><copies xmi:id="book" name="B"/></l:Library>"#);
    let cases = [
        valid.replace("name=\"B\"", "name=\"B\" mystery=\"lost\""),
        valid.replace("name=\"B\"", "name=\"B\" available=\"true\""),
        valid.replace("name=\"B\"", "name=\"B\" edition=\"2147483648\""),
        valid.replace("name=\"B\"", "name=\"B\" library=\"missing\""),
        valid.replace("name=\"B\"", "name=\"B\" library=\"\""),
        valid.replace("xmi:id=\"book\"", "xmi:id=\"lib\""),
        valid.replace("urn:intersect:example:library:1", "urn:wrong"),
        valid.replace("xmi:version=\"2.0\"", "xmi:version=\"9.0\""),
        valid.replace(
            "</xmi:XMI>",
            "<xmi:Extension extender=\"other\"/></xmi:XMI>",
        ),
        valid.replace("name=\"L\"", "name=\"&unknown;\""),
        valid.replace("</l:Library>", "</l:Wrong>"),
        format!("<!DOCTYPE x [<!ENTITY secret SYSTEM 'file:///secret'>]>{valid}"),
        format!("{valid}<extra/>"),
        doc(r#"<l:Named name="abstract"/>"#),
        doc(r#"<l:Loan name="missingRequiredReferences"/>"#),
    ];
    for text in cases {
        assert!(
            registry()
                .import("xmi", &package().schema, &[resource("a.xmi", &text)])
                .is_err(),
            "accepted {text}"
        );
    }
}
}
store_test! {
pub fn failed_store_import_and_revision_mismatch_preserve_previous_model() {
    let r = registry();
    let schema = package().schema;
    let model = library();
    let mut store = crate::stores::from_model::<Store>(&schema, &model).unwrap();
    let bad = resource("bad.xmi", &doc(r#"<l:Loan name="bad"/>"#));
    assert!(r.import_into("xmi", &schema, &[bad], &mut store).is_err());
    assert_eq!(storage::export(&store).unwrap(), model);
    let resources = r.export("xmi", &schema, &model).unwrap();
    let mut changed = schema.clone();
    changed.revision.push('x');
    assert!(
        r.import_into("xmi", &changed, &resources, &mut store)
            .is_err()
    );
    assert_eq!(storage::export(&store).unwrap(), model);
    let mut fresh = crate::stores::store::<Store>(&schema).unwrap();
    r.import_into("xmi", &schema, &resources, &mut fresh)
        .unwrap();
    assert_eq!(storage::export(&fresh).unwrap(), model);
    let result: Result<(), String> = storage::atomic(&mut fresh, |store| {
        r.import_into("xmi", &schema, &[], store)?;
        Err("language validation rejected".into())
    });
    assert!(result.is_err());
    assert_eq!(storage::export(&fresh).unwrap(), model);
}
}
struct JsonFormat {
    extension: &'static [&'static str],
}
impl ModelFormat for JsonFormat {
    fn id(&self) -> &str {
        "test-json"
    }
    fn extensions(&self) -> &[&str] {
        self.extension
    }
    fn media_type(&self) -> &str {
        "application/json"
    }
    fn decode(&self, _: &Schema, resources: &[Resource]) -> Result<Model, String> {
        serde_json::from_slice(&resources.first().ok_or("Missing JSON")?.bytes)
            .map_err(|e| e.to_string())
    }
    fn encode(&self, _: &Schema, model: &Model) -> Result<Vec<Resource>, String> {
        Ok(vec![Resource {
            uri: "model.test".into(),
            bytes: serde_json::to_vec(model).map_err(|e| e.to_string())?,
        }])
    }
}
control_test! {
pub fn another_codec_registers_without_dispatcher_changes_and_conflicts_are_atomic() {
    let mut r = registry();
    r.register_extension("library", "xmi").unwrap();
    assert_eq!(r.for_uri("test.LIBRARY").unwrap().id(), "xmi");
    assert!(r.register_extension("library", "xmi").is_err());
    assert!(r.register_extension("bad", "missing").is_err());
    assert!(
        r.register(JsonFormat {
            extension: &["test", "XMI"]
        })
        .is_err()
    );
    assert!(r.for_uri("model.test").is_err());
    assert!(r.get("test-json").is_err());
    r.register(JsonFormat {
        extension: &["test"],
    })
    .unwrap();
    assert_eq!(
        r.for_uri("file:///model.XMI?download=1").unwrap().id(),
        "xmi"
    );
    assert_eq!(r.for_uri("model.test").unwrap().id(), "test-json");
    assert!(r.register(XmiFormat).is_err());
    assert!(r.for_uri("model.unknown").is_err());
    let schema = package().schema;
    let model = library();
    let bytes = r.export("test-json", &schema, &model).unwrap();
    assert_eq!(r.import("test-json", &schema, &bytes).unwrap(), model);
    let invalid = Model {
        schema_revision: "wrong".into(),
        resources: Default::default(),
        elements: vec![],
    };
    let bytes = JsonFormat { extension: &[] }
        .encode(&schema, &invalid)
        .unwrap();
    assert!(r.import("test-json", &schema, &bytes).is_err());
}
}
control_test! {
pub fn empty_model_and_many_primitive_values_round_trip() {
    let ecore = ECORE.replace(
        r##"<eClassifiers xsi:type="ecore:EClass" name="Member" eSuperTypes="#//Named"/>"##,
        r##"<eClassifiers xsi:type="ecore:EClass" name="Member" eSuperTypes="#//Named">
          <eStructuralFeatures xsi:type="ecore:EAttribute" name="tags" unique="false" upperBound="-1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
          <eStructuralFeatures xsi:type="ecore:EAttribute" name="enabled" defaultValueLiteral="false" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EBoolean"/>
        </eClassifiers>"##,
    );
    assert_ne!(ecore, ECORE);
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_fixture(
        &ecore,
        XTEXT,
        RCORE_BEHAVIOR,
    )
    .unwrap()
    .schema;
    let r = registry();
    let empty = Model {
        schema_revision: schema.revision.clone(),
        resources: Default::default(),
        elements: vec![],
    };
    assert_eq!(
        r.import("xmi", &schema, &r.export("xmi", &schema, &empty).unwrap())
            .unwrap(),
        empty
    );
    let text = doc(
        r#"<l:Member name="A" enabled="1"><tags>two words</tags><tags></tags><tags>&lt;&amp;雪</tags><tags>two words</tags></l:Member>"#,
    );
    let model = r
        .import("xmi", &schema, &[resource("a.xmi", &text)])
        .unwrap();
    assert_eq!(
        model.elements[0].values["tags"],
        json!(["two words", "", "<&雪", "two words"])
    );
    assert_eq!(model.elements[0].values["enabled"], json!(true));
    assert_eq!(
        r.import("xmi", &schema, &r.export("xmi", &schema, &model).unwrap())
            .unwrap(),
        model
    );
}
}

control_test! {
pub fn enum_attributes_and_collections_round_trip_exact_literal_identities() {
    let source = intersect_model::definitions::bundle::DefinitionSource::new(
        "memory:enum.rcore",
        intersect_model::definitions::bundle::SourceKind::Rcore,
        "rcore 1; package library version \"1\"; metamodel \"urn:intersect:example:library:1\" prefix lib { enum Condition { GOOD = 0; ALIAS = 0; WORN = -1; } class Copy { attr condition: Condition [0,1] default \"GOOD\"; attr history: Condition [0,*]; } }",
    );
    let lowered =
        intersect_model::definitions::rcore::lower_structure(source, "memory:enum.ecore").unwrap();
    let schema = intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(
        &lowered.generated.text,
    )
    .unwrap();
    let r = registry();
    let text =
        doc(r#"<l:Copy condition="ALIAS"><history>GOOD</history><history>WORN</history></l:Copy>"#);
    let model = r
        .import("xmi", &schema, &[resource("enum.xmi", &text)])
        .unwrap();
    let enumeration = &schema.enumerations["Condition"];
    let alias = serde_json::to_value(enumeration.parse("ALIAS").unwrap()).unwrap();
    assert_eq!(model.elements[0].values["condition"], alias);
    let exported = r.export("xmi", &schema, &model).unwrap();
    let xml = String::from_utf8(exported[0].bytes.clone()).unwrap();
    assert!(xml.contains("condition=\"ALIAS\""));
    assert!(xml.contains("<history>WORN</history>"));
    assert_eq!(r.import("xmi", &schema, &exported).unwrap(), model);
    for spelling in ["MISSING", "0", " ALIAS"] {
        assert!(
            r.import(
                "xmi",
                &schema,
                &[resource(
                    "enum.xmi",
                    &text.replace("condition=\"ALIAS\"", &format!("condition=\"{spelling}\""))
                )]
            )
            .is_err()
        );
    }
    let mut wrong_identity = model.clone();
    wrong_identity.elements[0]
        .values
        .get_mut("condition")
        .unwrap()["literal"]["source"]["sha256"] = json!("stale");
    assert!(r.export("xmi", &schema, &wrong_identity).is_err());
}
}

// Codec-only controls run once; they have no selectable store backend.
control_module! {
    #[path = "resource_lifecycle.rs"]
    mod resource_lifecycle;
}
