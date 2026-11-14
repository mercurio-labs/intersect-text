//! Handwritten test corpus. Dynamic definitions and textual definitions are
//! authored independently; model data is varied deterministically, without clocks.
#![allow(dead_code)]
use intersect_model::core::{dynamic::DynamicModel, interchange::FormatRegistry, model::Model};
use intersect_model::definitions::{
    dynamic::{DynamicSchema, ScalarType, SchemaBuilder},
    schema::Schema,
};
use intersect_model::xmi::XmiFormat;
use intersect_text::distribution::ordinary::{self as distribution, Distribution, PackageMetadata};
use intersect_text::{Source, evaluate};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const ECORE: &str = include_str!("../../../../../fixtures/compatibility/graph.ecore");
pub const XTEXT: &str = include_str!("../../../../../fixtures/compatibility/graph.xtext");
pub const RCORE_BEHAVIOR: &str =
    include_str!("../../../../../fixtures/compatibility/graph-behavior.rcore");
pub fn distribution() -> Distribution {
    distribution_for_structure(&intersect_text_acceptance::migrate_model_fixture(ECORE).unwrap())
}
/// Exact generated Rcore input recovered from the pinned historical producer.
/// Re-running today's renderer creates a different input, not a wire-format change.
pub fn ordinary_baseline_distribution() -> Distribution {
    distribution_for_structure(include_str!(
        "../../../../../fixtures/xtext/inference-archive/ordinary-baseline-input.rcore"
    ))
}
fn distribution_for_structure(structure: &str) -> Distribution {
    distribution::build(
        structure,
        XTEXT,
        RCORE_BEHAVIOR,
        PackageMetadata {
            package_id: "compatibility.graph".into(),
            release: "1".into(),
            display_name: "Graph controls".into(),
            description: String::new(),
        },
    )
    .unwrap()
}
pub fn extracted(d: &Distribution) -> BTreeMap<String, String> {
    let mut files = d.files.clone();
    files.insert(
        "manifest.json".into(),
        serde_json::to_string(&d.manifest).unwrap(),
    );
    files
}
pub fn schema() -> DynamicSchema {
    let mut b = SchemaBuilder::new("graph", "urn:intersect:compatibility:graph");
    // Reverse declaration order relative to XML is intentional.
    let edge = b.class("Edge").unwrap();
    let node = b.class("Node").unwrap();
    let graph = b.class("Graph").unwrap();
    let named = b.class("Named").unwrap();
    named.abstract_type().unwrap();
    named
        .attribute("name", ScalarType::String)
        .required()
        .unwrap();
    for c in [&edge, &node, &graph] {
        c.extends(&named).unwrap();
    }
    let nodes = graph
        .reference("nodes", &node)
        .many()
        .containment()
        .unwrap();
    graph
        .reference("edges", &edge)
        .many()
        .containment()
        .unwrap();
    node.attribute("count", ScalarType::Integer)
        .default(7)
        .unwrap();
    node.attribute("active", ScalarType::Boolean)
        .default(true)
        .unwrap();
    node.attribute("labels", ScalarType::String)
        .many()
        .unique(false)
        .build()
        .unwrap();
    node.reference("peer", &node).optional().unwrap();
    node.reference("links", &node)
        .many()
        .unique(false)
        .build()
        .unwrap();
    let owner = node.reference("graph", &graph).optional().unwrap();
    b.opposites(&nodes, &owner).unwrap();
    edge.reference("source", &node).required().unwrap();
    edge.reference("target", &node).required().unwrap();
    edge.attribute("weight", ScalarType::Integer)
        .build()
        .unwrap();
    b.build().unwrap()
}
pub fn registry() -> FormatRegistry {
    let mut r = FormatRegistry::default();
    r.register(XmiFormat).unwrap();
    r
}

/// Compare every stored field and edge, retaining class/namespace and collection
/// order/multiplicity. IDs map bijectively through fixture-unique class/name keys.
/// Source offsets, resource spellings and revision digests are checked separately.
pub fn normalized(schema: &Schema, model: &Model) -> Value {
    model.validate(schema).unwrap();
    let mut ids = BTreeMap::new();
    let mut unique = std::collections::BTreeSet::new();
    for e in &model.elements {
        let key = format!(
            "{}#//{}:{}",
            schema.uri,
            e.class,
            e.values
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>")
        );
        assert!(unique.insert(key.clone()), "Nonbijective comparison key");
        assert!(ids.insert(e.id.clone(), key).is_none());
    }
    let mut objects = BTreeMap::new();
    for e in &model.elements {
        let mut fields = e.values.clone();
        for (name, f) in schema.features(&e.class).unwrap() {
            if !f.reference {
                continue;
            }
            let translate =
                |v: &Value| Value::String(ids.get(v.as_str().unwrap()).unwrap().clone());
            let value = &fields[&name];
            let value = if value.is_null() {
                Value::Null
            } else if f.upper == 1 {
                translate(value)
            } else {
                Value::Array(value.as_array().unwrap().iter().map(translate).collect())
            };
            fields.insert(name, value);
        }
        assert!(objects.insert(ids[&e.id].clone(), fields).is_none());
    }
    json!(objects)
}
pub struct Case {
    pub dynamic: DynamicModel,
    pub sources: Vec<Source>,
    pub text: Model,
}
pub fn case(seed: usize, d: &Distribution) -> Case {
    let dynamic_schema = schema();
    let mut structural = d.package.schema.classes.clone();
    for class in structural.values_mut() {
        assert_eq!(class.feature_order.len(), class.features.len());
        assert_eq!(class.feature_declarations.len(), class.features.len());
        for declaration in class.feature_declarations.values() {
            assert_eq!(
                declaration.source,
                d.package.schema.declarations[&class.name].source
            );
            assert_eq!(declaration.path.len(), 2);
        }
        class.feature_order.clear();
        class.feature_declarations.clear();
    }
    assert_eq!(dynamic_schema.as_schema().classes, structural);
    assert_eq!(dynamic_schema.as_schema().uri, d.package.schema.uri);
    assert_eq!(dynamic_schema.as_schema().name, d.package.schema.name);
    assert_ne!(
        dynamic_schema.as_schema().revision,
        d.package.schema.revision
    );
    let graph = dynamic_schema.class("Graph").unwrap();
    let node = dynamic_schema.class("Node").unwrap();
    let edge = dynamic_schema.class("Edge").unwrap();
    let name = graph.feature("name").unwrap();
    let n = 2 + seed % 15;
    let counts = [0, 1, 7, 2147483647];
    let labels = [
        "",
        "same",
        "same",
        "Café & <雪> \"quoted\"",
        "line\n\ttab\rreturn",
    ];
    let mut sources = vec![
        Source {
            uri: "catalog.graph".into(),
            text: "graph Catalog {\n".into(),
        },
        Source {
            uri: "archive.graph".into(),
            text: "graph Archive {\n".into(),
        },
    ];
    for i in 0..n {
        let mut line = format!("node n{i}");
        if (i + seed).is_multiple_of(2) {
            line.push_str(&format!(" count {}", counts[(seed + i) % counts.len()]));
        }
        for j in 0..(i + seed) % 6 {
            line.push_str(&format!(
                " label {}",
                serde_json::to_string(labels[j % labels.len()]).unwrap()
            ));
        }
        if i % 3 != 0 {
            line.push_str(&format!(" peer n{}", (i + seed + 1) % n));
        }
        for _ in 0..i % 3 {
            line.push_str(&format!(" link n{}", (i + 1) % n));
        }
        sources[i % 2].text.push_str(&(line + ";\n"));
    }
    // Cross-resource forward references and cycles; each graph has an edge.
    for (i, source) in sources.iter_mut().enumerate() {
        source.text.push_str(&format!(
            "edge e{i} n{} to n{} weight {seed};\n}}",
            i % n,
            (i + seed + 1) % n
        ));
    }
    let mut dynamic = DynamicModel::new(dynamic_schema).unwrap();
    dynamic
        .transaction(|tx| {
            let graphs = [
                tx.create(&graph, "root0", "catalog.graph")?,
                tx.create(&graph, "root1", "archive.graph")?,
            ];
            tx.set(&graphs[0], &name, "Catalog")?;
            tx.set(&graphs[1], &name, "Archive")?;
            let mut nodes = vec![];
            for i in 0..n {
                let object = tx.create(&node, &format!("node{i}"), &sources[i % 2].uri)?;
                tx.set(&object, &name, format!("n{i}"))?;
                if (i + seed).is_multiple_of(2) {
                    tx.set(
                        &object,
                        &node.feature("count")?,
                        counts[(seed + i) % counts.len()],
                    )?;
                }
                let values: Vec<_> = (0..(i + seed) % 6)
                    .map(|j| labels[j % labels.len()])
                    .collect();
                tx.set(&object, &node.feature("labels")?, json!(values))?;
                tx.add(&graphs[i % 2], &graph.feature("nodes")?, &object)?;
                nodes.push(object);
            }
            for (i, object) in nodes.iter().enumerate() {
                if i % 3 != 0 {
                    tx.set(object, &node.feature("peer")?, &nodes[(i + seed + 1) % n])?;
                }
                for _ in 0..i % 3 {
                    tx.add(object, &node.feature("links")?, &nodes[(i + 1) % n])?;
                }
            }
            for i in 0..2 {
                let e = tx.create(&edge, &format!("edge{i}"), &sources[i].uri)?;
                tx.set(&e, &name, format!("e{i}"))?;
                tx.set(&e, &edge.feature("source")?, &nodes[i % n])?;
                tx.set(&e, &edge.feature("target")?, &nodes[(i + seed + 1) % n])?;
                tx.set(&e, &edge.feature("weight")?, seed as i32)?;
                tx.add(&graphs[i], &graph.feature("edges")?, &e)?;
            }
            Ok(())
        })
        .unwrap();
    let result = evaluate(&d.package, &sources);
    assert!(
        result.diagnostics.is_empty(),
        "seed {seed}: {:?}",
        result.diagnostics
    );
    let text = result.model;
    assert_eq!(
        normalized(dynamic.schema().as_schema(), &dynamic.to_model().unwrap()),
        normalized(&d.package.schema, &text),
        "seed {seed}"
    );
    assert_eq!(text.elements.len(), n + 4);
    Case {
        dynamic,
        sources,
        text,
    }
}
