//! Exact imported enum-rule bindings; values use the shared enum codec.
use super::{Datatype, Expr, Kind, Parser, Rule};
use intersect_model::definitions::schema::Schema;
use std::collections::BTreeMap;

pub(super) fn key<'a>(schema: &'a Schema, namespace: &str, name: &str) -> Result<&'a str, String> {
    let mut found = schema.enumerations.iter().filter(|(key, value)| {
        schema.declarations.get(*key).map_or_else(
            || namespace == schema.uri && *key == name && value.name() == name,
            |declaration| {
                declaration.namespace == namespace
                    && declaration.name == name
                    && declaration.source == value.declaration().source
                    && declaration.path == value.declaration().path
            },
        )
    });
    let (key, _) = found
        .next()
        .ok_or_else(|| format!("Unknown imported enum {namespace}::{name}"))?;
    if found.next().is_some() {
        return Err(format!("Ambiguous imported enum {namespace}::{name}"));
    }
    Ok(key)
}

pub(super) fn parse(
    parser: &mut Parser,
    schema: &Schema,
    profile: super::Profile,
    inferred_name: Option<&str>,
    bound: Option<&(String, String)>,
) -> Result<Rule, String> {
    let explicit = parser.eat("returns");
    let name = if explicit {
        parser.type_name()?
    } else {
        inferred_name.ok_or("Expected 'returns'")?.to_owned()
    };
    let (namespace, local) = if !explicit && bound.is_some() {
        let (namespace, name) = bound.ok_or("Missing inferred enum type")?;
        (namespace.as_str(), name.as_str())
    } else {
        super::imported_type(
            &name,
            &parser.aliases,
            schema,
            profile,
            &parser.package_imports,
        )?
    };
    let key = key(schema, namespace, local)?.to_owned();
    let enumeration = schema
        .enumerations
        .get(&key)
        .ok_or("Missing imported enum")?;
    parser.require(":")?;
    let body_start = parser.pos;
    let mut enum_literals = BTreeMap::new();
    let mut items = Vec::new();
    loop {
        let literal_start = parser.pos;
        let name = parser.name()?;
        let literal = enumeration
            .by_name(&name)
            .ok_or_else(|| format!("Unknown enum literal {key}.{name}"))?;
        let spelling = if parser.eat("=") {
            let token = parser.take()?;
            if !token.quoted {
                return Err("Enum literal spelling requires a keyword".into());
            }
            token.text
        } else {
            name
        };
        if spelling.is_empty() {
            return Err("Empty enum keywords are unsupported".into());
        }
        let value = enumeration.value_by_id(&literal.declaration)?;
        if enum_literals.insert(spelling.clone(), value).is_some() {
            return Err(format!("Duplicate enum keyword {spelling}"));
        }
        items.push(Expr {
            kind: Kind::Literal(spelling),
            cardinality: '1',
            first_set_predicated: false,
            original_range: parser.original_range(literal_start),
            prediction_path: None,
        });
        if !parser.eat("|") {
            break;
        }
    }
    let body_range = parser.original_range(body_start);
    parser.require(";")?;
    Ok(Rule {
        hidden: None,
        enum_literals,
        nominal_return: None,
        inferred_return: false,
        override_declared: false,
        datatype: Some(Datatype::Enum),
        fragment: false,
        wildcard: false,
        class: key,
        body: Expr {
            kind: Kind::Alternative(items),
            cardinality: '1',
            first_set_predicated: false,
            original_range: body_range,
            prediction_path: None,
        },
    })
}

pub(super) fn check_assignment(
    schema: &Schema,
    class: &str,
    feature: &str,
    rule: &Rule,
    field: &intersect_model::definitions::schema::Feature,
) -> Result<(), String> {
    let target = schema.runtime_attribute_lexical_target(class, feature)?;
    if field.reference || target != Some(rule.class.as_str()) {
        return Err(format!(
            "Enum rule assignment type mismatch for {class}.{feature}"
        ));
    }
    let enumeration = schema
        .enumerations
        .get(&rule.class)
        .ok_or("Missing enum rule type")?;
    for value in rule.enum_literals.values() {
        enumeration.validate_value(value)?;
    }
    Ok(())
}
