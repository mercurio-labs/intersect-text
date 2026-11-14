//! Same literal ordinary input world for original/after strict parity controls.
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore_migration::BundleMigrationReport;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
use serde_json::{Value, json};

pub const EMPTY: &str = "rcore 1; package preparedRules version \"1\";";
pub const GOOD: &str = "rcore 1; package preparedRules version \"1\"; import model \"urn:prepared\" as p; derive p.Model.count using count(values);";
pub const INVALID: &str = "rcore 1; package preparedRules version \"1\"; import model \"urn:prepared\" as p; derive p.Model.count using count(name);";
pub const DUPLICATE: &str = "rcore 1; package preparedRules version \"1\"; import model \"urn:prepared\" as p; derive p.Model.count using count(values); derive p.Model.count using count(values);";

pub fn inputs(behavior: &str) -> Result<(BundleCompilation, BundleMigrationReport), String> {
    let ecore = DefinitionSource::new(
        "memory:prepared/model.ecore",
        SourceKind::Ecore,
        r##"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="prepared" nsURI="urn:prepared" nsPrefix="p">
 <eClassifiers xsi:type="ecore:EClass" name="Model">
  <eStructuralFeatures xsi:type="ecore:EAttribute" name="name" lowerBound="1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//EString"/>
  <eStructuralFeatures xsi:type="ecore:EAttribute" name="values" upperBound="-1" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//ELong"/>
  <eStructuralFeatures xsi:type="ecore:EAttribute" name="count" derived="true" eType="ecore:EDataType http://www.eclipse.org/emf/2002/Ecore#//ELong"/>
 </eClassifiers>
 <eClassifiers xsi:type="ecore:EClass" name="Sub" eSuperTypes="#//Model"/>
</ecore:EPackage>"##,
    );
    let mut xtext = DefinitionSource::new(
        "memory:prepared/Model.xtext",
        SourceKind::Xtext,
        "grammar prepared.Model hidden(WS)\nimport \"urn:prepared\" as p\nimport \"http://www.eclipse.org/emf/2002/Ecore\" as ecore\nModel returns p::Model: 'model' name=ID;\nterminal ID: ('a'..'z')+;\nterminal WS: (' '|'\\t'|'\\n'|'\\r')+;\n",
    );
    xtext.dependencies = vec![ecore.id.clone()];
    let mut rules = DefinitionSource::new(
        "memory:prepared/behavior.rcore",
        SourceKind::RcoreBehavior,
        behavior,
    );
    rules.dependencies = vec![ecore.id.clone()];
    let original = BundleCompilation {
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), xtext.clone(), rules.clone()],
            namespaces: vec![],
        },
        ecore: ecore.id.clone(),
        namespace: "urn:prepared".into(),
        xtext: Some(xtext.id),
        rcore_behavior: rules.id,
        materialized_types: vec![],
    };
    let report = intersect_model::migration::migrate_ecore_bundle(
        original.bundle.clone(),
        &[(ecore.id, "memory:prepared/model.rcore".into())].into(),
    )
    .map_err(|error| error.to_string())?;
    let compilation = intersect_text::migration::remap_compilation(&report, &original)?;
    Ok((compilation, report))
}

pub fn strict_observations() -> Result<Value, String> {
    let mut cases = Vec::new();
    for (label, source) in [
        ("good", GOOD),
        ("missing", EMPTY),
        ("invalid", INVALID),
        ("duplicate", DUPLICATE),
        ("recovery", GOOD),
    ] {
        let (input, _) = inputs(source)?;
        let result = intersect_text::ordinary::compile_with_context(
            input,
            Profile::NativeV67,
            &[],
            None,
            CompilationSourcePolicy::default(),
            CompilationSourcePolicy::default(),
        );
        let outcome = match result {
            Ok(package) => json!({"package": package}),
            Err(errors) => json!({"diagnostics": errors}),
        };
        cases.push(json!({"case": label, "outcome": outcome}));
    }
    Ok(json!(cases))
}
