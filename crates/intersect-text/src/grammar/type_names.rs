//! Handwritten grammar type-name policy over shared model metadata.
//! Lookup and inspection do not grant source-bound compilation or execution.
use intersect_model::definitions::schema::{
    EOBJECT_REFERENCE_TOP, Schema, eobject_reference_top_declaration,
};

/// Resolve an explicitly imported namespace for a reference, independently
/// from rule construction. The internal top key is never a source spelling.
pub fn reference_key<'schema>(
    schema: &'schema Schema,
    namespace: &str,
    name: &str,
) -> Result<&'schema str, String> {
    if name == EOBJECT_REFERENCE_TOP {
        return Err("Internal reference-domain keys are not grammar type names".into());
    }
    if namespace == intersect_model::definitions::ecore::ECORE {
        if name != "EObject" {
            return Err(
                "Only pinned Ecore EObject is supported as an imported grammar reference target"
                    .into(),
            );
        }
        schema.validate_reference_top()?;
        if schema.reference_top.as_ref() != Some(&eobject_reference_top_declaration()?) {
            return Err("Pinned EObject reference context is not installed".into());
        }
        return Ok(EOBJECT_REFERENCE_TOP);
    }
    if namespace != schema.uri {
        return Err("Grammar reference namespace is not the selected metamodel".into());
    }
    let key = schema.language_class_key(name)?;
    if schema
        .declarations
        .get(key)
        .is_some_and(|declaration| declaration.namespace != namespace)
    {
        return Err(
            "Grammar reference classifier belongs to a different imported namespace".into(),
        );
    }
    Ok(key)
}

/// Resolve a constructible rule class while retaining its explicit namespace.
pub(crate) fn rule_class_key<'schema>(
    schema: &'schema Schema,
    namespace: &str,
    name: &str,
) -> Result<&'schema str, String> {
    if namespace == intersect_model::definitions::ecore::ECORE {
        if name != "EObject" {
            return Err("Only explicitly materialized pinned EObject is supported as an imported rule class".into());
        }
        let receiver = intersect_model::definitions::generic_types::RuntimeType::new(
            intersect_model::definitions::generic_types::Type::Classifier {
                declaration: eobject_reference_top_declaration()?,
                arguments: vec![],
                raw: false,
            },
        )?;
        return schema.class_key_for_type(&receiver);
    }
    if namespace != schema.uri {
        return Err("Rule classifier namespace is not the selected metamodel".into());
    }
    let key = schema.language_class_key(name)?;
    if schema
        .declarations
        .get(key)
        .is_some_and(|declaration| declaration.namespace != namespace)
    {
        return Err("Rule classifier belongs to a different imported namespace".into());
    }
    Ok(key)
}
