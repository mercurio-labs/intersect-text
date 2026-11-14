//! Handwritten host adapter over the shared source-preserving definition editor.
use intersect_model::definitions::ecore_document;
use serde_json::Value;

pub(crate) fn dispatch(method: &str, request: &Value) -> Result<Value, String> {
    let source = request
        .get("source")
        .and_then(Value::as_str)
        .ok_or("Missing Ecore source")?;
    let document = match method {
        "definition/inspect" => ecore_document::inspect(source)?,
        "definition/edit" => {
            let edit = serde_json::from_value(
                request
                    .get("edit")
                    .cloned()
                    .ok_or("Missing definition edit")?,
            )
            .map_err(|e| format!("Invalid definition edit: {e}"))?;
            ecore_document::edit(source, edit)?
        }
        _ => return Err(format!("Unknown definition method {method}")),
    };
    serde_json::to_value(document).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn dispatches_inspection_and_typed_edits() {
        let source = r#"<ecore:EPackage xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" name="library" nsURI="urn:library" nsPrefix="library"/>"#;
        let inspected = crate::dispatch(
            &json!({ "method": "definition/inspect", "source": source }).to_string(),
        )
        .unwrap();
        assert_eq!(inspected["name"], "library");
        let edited = crate::dispatch(&json!({ "method": "definition/edit", "source": source, "edit": { "op": "add_class", "name": "Book" } }).to_string()).unwrap();
        assert_eq!(edited["classes"][0]["name"], "Book");
        assert!(crate::dispatch(&json!({ "method": "definition/edit", "source": source, "edit": { "op": "add_class", "name": 12 } }).to_string()).is_err());
    }
}
