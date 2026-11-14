//! Checked cursor transitions through the retained language provider.
use intersect_model::definitions::{conversion::ConvertedValue, resources::DeclarationId};
use intersect_model::tree_iterator_conversion::TreeIteratorConverter;
use serde_json::{Value, json};
pub fn step(request: &Value) -> Result<Value, String> {
    let package = crate::conversions::rebuild(request.get("package").ok_or("Missing package")?)?;
    let datatype: DeclarationId = serde_json::from_value(
        request
            .get("datatype")
            .cloned()
            .ok_or("Missing iterator datatype")?,
    )
    .map_err(|e| e.to_string())?;
    let contract = package
        .schema
        .conversion_contracts
        .iter()
        .find(|c| c.datatype == datatype)
        .ok_or("Missing iterator provider contract")?;
    if contract.provider.name != "ecore.definition-tree-iterator"
        || contract.provider.version != "1"
    {
        return Err("Iterator transitions require the installed tree provider".into());
    }
    let action = request
        .get("action")
        .and_then(Value::as_str)
        .ok_or("Missing iterator action")?;
    crate::conversions::with_package(&package, |registry| {
        let adapter = TreeIteratorConverter::new(
            contract.clone(),
            &crate::conversions::retained_bundle(&package)?,
        )?;
        let mut snapshot = if action == "start" {
            if request.get("cursor").is_some() {
                return Err("Start creates a fresh iterator and does not consume a cursor".into());
            }
            adapter.start()
        } else {
            let cursor: ConvertedValue = serde_json::from_value(
                request
                    .get("cursor")
                    .cloned()
                    .ok_or("Missing iterator cursor")?,
            )
            .map_err(|e| e.to_string())?;
            let ConvertedValue::Converted {
                datatype: actual,
                value,
                ..
            } = &cursor;
            if *actual != datatype {
                return Err("Cursor datatype differs from requested iterator".into());
            }
            registry.validate(&cursor)?;
            adapter.snapshot(value)?
        };
        let item = match action {
            "start" | "has_next" => None,
            "next" => adapter.next(&mut snapshot)?,
            "prune" => {
                adapter.prune(&mut snapshot)?;
                None
            }
            "remove" => {
                adapter.remove(&mut snapshot)?;
                None
            }
            _ => return Err("Unknown iterator action".into()),
        };
        let has_next = adapter.has_next(&snapshot)?;
        let cursor = registry.bind(
            &datatype,
            serde_json::to_value(snapshot).map_err(|e| e.to_string())?,
        )?;
        Ok(json!({"cursor":cursor,"item":item,"has_next":has_next}))
    })
}
