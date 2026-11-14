use intersect_model::definitions::{
    bundle::{DefinitionBundle, DefinitionSource, SourceKind},
    generic_resolution::Resolver,
    resources::DeclarationId,
};
use serde_json::{Value, json};
fn request(text: &str, path: &[usize], parameter: bool) -> Value {
    let source = DefinitionSource::new("memory:generics", SourceKind::Ecore, text);
    json!({"method": if parameter {"ecore/generic-parameter"} else {"ecore/generic-type"}, "declaration": {"source":source.id,"path":path}, "bundle":{"version":1,"sources":[source],"namespaces":[]}})
}
fn dispatch(input: &Value) -> Value {
    intersect_web::dispatch(&input.to_string()).unwrap()
}
const FIXTURE: &str = include_str!("../../../fixtures/ecore-generics/signatures.ecore");
#[test]
fn host_checks_exact_shared_reports_without_claiming_execution() {
    for (path, parameter) in [
        (vec![1, 0], true),
        (vec![1, 1, 0], false),
        (vec![1, 2, 1], false),
        (vec![1, 2, 2, 0], false),
        (vec![2, 0, 0], false),
        (vec![2, 1, 0], false),
    ] {
        let input = request(FIXTURE, &path, parameter);
        let bundle: DefinitionBundle = serde_json::from_value(input["bundle"].clone()).unwrap();
        let id: DeclarationId = serde_json::from_value(input["declaration"].clone()).unwrap();
        let resolver = Resolver::new(&bundle).unwrap();
        let expected = if parameter {
            json!(resolver.resolve_parameter(&id).unwrap())
        } else {
            json!(resolver.resolve_type(&id).unwrap())
        };
        let result = dispatch(&input);
        assert_eq!(
            result,
            json!({"ok":true,"stage":"generic_signature","executable":false,"report":expected})
        );
        if path == [2, 0, 0] {
            assert_eq!(result["report"]["warnings"][0]["code"], "raw_type");
        }
    }
}
#[test]
fn host_rejects_stale_identities_scope_and_arity_with_source_locations() {
    for text in [
        FIXTURE.replace("#//Box/convert/U", "#//Comparable/T"),
        FIXTURE.replace("<eTypeArguments/>", "<eTypeArguments/><eTypeArguments/>"),
    ] {
        let path = if text.contains("eTypeArguments/><eTypeArguments") {
            vec![2, 1, 0]
        } else {
            vec![1, 2, 1]
        };
        let input = request(&text, &path, false);
        let result = dispatch(&input);
        assert_eq!(result["ok"], false, "{result}");
        assert_eq!(result["executable"], false);
        assert_eq!(
            result["diagnostics"][0]["span"]["source"],
            input["declaration"]["source"]
        );
    }
    let mut stale = request(FIXTURE, &[1, 1, 0], false);
    stale["declaration"]["source"]["sha256"] = json!("0".repeat(64));
    assert_eq!(dispatch(&stale)["ok"], false);
    assert!(intersect_web::dispatch(&json!({"method":"ecore/generic-type"}).to_string()).is_err());
}

#[test]
fn complete_host_inventory_reports_unused_errors_and_source_edits() {
    let mut input = request(FIXTURE, &[], false);
    input["method"] = json!("ecore/generic-signatures");
    input.as_object_mut().unwrap().remove("declaration");
    let accepted = dispatch(&input);
    assert_eq!(accepted["ok"], true);
    assert_eq!(accepted["executable"], false);
    assert_eq!(
        accepted["report"]["parameters"].as_array().unwrap().len(),
        3
    );
    assert_eq!(accepted["report"]["types"].as_array().unwrap().len(), 5);
    let changed = FIXTURE.replace("name=\"raw\"", "name=\"renamed\"");
    let mut revised = request(&changed, &[], false);
    revised["method"] = json!("ecore/generic-signatures");
    let report = dispatch(&revised);
    assert_eq!(report["ok"], true);
    assert_ne!(accepted["report"]["sources"], report["report"]["sources"]);
    let broken = FIXTURE.replace("#//Box/convert/U", "#//Comparable/T");
    let mut rejected = request(&broken, &[], false);
    rejected["method"] = json!("ecore/generic-signatures");
    let result = dispatch(&rejected);
    assert_eq!(result["ok"], false);
    assert!(result.get("report").is_none());
    // All definition sources share the host budget, including non-Ecore inputs.
    let extra = DefinitionSource::new(
        "memory:unused",
        SourceKind::RcoreBehavior,
        " ".repeat(256_001),
    );
    input["bundle"]["sources"]
        .as_array_mut()
        .unwrap()
        .push(json!(extra));
    assert!(
        intersect_web::dispatch(&input.to_string())
            .unwrap_err()
            .contains("budget")
    );
}

#[test]
fn source_substitution_resolves_inputs_and_never_claims_bound_verification() {
    let mut input = request(FIXTURE, &[1, 1, 0], false);
    input["method"] = json!("ecore/generic-substitute");
    let id = |path: &[usize]| json!({"source": input["declaration"]["source"], "path":path});
    let binding = json!({"parameter":id(&[1,0]),"argument":id(&[2,1,0])});
    input["bindings"] = json!([binding.clone()]);
    let accepted = dispatch(&input);
    assert_eq!(accepted["ok"], true, "{accepted}");
    assert_eq!(accepted["stage"], "generic_substitution");
    assert_eq!(accepted["bounds_verified"], false);
    assert_eq!(accepted["executable"], false);
    assert_eq!(accepted["report"]["expression"]["kind"], "classifier");
    assert_eq!(
        accepted["report"]["expression"]["arguments"][0]["kind"],
        "wildcard"
    );
    assert_eq!(
        accepted["report"]["arguments"][0]["declaration"],
        binding["argument"]
    );
    // The selected Box<?> is not evidence of satisfaction of T's Comparable<T> bound.
    for (parameter, argument, message) in [
        (vec![1], vec![2, 1, 0], ""), // Classifier cannot be asserted to be a formal.
        (vec![1, 0], vec![1, 0, 0], "Cyclic"), // T -> Comparable<T> expands infinitely.
        (vec![1, 0], vec![2], ""),    // Argument must select a generic type use.
    ] {
        let mut rejected = input.clone();
        rejected["bindings"][0]["parameter"]["path"] = json!(parameter);
        rejected["bindings"][0]["argument"]["path"] = json!(argument);
        let report = dispatch(&rejected);
        assert_eq!(report["ok"], false, "{report}");
        assert!(
            report["diagnostics"][0]["message"]
                .as_str()
                .unwrap()
                .contains(message)
        );
        assert_eq!(report["bounds_verified"], false);
        assert_eq!(report["executable"], false);
    }
    let mut duplicate = input.clone();
    duplicate["bindings"] = json!([binding.clone(), binding]);
    assert!(
        dispatch(&duplicate)["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Duplicate")
    );
    let mut stale = input.clone();
    stale["bindings"][0]["argument"]["source"]["sha256"] = json!("0".repeat(64));
    assert_eq!(dispatch(&stale)["ok"], false);
    let mut forged = input;
    forged["bindings"][0]["expression"] = json!({"kind":"wildcard","upper":null,"lower":null});
    assert!(intersect_web::dispatch(&forged.to_string()).is_err());
}

#[test]
fn provider_selected_reports_share_bounds_members_and_operation_checks() {
    let text = include_str!("../../../fixtures/ecore-generics/provider-alias-bounds.ecore").replacen(
        "  </eClassifiers>",
        r##"    <eOperations name="identity"><eTypeParameters name="U"><eBounds eClassifier="#//RGB"/></eTypeParameters><eGenericType eTypeParameter="#//Holder/identity/U"/></eOperations>
  </eClassifiers>"##, 1);
    let source = DefinitionSource::new("memory:provider-reports", SourceKind::Ecore, text);
    let id = |path: &[usize]| json!({"source": source.id, "path": path});
    let ty = |n| json!({"kind":"classifier","declaration":id(&[n]),"arguments":[],"raw":false});
    let receiver =
        json!({"kind":"classifier","declaration":id(&[2]),"arguments":[ty(1)],"raw":false});
    let contracts = (0..2).map(|n| json!({"datatype":id(&[n]),"provider":{"name":"example.rgb","version":"1"},"serializable":true})).collect::<Vec<_>>();
    let base = json!({"bundle":{"version":1,"sources":[source],"namespaces":[]},"declaration":id(&[3,0,0]),"receiver":receiver,"operation":id(&[2,2]),"arguments":[ty(1)],"conversions":contracts});
    for method in [
        "ecore/generic-application-bounds",
        "ecore/generic-member-types",
        "ecore/generic-instantiate-members",
        "ecore/generic-operation-types",
    ] {
        let mut input = base.clone();
        input["method"] = json!(method);
        let accepted = dispatch(&input);
        assert_eq!(accepted["ok"], true, "{accepted}");
        assert_eq!(accepted["executable"], false);
        input.as_object_mut().unwrap().remove("conversions");
        assert_eq!(dispatch(&input)["ok"], false);
        input["conversions"] = json!([contracts[1]]);
        assert_eq!(dispatch(&input)["ok"], false);
        input["conversions"] = json!(contracts);
        input["conversions"][0]["provider"]["version"] = json!("unknown");
        assert!(
            intersect_web::dispatch(&input.to_string())
                .unwrap_err()
                .contains("Unknown native conversion provider")
        );
        input["conversions"] = json!(contracts);
        input["conversions"][0]["relation"] = json!("equivalent");
        assert!(
            intersect_web::dispatch(&input.to_string())
                .unwrap_err()
                .contains("unknown field")
        );
    }
}

#[test]
fn host_operation_selection_and_override_reports_keep_claims_separate() {
    let text = include_str!("../../../fixtures/ecore-generics/operation-overrides.ecore");
    let source = DefinitionSource::new("memory:selection", SourceKind::Ecore, text);
    let id = |path: &[usize]| json!({"source":source.id,"path":path});
    let receiver = json!({"kind":"classifier","declaration":id(&[3]),"arguments":[],"raw":false});
    let bundle = json!({"version":1,"sources":[source],"namespaces":[]});
    let input =
        json!({"method":"ecore/generic-operation-selection","bundle":bundle,"receiver":receiver});
    let result = dispatch(&input);
    assert_eq!(result["ok"], true);
    assert_eq!(result["selection_verified"], true);
    assert_eq!(result["executable"], false);
    assert_eq!(
        result["report"]["operations"][0]["selected"]["member"]["declaration"],
        id(&[3, 1])
    );
    assert_eq!(
        result["report"]["operations"][0]["overridden"][0]["member"]["declaration"],
        id(&[2, 1])
    );
    let definition_bundle: DefinitionBundle = serde_json::from_value(bundle.clone()).unwrap();
    let checked_receiver = serde_json::from_value(receiver).unwrap();
    assert_eq!(
        result["report"],
        json!(
            Resolver::new(&definition_bundle)
                .unwrap()
                .resolved_generic_operations(&checked_receiver)
                .unwrap()
        )
    );
    for (operation, base, overrides) in [
        (vec![3, 1], vec![2, 1], true),
        (vec![2, 1], vec![3, 1], false),
    ] {
        let report = dispatch(
            &json!({"method":"ecore/generic-operation-override","bundle":bundle,"operation":id(&operation),"base":id(&base)}),
        );
        assert_eq!(report["ok"], true);
        assert_eq!(report["report"]["overrides"], overrides);
        assert_eq!(report["selection_verified"], false);
        assert_eq!(report["executable"], false);
    }
    let mut invalid = input.clone();
    invalid["receiver"]["declaration"]["path"] = json!([0]);
    invalid["selection_verified"] = json!(true);
    let rejected = dispatch(&invalid);
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["selection_verified"], false);
    invalid = input;
    invalid["receiver"]["declaration"]["source"]["sha256"] = json!("0".repeat(64));
    let rejected = dispatch(&invalid);
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["selection_verified"], false);
    assert!(rejected["diagnostics"][0]["span"].is_object());
}

#[test]
fn materialized_method_applications_match_the_shared_resolver() {
    let source = DefinitionSource::new(
        "memory:host-methods",
        SourceKind::Ecore,
        include_str!("../../../fixtures/ecore-generics/method-templates.ecore"),
    );
    let (source, ids) =
        intersect_text_acceptance::migration_fixture::migrate_source_fixture(source);
    let id = |path: &[usize]| json!(ids[path]);
    let ty = |n| json!({"kind":"classifier","declaration":id(&[n]),"arguments":[],"raw":false});
    let bundle = json!({"version":1,"sources":[source],"namespaces":[]});
    let base = json!({"method":"ecore/materialize","bundle":bundle,"types":[ty(0),ty(2),ty(3)],"instances":[{"id":"c","type":ty(2)}],"operation_applications":[{"receiver":ty(2),"operation":id(&[1,1]),"arguments":[ty(0)]}]});
    let result = dispatch(&base);
    assert_eq!(result["ok"], true);
    assert_eq!(result["model"]["elements"][0]["id"], "c");
    assert_eq!(result["operation_applications"][0]["executable"], false);
    let standalone = dispatch(
        &json!({"method":"ecore/generic-operation-types","bundle":bundle,"receiver":ty(2),"operation":id(&[1,1]),"arguments":[ty(0)]}),
    );
    assert_eq!(
        result["operation_applications"][0]["report"],
        standalone["report"]
    );
    for arguments in [json!([]), json!([ty(3)]), json!([ty(0), ty(0)])] {
        let mut bad = base.clone();
        bad["operation_applications"][0]["arguments"] = arguments;
        let rejected = dispatch(&bad);
        assert_eq!(rejected["ok"], false);
        assert!(rejected["diagnostics"][0]["span"].is_object());
    }
    let mut stale = base;
    stale["operation_applications"][0]["operation"]["source"]["sha256"] = json!("0".repeat(64));
    assert_eq!(dispatch(&stale)["ok"], false);
}
