//! Portable independent observations for the bounded F17–F19 resource lifecycle.
use intersect_text::package::Package;
mod f20;
#[cfg(not(target_arch = "wasm32"))]
pub use f20::measure_cache;
use intersect_model::{
    api::{BasicModel, ModelEdit},
    core::{
        frozen::FrozenStore,
        interchange::{FormatRegistry, ReferenceClosure},
        model::Model,
        resource_set::{ResourceContext, ResourceSet, TargetResolution},
        resources::{ResourceLoadState, ResourceRecord},
        storage::{self, ModelRead},
    },
    definitions::schema::Schema,
};
use intersect_text::compiler;
use intersect_text::{Source, evaluate};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn handle_id(
    store: &FrozenStore,
    handle: &intersect_model::storage::ObjectHandle,
) -> Result<String, String> {
    let mut id = String::new();
    store.inspect_handle(handle, &mut |object| {
        object.id().clone_into(&mut id);
        Ok(())
    })?;
    Ok(id)
}

fn require(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.into()) }
}
fn package() -> Result<Package, String> {
    compiler::compile_rcore_language(
        include_str!("../../../fixtures/library/library.rcore"),
        include_str!("../../../fixtures/library/library.xtext"),
        include_str!("../../../fixtures/library/library-behavior.rcore"),
    )
    .map_err(|errors| format!("Library compilation: {errors:?}"))
}
fn make_context(schema: &Schema) -> Result<ResourceContext, String> {
    ResourceContext::new(
        schema,
        BTreeMap::from([("library-policy".into(), "resource-test-v1".into())]),
    )
}
fn class_key(schema: &Schema, name: &str) -> Result<String, String> {
    schema
        .declarations
        .iter()
        .find(|(key, declaration)| declaration.name == name && schema.classes.contains_key(*key))
        .map(|(key, _)| key.clone())
        .ok_or_else(|| format!("Unknown fixture class {name}"))
}
fn part(model: &Model, uri: &str) -> Model {
    Model {
        schema_revision: model.schema_revision.clone(),
        resources: model
            .resources
            .iter()
            .filter(|(id, _)| id.as_str() == uri)
            .map(|(id, r)| (id.clone(), r.clone()))
            .collect(),
        elements: model
            .elements
            .iter()
            .filter(|e| e.resource == uri)
            .cloned()
            .collect(),
    }
}
fn observed(model: &Model, schema: &Schema) -> Result<BTreeMap<String, Value>, String> {
    let names: BTreeMap<_, _> = model
        .elements
        .iter()
        .map(|e| {
            (
                e.id.as_str(),
                format!("{}:{}", e.class, e.values["name"].as_str().unwrap_or("")),
            )
        })
        .collect();
    let mut result = BTreeMap::new();
    for e in &model.elements {
        let mut fields = e.values.clone();
        for (name, field) in schema.features(&e.class)? {
            if !field.reference || field.derived {
                continue;
            }
            let convert = |value: &Value| -> Result<Value, String> {
                if value.is_null() {
                    return Ok(Value::Null);
                }
                let target = value
                    .as_str()
                    .and_then(|id| names.get(id))
                    .ok_or("Unobserved reference target")?;
                Ok(json!(target))
            };
            let value = fields.get(&name).ok_or("Missing expected stored field")?;
            let normalized = if let Some(list) = value.as_array() {
                Value::Array(list.iter().map(convert).collect::<Result<_, _>>()?)
            } else {
                convert(value)?
            };
            fields.insert(name, normalized);
        }
        result.insert(
            names[&e.id.as_str()].clone(),
            json!({"resource": e.resource, "values": fields}),
        );
    }
    Ok(result)
}
fn build_direct(schema: &Schema) -> Result<Model, String> {
    let mut model = BasicModel::new(schema.clone())?;
    for (id, class, resource, name) in [
        ("catalog", "Library", "catalog.library", "Catalog"),
        ("dune", "BookCopy", "catalog.library", "dune"),
        ("earthsea", "BookCopy", "catalog.library", "earthsea"),
        ("ada", "Member", "catalog.library", "ada"),
        ("circulation", "Library", "loans.library", "Circulation"),
        ("first", "Loan", "loans.library", "first"),
    ] {
        model.create(id, class, resource)?;
        model.set(id, "name", json!(name))?;
    }
    model.set("dune", "title", json!("Dune"))?;
    model.set("earthsea", "title", json!("A Wizard of Earthsea"))?;
    model.set("earthsea", "edition", json!(2))?;
    model.set("catalog", "copies", json!(["dune", "earthsea"]))?;
    model.set("catalog", "members", json!(["ada"]))?;
    model.set("circulation", "loans", json!(["first"]))?;
    model.set("first", "copy", json!("dune"))?;
    model.set("first", "member", json!("ada"))?;
    model.validate()?;
    storage::export(&model)
}

fn verify_f17_for<S: stores::StoreFixture>() -> Result<(), String> {
    let package = intersect_model::compile_rcore(include_str!(
        "../../../fixtures/rcore/api/organization.rcore"
    ))
    .map_err(|e| e.to_string())?;
    let schema = package.schema().clone();
    let node = class_key(&schema, "Node")?;
    let mut set = stores::resources::<S>(schema.clone(), make_context(&schema)?)?;
    set.create_resource("empty.xmi")?;
    set.create_resource("nodes.xmi")?;
    require(
        set.create_resource("empty.xmi").is_err(),
        "Duplicate URI accepted",
    )?;
    set.transaction(|tx| {
        tx.create("A", &node, "nodes.xmi")?;
        tx.create("B", &node, "nodes.xmi")
    })?;
    set.reorder_roots("nodes.xmi", &["B".into(), "A".into()])?;
    let before = set.snapshot()?;
    require(
        before.resources["empty.xmi"].roots.is_empty(),
        "Empty resource lost",
    )?;
    require(
        before.resources["nodes.xmi"].roots.as_slice() == ["B", "A"],
        "Explicit root ordering lost",
    )?;
    let rejected = set.transaction(|tx| {
        tx.set("A", "name", json!("changed"))?;
        tx.set("A", "links", json!(["missing"]))
    });
    require(
        rejected.is_err() && set.snapshot()? == before,
        "Failed transaction changed resources or content",
    )?;
    let rejected = set.reorder_roots("nodes.xmi", &["A".into(), "A".into()]);
    require(
        rejected.is_err() && set.snapshot()? == before,
        "Invalid root reorder changed model",
    )?;
    set.transaction(|tx| tx.set("A", "name", json!("changed")))?;
    require(
        set.resources()?["nodes.xmi"].content_revision
            > before.resources["nodes.xmi"].content_revision,
        "Content edit did not advance resource revision",
    )?;
    require(
        set.resources()?["empty.xmi"] == before.resources["empty.xmi"],
        "Unrelated empty resource changed",
    )?;
    let mut annotated = set.snapshot()?;
    let record = annotated
        .resources
        .get_mut("nodes.xmi")
        .ok_or("Missing nodes resource")?;
    record.source = Some(intersect_model::resources::SourceCorrespondence {
        uri: "nodes.source".into(),
        revision: "unchanged-source".into(),
    });
    record
        .diagnostics
        .push("Retained current-source diagnostic".into());
    let mut idempotent =
        stores::resources_from_model::<S>(schema.clone(), make_context(&schema)?, &annotated)?;
    idempotent.transaction(|tx| tx.set("A", "name", json!("changed")))?;
    require(
        idempotent.snapshot()? == annotated,
        "Identical assignment invalidated resource/source state",
    )?;
    let mut raw = stores::store::<S>(&schema)?;
    storage::atomic(&mut raw, |store| {
        store.set_revision(&annotated.schema_revision)?;
        for element in &annotated.elements {
            store.put(element.clone())?;
        }
        for record in annotated.resources.values() {
            store.set_resource_record(record.clone())?;
        }
        Ok(())
    })?;
    require(
        storage::export(&raw)? == annotated,
        "Raw fixture lost supplied metadata",
    )?;
    storage::atomic(&mut raw, |store| store.configure_schema(&schema))?;
    require(
        storage::export(&raw)? == annotated,
        "Schema binding changed explicit root order or source correspondence",
    )?;
    verify_populated_resource_defaults::<S>()
        .map_err(|error| format!("Populated defaults: {error}"))
}

fn verify_populated_resource_defaults<S: stores::StoreFixture>() -> Result<(), String> {
    let mut schema = package()?.schema;
    let library = class_key(&schema, "Library")?;
    let member = class_key(&schema, "Member")?;
    schema
        .classes
        .get_mut(&library)
        .ok_or("Missing Library class")?
        .features
        .get_mut("members")
        .ok_or("Missing members feature")?
        .default = json!(["reader"]);
    let mut model = stores::basic::<S>(schema.clone())?;
    model
        .create("reader", &member, "defaults.library")
        .map_err(|error| format!("Create reader: {error}"))?;
    model
        .create("library", &library, "defaults.library")
        .map_err(|error| format!("Create library: {error}"))?;
    let state = storage::export(&model)?;
    require(
        state.resources["defaults.library"].roots.as_slice() == ["library"],
        "Populated containment default left its child as a resource root",
    )?;
    require(
        model
            .create("branch", &library, "defaults.library")
            .is_err()
            && storage::export(&model)? == state,
        "Conflicting default owner changed published resources",
    )?;
    let mut direct = Model::default();
    direct.create(&schema, "reader", &member, "defaults.library", 0, 0)?;
    direct.create(&schema, "library", &library, "defaults.library", 0, 0)?;
    require(
        direct.resources["defaults.library"].roots.as_slice() == ["library"],
        "Direct materialization lost populated containment roots",
    )
}

fn verify_imported_structural_resources() -> Result<(), String> {
    use intersect_model::api::BasicModel;
    use intersect_model::core::{
        query::QueryBudget,
        storage::{
            ContainmentDestination, DeleteRequest, DeleteSelection, IncomingReferences,
            ReparentRequest, StructuralContext,
        },
    };
    let schema = package()?.schema;
    let library = class_key(&schema, "Library")?;
    let member = class_key(&schema, "Member")?;
    let mut model = BasicModel::new(schema.clone())?;
    model.create("owner", &library, "catalog.library")?;
    model.create("child", &member, "script.py")?;
    model.create("branch", &library, "script.py")?;
    let before = storage::export(&model)?;
    // Observe semantic roots/identity here; native controls separately freeze budget rollback.
    let context = StructuralContext {
        schema: &schema,
        conversions: None,
    };
    let mut store = FrozenStore::from_materialized(&schema, &before)?;
    require(
        store.metamodel().schema() == &schema,
        "Reparent control lost the imported schema",
    )?;
    require(
        context.reparent(
            &mut store,
            ReparentRequest {
                object: "child",
                destination: Some(ContainmentDestination {
                    owner: "owner",
                    feature: "members",
                    position: Some(0),
                }),
            },
            &mut QueryBudget::new(usize::MAX),
        )?,
        "Cross-resource reparent did not apply",
    )?;
    require(
        store
            .resource_record("script.py")?
            .ok_or("Missing script resource")?
            .roots
            .as_slice()
            == ["branch"],
        "Reparent left a foreign contained child in resource roots",
    )?;
    context.reparent(
        &mut store,
        ReparentRequest {
            object: "child",
            destination: Some(ContainmentDestination {
                owner: "branch",
                feature: "members",
                position: None,
            }),
        },
        &mut QueryBudget::new(usize::MAX),
    )?;
    // Deletion also starts from a separately imported snapshot.
    let mut store = FrozenStore::from_materialized(&schema, &storage::export(&store)?)?;
    require(
        store.metamodel().schema() == &schema,
        "Delete control lost the imported schema",
    )?;
    require(
        context.delete(
            &mut store,
            DeleteRequest {
                roots: &["branch"],
                selection: DeleteSelection::Subtrees,
                incoming: IncomingReferences::Reject,
            },
            &mut QueryBudget::new(usize::MAX),
        )? == ["branch", "child"],
        "Imported subtree delete removed incorrect identities",
    )?;
    require(
        store
            .resource_record("script.py")?
            .ok_or("Missing empty script")?
            .roots
            .is_empty()
            && store.get("child")?.is_none()
            && store.get("branch")?.is_none(),
        "Deletion lost empty resource state or retained deleted objects",
    )?;
    require(
        store.get("owner")? == before.elements.iter().find(|e| e.id == "owner").cloned(),
        "Structural operations changed retained owner identity or values",
    )?;
    storage::validate_draft(&store, &schema)
}

pub fn verify_f18() -> Result<(), String> {
    let package = package()?;
    let direct = build_direct(&package.schema)?;
    let textual = evaluate(
        &package,
        &[
            Source {
                uri: "catalog.library".into(),
                text: include_str!("../../../fixtures/library/catalog.library").into(),
            },
            Source {
                uri: "loans.library".into(),
                text: include_str!("../../../fixtures/library/loans.library").into(),
            },
        ],
    );
    require(
        textual.diagnostics.is_empty(),
        &format!("Text diagnostics: {:?}", textual.diagnostics),
    )?;
    require(
        observed(&direct, &package.schema)? == observed(&textual.model, &package.schema)?,
        "Text and direct Library observations differ",
    )?;
    let context = make_context(&package.schema)?;
    let mut set = stores::resources_from_model::<FrozenStore>(
        package.schema.clone(),
        context.clone(),
        &direct,
    )?;
    let handle = set
        .reader()
        .handle("dune")
        .ok_or("Missing retained handle")?;
    set.bind_fragment("catalog.library", "book", "dune")?;
    require(
        matches!(set.resolve("loans.library", "catalog.library#book")?, TargetResolution::Resolved { element, .. } if element == "dune"),
        "Cross-resource fragment resolution failed",
    )?;
    require(
        matches!(
            set.resolve("loans.library", "missing.library#book")?,
            TargetResolution::MissingResource(_)
        ),
        "Missing resource was fabricated",
    )?;
    require(
        matches!(
            set.resolve("loans.library", "catalog.library#missing")?,
            TargetResolution::MissingFragment { .. }
        ),
        "Unknown fragment resolved",
    )?;
    set.bind_fragment("catalog.library", "book", "earthsea")?;
    require(
        matches!(set.resolve("loans.library", "catalog.library#book")?, TargetResolution::Ambiguous { candidates, .. } if candidates == ["dune", "earthsea"]),
        "Ambiguous lookup selected a target",
    )?;
    let selection = vec!["loans.library".into()];
    let mut replacement = part(&direct, "loans.library");
    replacement
        .elements
        .iter_mut()
        .find(|e| e.id == "first")
        .ok_or("Missing loan")?
        .values
        .insert("name".into(), json!("renewed"));
    set.replace(&selection, replacement.clone(), &context)?;
    require(
        handle_id(set.reader(), &handle)? == "dune",
        "Replacement invalidated retained handle",
    )?;
    require(
        set.reader().value("first", "name")? == Some(json!("renewed")),
        "Selected replacement did not publish",
    )?;
    let before = set.snapshot()?;
    let mut duplicate = part(&before, "loans.library");
    duplicate
        .elements
        .iter_mut()
        .find(|element| element.id == "circulation")
        .ok_or("Missing selected Library root")?
        .id = "dune".into();
    duplicate
        .resources
        .get_mut("loans.library")
        .ok_or("Missing selected resource record")?
        .roots = std::sync::Arc::new(vec!["dune".into()]);
    require(
        matches!(set.replace(&selection, duplicate, &context), Err(error) if error.to_ascii_lowercase().contains("duplicate"))
            && set.snapshot()? == before
            && handle_id(set.reader(), &handle)? == "dune",
        "Duplicate global identity crossed the resource boundary or changed the published graph",
    )?;
    replacement
        .elements
        .iter_mut()
        .find(|e| e.id == "first")
        .ok_or("Missing loan")?
        .values
        .insert("copy".into(), json!("missing"));
    require(
        set.replace(&selection, replacement, &context).is_err() && set.snapshot()? == before,
        "Dangling replacement changed published graph",
    )?;
    require(
        set.replace_with(&selection, part(&before, "loans.library"), &context, |_| {
            Err("language rejection".into())
        })
        .is_err()
            && set.snapshot()? == before,
        "Language validation did not roll back",
    )?;
    let incompatible = ResourceContext::new(
        &package.schema,
        BTreeMap::from([("library-policy".into(), "v2".into())]),
    )?;
    require(
        set.replace(&selection, part(&before, "loans.library"), &incompatible)
            .is_err(),
        "Incompatible dependency context accepted",
    )?;
    let mut invalid_context_wire =
        serde_json::to_value(&context).map_err(|error| error.to_string())?;
    invalid_context_wire["dependencies"] = json!({"": ""});
    let invalid_context: ResourceContext =
        serde_json::from_value(invalid_context_wire).map_err(|error| error.to_string())?;
    require(
        matches!(ResourceSet::new(package.schema.clone(), invalid_context), Err(error) if error.contains("nonempty schema/dependency identities")),
        "Deserialized context bypassed nonempty dependency identity admission",
    )?;
    let mut different_schema = package.schema.clone();
    different_schema.uri.push_str(":different-schema-digest");
    require(
        different_schema.revision == package.schema.revision,
        "Schema digest fixture accidentally changed its revision",
    )?;
    let mismatched_digest =
        ResourceContext::new(&different_schema, context.dependencies().clone())?;
    require(
        matches!(ResourceSet::from_model(package.schema.clone(), mismatched_digest.clone(), &before), Err(error) if error.contains("Incompatible resource schema context")),
        "Same-revision context with a different schema digest was accepted",
    )?;
    require(
        set.replace(
            &selection,
            part(&before, "loans.library"),
            &mismatched_digest,
        )
        .is_err()
            && set.snapshot()? == before,
        "Mismatched schema digest replacement changed the published graph",
    )?;
    let mut availability = before.clone();
    for (uri, state) in [
        ("unloaded.library", ResourceLoadState::Unloaded),
        ("failed.library", ResourceLoadState::Failed),
    ] {
        let mut record = ResourceRecord::new(uri)?;
        record.load_state = state;
        availability.resources.insert(uri.into(), record);
    }
    let unavailable =
        ResourceSet::from_model(package.schema.clone(), context.clone(), &availability)?;
    let mut availability_formats = FormatRegistry::default();
    availability_formats.register(intersect_model::xmi::XmiFormat)?;
    for (uri, expected_state) in [
        ("unloaded.library", ResourceLoadState::Unloaded),
        ("failed.library", ResourceLoadState::Failed),
    ] {
        require(
            matches!(unavailable.resolve("loans.library", &format!("{uri}#book"))?, TargetResolution::UnavailableResource { resource, state } if resource == uri && state == expected_state),
            "Unavailable resource lookup became a missing fragment or a resolved target",
        )?;
        for closure in [
            ReferenceClosure::SelectedOnly,
            ReferenceClosure::IncludeDependencies,
        ] {
            require(
                matches!(availability_formats.export_selected(
                    "xmi", &package.schema, &availability, &[uri.into()], closure,
                ), Err(error) if error.contains("Selected resource is unavailable")),
                "Direct registry selected export serialized unavailable resource content",
            )?;
        }
    }
    // Explicit bounded KerML specialization projection, not full KerML qualification.
    let kerml = intersect_model::compile_rcore(include_str!(
        "../../../fixtures/rcore/day04/kerml-specialization.rcore"
    ))
    .map_err(|e| e.to_string())?;
    let schema = kerml.schema().clone();
    let context = make_context(&schema)?;
    let type_key = class_key(&schema, "Type")?;
    let specialization_key = class_key(&schema, "Specialization")?;
    let mut kerml = stores::resources::<FrozenStore>(schema, context.clone())?;
    kerml.transaction(|tx| {
        tx.create("general", &type_key, "library.kerml")?;
        tx.create("alternate", &type_key, "library.kerml")?;
        tx.create("specific", &type_key, "user.kerml")?;
        tx.create("specialization", &specialization_key, "user.kerml")?;
        tx.set("specialization", "general", json!("general"))?;
        tx.set("specialization", "specific", json!("specific"))
    })?;
    let handle = kerml.reader().handle("general").ok_or("Missing general")?;
    let initial_kerml = kerml.snapshot()?;
    let retained_library = part(&initial_kerml, "library.kerml");
    let mut user = part(&initial_kerml, "user.kerml");
    user.elements
        .iter_mut()
        .find(|element| element.id == "specialization")
        .ok_or("Missing specialization")?
        .values
        .insert("general".into(), json!("alternate"));
    kerml.replace(&["user.kerml".into()], user, &context)?;
    let after_positive = kerml.snapshot()?;
    require(
        kerml.reader().value("specialization", "general")? == Some(json!("alternate"))
            && kerml.reader().value("specialization", "specific")? == Some(json!("specific"))
            && after_positive.resources["user.kerml"].content_revision
                > initial_kerml.resources["user.kerml"].content_revision,
        "Valid KerML selected replacement did not publish its independently expected relation",
    )?;
    require(
        part(&after_positive, "library.kerml") == retained_library
            && handle_id(kerml.reader(), &handle)? == "general",
        "Successful KerML replacement disturbed retained library content, metadata or handles",
    )?;
    let mut user = part(&after_positive, "user.kerml");
    user.elements
        .iter_mut()
        .find(|e| e.id == "specialization")
        .ok_or("Missing specialization")?
        .values
        .insert("general".into(), json!("missing"));
    require(
        kerml
            .replace(&["user.kerml".into()], user, &context)
            .is_err()
            && kerml.snapshot()? == after_positive,
        "Invalid KerML projection replacement accepted or changed the published graph",
    )?;
    require(
        handle_id(kerml.reader(), &handle)? == "general",
        "Failed replacement disturbed KerML library",
    )
}

pub fn verify_f19() -> Result<(), String> {
    let package = package()?;
    let model = build_direct(&package.schema)?;
    let context = make_context(&package.schema)?;
    let mut set = stores::resources_from_model::<FrozenStore>(
        package.schema.clone(),
        context.clone(),
        &model,
    )?;
    set.create_resource("empty.library")?;
    let mut registry = FormatRegistry::default();
    registry.register(intersect_model::xmi::XmiFormat)?;
    let selection = vec!["loans.library".into()];
    let ticket = set.prepare_save(&registry, "xmi", &selection, ReferenceClosure::SelectedOnly)?;
    require(
        ticket.resources().len() == 1 && ticket.resources()[0].uri == "loans.library",
        "Selected export included unrelated resources",
    )?;
    set.transaction(|tx| tx.set("first", "name", json!("edited after save started")))?;
    require(
        set.acknowledge_save(&registry, &ticket).is_err()
            && set.is_dirty(&registry, "loans.library")?,
        "Stale save marked newer content clean",
    )?;
    let ticket = set.prepare_save(&registry, "xmi", &selection, ReferenceClosure::SelectedOnly)?;
    set.acknowledge_save(&registry, &ticket)?;
    require(
        !set.is_dirty(&registry, "loans.library")?,
        "Successful exact save remained dirty",
    )?;
    set.transaction(|tx| tx.set("dune", "title", json!("Revised Dune")))?;
    require(
        !set.is_dirty(&registry, "loans.library")? && set.is_dirty(&registry, "catalog.library")?,
        "Unrelated edit changed saved loan state",
    )?;
    let retained = set.reader().handle("dune").ok_or("Missing retained copy")?;
    set.import_with(
        &registry,
        "xmi",
        &selection,
        ticket.resources(),
        &context,
        |_| Ok(()),
    )?;
    require(
        handle_id(set.reader(), &retained)? == "dune",
        "Contextual import replaced catalog",
    )?;
    let all = vec![
        "catalog.library".into(),
        "loans.library".into(),
        "empty.library".into(),
    ];
    let ticket = set.prepare_save(&registry, "xmi", &all, ReferenceClosure::SelectedOnly)?;
    let roundtrip = registry.import("xmi", &package.schema, ticket.resources())?;
    require(
        roundtrip == set.snapshot()?,
        "XMI lost resource boundaries, order, identities or provenance",
    )?;
    let closure = set.prepare_save(
        &registry,
        "xmi",
        &selection,
        ReferenceClosure::IncludeDependencies,
    )?;
    require(
        closure.resources().len() == 2,
        "Explicit dependency closure omitted catalog or included unrelated empty resource",
    )?;
    let before = set.snapshot()?;
    let mut corrupt = ticket.resources().to_vec();
    corrupt[0].bytes = b"<unsupported/>".to_vec();
    require(
        set.import_with(&registry, "xmi", &all, &corrupt, &context, |_| Ok(()))
            .is_err()
            && set.snapshot()? == before,
        "Invalid XMI changed published model",
    )?;
    let mut other = ResourceSet::from_model(package.schema.clone(), context.clone(), &model)?;
    require(
        other
            .prepare_save(&registry, "xmi", &selection, ReferenceClosure::SelectedOnly)
            .is_ok(),
        "Second resource set failed",
    )?;
    require(
        other.acknowledge_save(&registry, &ticket).is_err(),
        "Foreign save ticket accepted",
    )?;
    // Moving a referenced global identity changes hrefs in untouched resources.
    set.create_resource("other.library")?;
    let old = set.prepare_save(&registry, "xmi", &selection, ReferenceClosure::SelectedOnly)?;
    set.acknowledge_save(&registry, &old)?;
    let mut moved = set.snapshot()?;
    moved.elements.retain(|e| e.resource != "loans.library");
    moved.resources.remove("loans.library");
    moved.resources.remove("empty.library");
    let copy = moved
        .elements
        .iter_mut()
        .find(|e| e.id == "dune")
        .ok_or("Missing moved copy")?;
    copy.resource = "other.library".into();
    copy.values.insert("library".into(), Value::Null);
    moved
        .elements
        .iter_mut()
        .find(|e| e.id == "catalog")
        .ok_or("Missing catalog")?
        .values
        .insert("copies".into(), json!(["earthsea"]));
    moved
        .resources
        .get_mut("other.library")
        .ok_or("Missing destination resource")?
        .roots = std::sync::Arc::new(vec!["dune".into()]);
    set.replace(
        &["catalog.library".into(), "other.library".into()],
        moved,
        &context,
    )?;
    require(
        set.acknowledge_save(&registry, &old).is_err()
            && set.is_dirty(&registry, "loans.library")?,
        "Moved target did not invalidate serialization context",
    )
}

pub fn verify_f17() -> Result<(), String> {
    verify_f17_for::<FrozenStore>()?;
    verify_imported_structural_resources()
}

#[unsafe(no_mangle)]
pub extern "C" fn verify_resource_f17() -> u32 {
    u32::from(verify_f17().is_err())
}
#[unsafe(no_mangle)]
pub extern "C" fn verify_resource_f18() -> u32 {
    u32::from(verify_f18().is_err())
}
#[unsafe(no_mangle)]
pub extern "C" fn verify_resource_f19() -> u32 {
    u32::from(verify_f19().is_err())
}
mod stores {
    #[allow(unused_imports)]
    pub use intersect_model_test_support::stores::*;
    #[test]
    fn frozen_store_starts_empty_with_the_requested_schema() -> Result<(), String> {
        intersect_model_test_support::stores::verify_frozen_store_starts_empty_with_the_requested_schema()
    }
}
#[cfg(test)]
mod tests {
    mod memory {
        #[test]
        fn f17_resource_records() -> Result<(), String> {
            super::super::verify_f17()
        }
        #[test]
        fn f18_closed_replacement() -> Result<(), String> {
            super::super::verify_f18()
        }
        #[test]
        fn f19_resource_persistence() -> Result<(), String> {
            super::super::verify_f19()
        }
    }
    mod frozen {
        use intersect_model::core::frozen::FrozenStore as Store;
        #[test]
        fn nested_rollback_restores_each_touched_resource() -> Result<(), String> {
            use super::super::stores;
            use intersect_model::core::{
                resources::ResourceRecord,
                storage::{ModelRead, ModelStore, atomic},
            };

            let schema = stores::empty_schema()?;
            let mut store = stores::store::<Store>(&schema)?;
            atomic(&mut store, |store| {
                store.set_resource_record(ResourceRecord::new("first")?)?;
                store.set_resource_record(ResourceRecord::new("second")?)
            })?;
            let original = store.resource_records()?;
            let outer = store.begin()?;
            let mut first = ResourceRecord::new("first")?;
            first.diagnostics.push("outer".into());
            store.set_resource_record(first.clone())?;
            atomic(&mut store, |store| {
                first.diagnostics.push("inner".into());
                store.set_resource_record(first)?;
                store.remove_resource_record("second")?;
                store.set_resource_record(ResourceRecord::new("third")?)
            })?;
            assert!(store.resource_record("second")?.is_none());
            assert!(store.resource_record("third")?.is_some());
            store.rollback(&outer)?;
            assert_eq!(store.resource_records()?, original);
            Ok(())
        }
    }
}
