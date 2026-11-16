//! A language that is not KerML through the whole frozen text path: an Xtext
//! grammar and an Ecore metamodel (`fixtures/frozen-text-catalog`) parsed by
//! `OpenDraftCapability` with only `GrammarPolicy`, linked with Xtext-default
//! scoping written in Rcore, and validated. No language-specific Rust is used.
use intersect_model::core::frozen::FeatureId;
use intersect_model::core::frozen::FrozenError;
use intersect_model::core::frozen::FrozenStore;
use intersect_model::core::frozen::LinkView;
use intersect_model::core::frozen::Metamodel;
use intersect_model::core::frozen::ObjId;
use intersect_model::core::query::QueryBudget;
use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_model::definitions::ecore::ParseLimits;
use intersect_model::definitions::rcore_behavior::ir_v2::Limits;
use intersect_model::definitions::rcore_scope::compile_bindings_with_source_limits;
use intersect_model::definitions::rcore_scope::source_scope_bindings;
use intersect_model::frozen_scopes::RcoreScopes;
use intersect_text::grammar::Profile;
use intersect_text::package::{BundleCompilation, CompilationSourcePolicy};
use intersect_text::{
    EvaluationLimits, Source,
    document::AnalysisLimits,
    draft::{DraftLanguage, GrammarPolicy, OpenDraftCapability},
};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};

const DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/frozen-text-catalog"
);
const LIMITS: ParseLimits = ParseLimits {
    source_bytes: 1_000_000,
};

fn read(name: &str) -> Result<String, String> {
    std::fs::read_to_string(format!("{DIR}/{name}")).map_err(|e| format!("{name}: {e}"))
}

#[test]
fn catalog_language_parses_links_with_rcore_scoping_and_validates() -> Result<(), String> {
    // The language definition: metamodel and grammar, nothing else. The
    // grammar's parent is Xtext's unmodified common terminals.
    let ecore = DefinitionSource::new("catalog.ecore", SourceKind::Ecore, read("catalog.ecore")?);
    let mut terminals = DefinitionSource::new(
        "org/eclipse/xtext/common/Terminals.xtext",
        SourceKind::Xtext,
        include_str!(
            "../../../fixtures/definition-conformance/upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"
        ),
    );
    terminals.dependencies.push(ecore.id.clone());
    let mut grammar =
        DefinitionSource::new("catalog.xtext", SourceKind::Xtext, read("catalog.xtext")?);
    grammar
        .dependencies
        .extend([ecore.id.clone(), terminals.id.clone()]);
    let mut rules = DefinitionSource::new(
        "catalog-rules.rcore",
        SourceKind::RcoreBehavior,
        r#"rcore 1; package catalogRules version "1";"#,
    );
    rules.dependencies.push(ecore.id.clone());
    let input = BundleCompilation {
        bundle: DefinitionBundle {
            version: 1,
            sources: vec![ecore.clone(), terminals, grammar.clone(), rules.clone()],
            namespaces: vec![],
        },
        ecore: ecore.id.clone(),
        xtext: Some(grammar.id.clone()),
        rcore_behavior: rules.id.clone(),
        namespace: "urn:intersect:example:catalog:1".into(),
        materialized_types: vec![],
    };
    let migration = intersect_model::migration::migrate_ecore_bundle(
        input.bundle.clone(),
        &[(ecore.id.clone(), "catalog.rcore".into())].into(),
    )
    .map_err(|e| e.to_string())?;
    let input = intersect_text::migration::remap_compilation(&migration, &input)?;
    let policy = CompilationSourcePolicy::default();
    // Ordinary preparation: this grammar needs no guarded runtime prediction.
    let definition = intersect_text::compiler::prepare_bundle_with_borrowed_authored_validation(
        &input,
        &migration,
        Profile::NativeV80,
        &[],
        None,
        policy,
        policy,
        &|_, _, _| Ok(()),
    )
    .map_err(|e| format!("{e:?}"))?;

    // Construction from text into the frozen engine.
    let inner = GrammarPolicy::new(&definition);
    let capability = OpenDraftCapability::new(&definition, &inner)?;
    let language = DraftLanguage::bind(&definition, &capability, AnalysisLimits::workbench())?;
    let mm = Arc::new(Metamodel::freeze(definition.schema()).map_err(|e| e.to_string())?);
    let mut store = FrozenStore::new(mm.clone());
    let mut proxies = 0;
    for name in ["types.catalog", "shop.catalog"] {
        let source = Source {
            uri: name.into(),
            text: read(name)?,
        };
        let (evaluation, written, _) = language.parse_runtime_into(
            &source,
            EvaluationLimits::native(),
            &mut store,
            &mut QueryBudget::new(usize::MAX),
        )?;
        assert!(
            evaluation.diagnostics.is_empty(),
            "{name}: {:?}",
            evaluation.diagnostics
        );
        assert_eq!(written, evaluation.unresolved_references.len());
        assert!(
            evaluation.references.is_empty(),
            "construction does not invent resolved targets"
        );
        for reference in &evaluation.unresolved_references {
            assert_eq!(reference.uri, source.uri);
            assert_eq!(
                &source.text[reference.start..reference.end],
                reference.spelling
            );
            assert!(!reference.owner.is_empty());
            assert!(!reference.feature.is_empty());
            assert!(!reference.target_class.is_empty());
        }
        proxies += written;
    }
    assert_eq!(proxies, 9, "cross references left for the linker");

    // Xtext-default scoping, written in Rcore against the migrated structure.
    let structure = definition
        .sources()
        .sources
        .iter()
        .find(|source| source.kind == SourceKind::Rcore)
        .ok_or("Missing migrated structure")?
        .clone();
    let mut scoping = DefinitionSource::new(
        "catalog-scoping.rcore",
        SourceKind::RcoreBehavior,
        read("catalog-scoping.rcore")?,
    );
    scoping.dependencies.push(structure.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![structure, scoping.clone()],
        namespaces: vec![],
    };
    let declarations: Vec<_> = source_scope_bindings(&scoping, LIMITS)
        .map_err(|e| format!("{e:?}"))?
        .iter()
        .map(|b| (b.declaration, false))
        .collect();
    let scopes = compile_bindings_with_source_limits(
        &bundle,
        &scoping.id,
        &declarations,
        &Limits::default(),
        LIMITS,
    )
    .map_err(|e| format!("{e:?}"))?;
    let resolvers = RcoreScopes::new(&mm, &bundle, &scopes).map_err(|e| e.to_string())?;

    // Link, recording each answer by (owner name, feature, spelling).
    let names = RefCell::new(BTreeMap::new());
    let name_of =
        |view: &LinkView<'_>, obj: ObjId| -> String { view.identity(obj).unwrap_or("").to_owned() };
    let resolve = |view: &LinkView<'_>,
                   owner: ObjId,
                   feature: FeatureId,
                   spelling: &str|
     -> Result<Option<ObjId>, FrozenError> {
        let found = resolvers.candidates(view, owner, feature, spelling)?;
        names.borrow_mut().insert(
            (
                name_of(view, owner),
                mm.feature_name(feature).to_owned(),
                spelling.to_owned(),
            ),
            found.iter().map(|o| name_of(view, *o)).collect::<Vec<_>>(),
        );
        Ok(match found.as_slice() {
            [one] => Some(*one),
            _ => None,
        })
    };
    let linked = store
        .into_builder()
        .map_err(|e| e.to_string())?
        .link(&resolve)
        .map_err(|e| format!("{e:?}"))?;
    let model = linked.validate().map_err(|e| format!("{e:?}"))?;
    let exported = model.to_model().map_err(|e| e.to_string())?;
    let qualified = |id: &str| -> String {
        // Identity -> qualified name, through the exported containment.
        let by_id: BTreeMap<_, _> = exported
            .elements
            .iter()
            .map(|e| (e.id.as_str(), e))
            .collect();
        let parent: BTreeMap<&str, &str> = exported
            .elements
            .iter()
            .flat_map(|e| {
                ["elements", "features"].into_iter().flat_map(move |f| {
                    e.values
                        .get(f)
                        .and_then(|v| v.as_array())
                        .into_iter()
                        .flatten()
                        .filter_map(|c| c.as_str())
                        .map(move |c| (c, e.id.as_str()))
                })
            })
            .collect();
        let mut parts = Vec::new();
        let mut at = Some(id);
        while let Some(current) = at {
            if let Some(name) = by_id
                .get(current)
                .and_then(|e| e.values.get("name"))
                .and_then(|v| v.as_str())
            {
                parts.push(name.to_owned());
            }
            at = parent.get(current).copied();
        }
        parts.reverse();
        parts.join(".")
    };
    let recorded: BTreeMap<(String, String, String), Vec<String>> = names.into_inner();
    let answers: BTreeMap<(String, String, String), Vec<String>> = recorded
        .into_iter()
        .map(|((owner, feature, spelling), found)| {
            (
                (qualified(&owner), feature, spelling),
                found.iter().map(|f| qualified(f)).collect(),
            )
        })
        .collect();
    let expect = |owner: &str, feature: &str, spelling: &str, target: &str| {
        (
            (owner.to_owned(), feature.to_owned(), spelling.to_owned()),
            vec![target.to_owned()],
        )
    };
    let expected: BTreeMap<_, _> = [
        // Relative to an enclosing package.
        expect(
            "base.Named.name",
            "type",
            "types.String",
            "base.types.String",
        ),
        expect("shop.Customer.orders", "type", "Order", "shop.Order"),
        expect("shop.Order.customer", "type", "Customer", "shop.Customer"),
        expect(
            "shop.billing.Invoice.buyer",
            "type",
            "Customer",
            "shop.Customer",
        ),
        // Through `import base.types.*` and `import base.Named`.
        expect("shop.Customer", "superType", "Named", "base.Named"),
        expect("shop.Customer.age", "type", "Int", "base.types.Int"),
        expect(
            "shop.billing.Invoice.number",
            "type",
            "String",
            "base.types.String",
        ),
        // Fully qualified.
        expect(
            "shop.Order.total",
            "type",
            "base.types.Int",
            "base.types.Int",
        ),
        expect(
            "shop.billing.Invoice",
            "superType",
            "shop.Order",
            "shop.Order",
        ),
    ]
    .into_iter()
    .collect();
    assert_eq!(answers, expected);
    Ok(())
}
