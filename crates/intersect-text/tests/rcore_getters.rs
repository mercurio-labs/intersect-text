use intersect_model::api::ModelEdit;
use intersect_model::core::query::QueryBudget;
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_model::definitions::rcore_behavior::ir_v2::Limits;
use intersect_model::rcore_behavior_functions::ExecutionLimits;
use intersect_text::extensions::RcoreGetters;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, Package};
use serde_json::json;

const SOURCE: &str = r#"rcore 1; package language.getters version "1";
metamodel "urn:language-getters" prefix m {
  class Item { attr raw: String [0,1]; attr label: String [0,1] derived; }
}
behavior {
  import model "urn:language-getters" as m;
  derive m.Item.label(self) -> Option<String> = label(self);
  fn label(self:m.Item) -> Option<String> = match self.raw {
    Some(raw) => if raw == "A" { Some("source") } else { Some(raw) }, None => None
  };
}"#;

fn package() -> Result<Package, String> {
    package_for(SOURCE)
}
fn package_for(source: &str) -> Result<Package, String> {
    let model = DefinitionSource::new("memory:language-getters.rcore", SourceKind::Rcore, source);
    let mut grammar = DefinitionSource::new(
        "memory:language-getters.xtext",
        SourceKind::Xtext,
        "grammar language.Getters import \"urn:language-getters\" as m import \"http://www.eclipse.org/emf/2002/Ecore\" as ecore Item returns m::Item: 'item' raw=ID; terminal ID returns ecore::EString: ('a'..'z'|'A'..'Z')+;",
    );
    grammar.dependencies.push(model.id.clone());
    intersect_text::ordinary::compile(
        BundleCompilation {
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![model.clone(), grammar.clone()],
                namespaces: vec![],
            },
            ecore: model.id.clone(),
            namespace: "urn:language-getters".into(),
            xtext: Some(grammar.id),
            rcore_behavior: model.id,
            materialized_types: vec![],
        },
        Profile::NativeV67,
    )
    .map_err(|e| format!("{e:?}"))
}
fn getters(package: &Package) -> Result<RcoreGetters<'_>, String> {
    RcoreGetters::from_package(
        package,
        &Limits::default(),
        ExecutionLimits::default(),
        ParseLimits::default(),
    )
}

#[test]
fn retained_package_composes_source_getters_over_live_reads() -> Result<(), String> {
    let source=SOURCE
        .replace("attr label: String [0,1] derived;", "attr canonical: String [0,1] derived; attr label: String [0,1] derived;")
        .replace("  derive m.Item.label", "  derive m.Item.canonical(self) -> Option<String> = canonical(self);\n  fn canonical(self:m.Item) -> Option<String> = self.raw;\n  derive m.Item.label")
        .replace("match self.raw", "match self.canonical");
    let package = package_for(&source)?;
    let service = getters(&package)?;
    let mut model = intersect_model::api::BasicModel::new(package.schema.clone())?;
    model.create("item", "Item", "memory:objects")?;
    model.set("item", "raw", json!("A"))?;
    let mut measured = QueryBudget::new(1_000_000);
    assert_eq!(
        service.read_model(&model, "item", "label", &mut measured)?,
        json!("source")
    );
    let mut exact = QueryBudget::new(7 + measured.reads);
    exact.charge_many(7)?;
    assert_eq!(
        service.read_model(&model, "item", "label", &mut exact)?,
        json!("source")
    );
    assert_eq!(exact.remaining(), 0);
    model.set("item", "raw", json!("changed"))?;
    assert_eq!(
        service.read_model(&model, "item", "label", &mut QueryBudget::new(1_000_000))?,
        json!("changed")
    );
    Ok(())
}

#[test]
fn retained_language_package_keeps_reference_getter_occurrences_and_source_types()
-> Result<(), String> {
    let source = DefinitionSource::new(
        "memory:reference-getters.rcore",
        SourceKind::Rcore,
        include_str!("../../../fixtures/rcore-getter-service/references.rcore"),
    );
    let mut grammar = DefinitionSource::new(
        "memory:reference-getters.xtext",
        SourceKind::Xtext,
        "grammar reference.Getters import \"urn:reference-getters\" as m Node returns m::Node: 'node';",
    );
    grammar.dependencies.push(source.id.clone());
    let package = intersect_text::ordinary::compile(
        BundleCompilation {
            bundle: DefinitionBundle {
                version: 1,
                sources: vec![source.clone(), grammar.clone()],
                namespaces: vec![],
            },
            ecore: source.id.clone(),
            namespace: "urn:reference-getters".into(),
            xtext: Some(grammar.id),
            rcore_behavior: source.id,
            materialized_types: vec![],
        },
        Profile::NativeV67,
    )
    .map_err(|e| format!("{e:?}"))?;
    let service = getters(&package)?;
    let mut model = intersect_model::api::BasicModel::new(package.schema.clone())?;
    model.create("a", "Node", "memory:objects")?;
    model.create("b", "Child", "memory:objects")?;
    model.set("a", "repeats", json!(["b", "a", "b"]))?;
    let mut work = QueryBudget::new(10_000_000);
    assert_eq!(
        service.read_model(&model, "a", "required", &mut work)?,
        json!("a")
    );
    assert_eq!(
        service.read_model(&model, "a", "occurrences", &mut work)?,
        json!(["b", "a", "b"])
    );
    let mut changed = package.clone();
    changed
        .schema
        .classes
        .get_mut("Node")
        .ok_or("Missing Node")?
        .features
        .get_mut("occurrences")
        .ok_or("Missing occurrences")?
        .unique = true;
    assert!(getters(&changed).is_err());
    Ok(())
}

#[test]
fn retained_language_package_executes_source_getters_on_current_model() -> Result<(), String> {
    let package = package()?;
    let getters = getters(&package)?;
    let mut model = intersect_model::api::BasicModel::new(package.schema.clone())?;
    model.create("item", "Item", "memory:objects")?;
    model.set("item", "raw", json!("A"))?;
    let mut measured = QueryBudget::new(1_000_000);
    assert_eq!(
        getters.read_model(&model, "item", "label", &mut measured)?,
        json!("source")
    );
    let mut exact = QueryBudget::new(7 + measured.reads);
    exact.charge_many(7)?;
    assert_eq!(
        getters.read_model(&model, "item", "label", &mut exact)?,
        json!("source")
    );
    assert_eq!(exact.remaining(), 0);
    model.set("item", "raw", json!("B"))?;
    assert_eq!(
        getters.read_model(&model, "item", "label", &mut measured)?,
        json!("B")
    );
    model.set("item", "raw", json!(null))?;
    assert_eq!(
        getters.read_model(&model, "item", "label", &mut measured)?,
        json!(null)
    );
    assert!(
        getters
            .read_model(&model, "item", "raw", &mut measured)
            .is_err()
    );
    let mut short = QueryBudget::new(0);
    assert!(
        getters
            .read_model(&model, "item", "label", &mut short)
            .is_err()
    );
    Ok(())
}

#[test]
fn cached_getter_selectors_and_schema_are_never_executable_authority() -> Result<(), String> {
    let package = package()?;
    let mut removed = package.clone();
    removed.semantics.derived.clear();
    assert!(getters(&removed).is_err());
    let mut redirected = package.clone();
    redirected.semantics.derived[0].feature = "raw".into();
    assert!(getters(&redirected).is_err());
    let mut shape = package.clone();
    shape
        .schema
        .classes
        .get_mut("Item")
        .ok_or("Missing Item")?
        .features
        .get_mut("label")
        .ok_or("Missing label")?
        .lower = 1;
    assert!(getters(&shape).is_err());
    let mut reference_context = package.clone();
    reference_context.schema.reference_top = None;
    assert!(getters(&reference_context).is_err());
    let mut revision = package.clone();
    revision.schema.revision = "invented-revision".into();
    assert!(getters(&revision).is_err());
    let mut stale = package.clone();
    stale
        .compilation
        .as_mut()
        .ok_or("Missing recipe")?
        .bundle
        .sources[0]
        .text
        .push_str("// changed");
    assert!(getters(&stale).is_err());
    assert!(
        RcoreGetters::from_package(
            &package,
            &Limits::default(),
            ExecutionLimits::default(),
            ParseLimits { source_bytes: 0 }
        )
        .is_err()
    );
    getters(&package)?;
    Ok(())
}

#[test]
fn getter_schema_replay_follows_the_exact_authored_parent_grammar() -> Result<(), String> {
    let initial = package()?;
    let mut recipe = initial.compilation.ok_or("Missing retained recipe")?;
    let mut parent = DefinitionSource::new(
        "memory:getter-terminals.xtext",
        SourceKind::Xtext,
        "grammar language.Parent import \"http://www.eclipse.org/emf/2002/Ecore\" as ecore terminal ID returns ecore::EString: ('a'..'z'|'A'..'Z')+;",
    );
    parent.dependencies.push(recipe.ecore.clone());
    let mut child = DefinitionSource::new(
        "memory:language-getters.xtext",
        SourceKind::Xtext,
        "grammar language.Getters with language.Parent import \"urn:language-getters\" as m Item returns m::Item: 'item' raw=ID;",
    );
    child.dependencies = vec![recipe.ecore.clone(), parent.id.clone()];
    recipe
        .bundle
        .sources
        .retain(|source| source.kind != SourceKind::Xtext);
    recipe.xtext = Some(child.id.clone());
    recipe.bundle.sources.extend([parent.clone(), child]);
    let compiled = intersect_text::ordinary::compile(recipe.clone(), Profile::NativeV67)
        .map_err(|e| format!("{e:?}"))?;
    getters(&compiled)?;

    let mut duplicate = DefinitionSource::new(
        "memory:other-getter-terminals.xtext",
        SourceKind::Xtext,
        &parent.text,
    );
    duplicate.dependencies = parent.dependencies.clone();
    let mut ambiguous = compiled.clone();
    let replay = ambiguous
        .compilation
        .as_mut()
        .ok_or("Missing inherited recipe")?;
    let selected = replay.xtext.clone().ok_or("Missing selected grammar")?;
    replay
        .bundle
        .sources
        .iter_mut()
        .find(|source| source.id == selected)
        .ok_or("Missing child grammar")?
        .dependencies
        .push(duplicate.id.clone());
    replay.bundle.sources.push(duplicate);
    assert!(getters(&ambiguous).is_err());
    Ok(())
}
