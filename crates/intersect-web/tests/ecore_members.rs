use intersect_model::definitions::bundle::{DefinitionBundle, DefinitionSource, SourceKind};
use intersect_web::dispatch;
use serde_json::json;

#[test]
fn host_uses_source_verified_members_and_preserves_import_diagnostics() {
    let text = "<e:EPackage xmlns:e='http://www.eclipse.org/emf/2002/Ecore' xmlns:xsi='http://www.w3.org/2001/XMLSchema-instance' name='test' nsURI='urn:test'><eClassifiers xsi:type='e:EClass' name='A'><eOperations name='f'/></eClassifiers><eClassifiers xsi:type='e:EClass' name='B' eSuperTypes='#//A'><eOperations name='f'/></eClassifiers></e:EPackage>";
    let source = DefinitionSource::new("file:///members.ecore", SourceKind::Ecore, text);
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![source.clone()],
        namespaces: vec![],
    };
    let request = json!({"method":"ecore/members", "bundle":bundle, "entry":source.id, "namespace":"urn:test", "class":"B"});
    let result = dispatch(&request.to_string()).unwrap();
    assert_eq!(result["ok"], true);
    assert_eq!(result["operations_executable"], false);
    assert!(result["members"]["features"].as_array().unwrap().is_empty());
    assert_eq!(result["members"]["operations"].as_array().unwrap().len(), 2);
    assert_eq!(
        result["members"]["resolved_operations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(result["members"].to_string().contains(&source.id.sha256));
    let mut invalid = request.clone();
    invalid["class"] = json!("Missing");
    assert_eq!(dispatch(&invalid.to_string()).unwrap()["ok"], false);
    let mut invalid = request.clone();
    invalid["bundle"]["sources"][0]["text"] = json!(format!("{text} "));
    let result = dispatch(&invalid.to_string()).unwrap();
    assert_eq!(result["ok"], false);
    assert!(
        result["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("digest")
    );
    let mut invalid = request;
    invalid["bundle"]["sources"][0]["text"] = json!(" ".repeat(256_001));
    assert!(
        dispatch(&invalid.to_string())
            .unwrap_err()
            .contains("size budget")
    );
}
