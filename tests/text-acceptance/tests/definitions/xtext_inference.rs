use intersect_model::definitions::bundle::DefinitionBundle;
use intersect_model::definitions::bundle::DefinitionSource;
use intersect_model::definitions::bundle::SourceKind;
use intersect_text::grammar;
use intersect_text::grammar::Datatype;
use intersect_text::grammar::Profile;
const ECORE: &str = include_str!("../../../../fixtures/xtext/qualified-names/model.ecore");
const IMPORT: &str = "import \"urn:intersect:xtext:mini-kerml:qualified-names:1\" as k";
const TOKENS: &str = r"terminal ID: ('a'..'z'|'A'..'Z'|'_') ('a'..'z'|'A'..'Z'|'0'..'9'|'_')*;
terminal WS: (' '|'\t'|'\r'|'\n')+;";
fn lexical(text: &str) -> String {
    let (header, body) = text.split_once('\n').unwrap();
    let import = if body.contains(IMPORT) { "" } else { IMPORT };
    format!(
        "{} hidden(WS)\n{import}\n{body}\n{TOKENS}",
        header.replace(" with org.eclipse.xtext.common.Terminals", "")
    )
}
const ORIGINAL: &str = include_str!("../../../../fixtures/xtext/qualified-names/model.xtext");
fn compile(
    text: &str,
    profile: Profile,
) -> Result<grammar::Grammar, intersect_model::definitions::schema::Diagnostic> {
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    grammar::compile_with_reference_context_and_profile(&lexical(text), &mut schema, profile)
}
#[test]
fn forward_inference_preserves_return_origin_and_old_profile_rejection() {
    let text = ORIGINAL.replace(
        "QualifiedName returns ecore::EString: ID ('::' ID)*;",
        "QualifiedName: Name ('::' Name)*; Name: ID;",
    );
    assert!(compile(&text, Profile::NativeV3).is_err());
    let g = compile(&text, Profile::NativeV4).unwrap();
    for name in ["QualifiedName", "Name"] {
        assert!(g.rules[name].inferred_return);
        assert_eq!(g.rules[name].datatype, Some(Datatype::EString));
    }
    assert!(!g.rules["Model"].inferred_return);
    assert_eq!(
        serde_json::to_value(&g).unwrap()["rules"]["Name"]["inferred_return"],
        true
    );
    let old = compile(ORIGINAL, Profile::NativeV3).unwrap();
    assert!(
        serde_json::to_value(old).unwrap()["rules"]["QualifiedName"]
            .get("inferred_return")
            .is_none()
    );
}
#[test]
fn collision_matches_upstream_without_changing_old_profiles() {
    let text = "grammar example.Collision\nModel: 'package' name=ID; Type: 'type';";
    let old = compile(text, Profile::NativeV3).unwrap();
    assert_eq!(old.rules["Type"].class, "Type");
    assert_eq!(old.rules["Type"].datatype, None);
    let current = compile(text, Profile::NativeV4).unwrap();
    assert_eq!(current.rules["Type"].datatype, Some(Datatype::EString));
    assert_eq!(current.rules["Model"].datatype, None);
    assert!(current.rules["Model"].inferred_return);
    let explicit = compile(
        &text.replace("Type:", "Type returns Type:"),
        Profile::NativeV4,
    )
    .unwrap();
    assert_eq!(explicit.rules["Type"].datatype, None);
    assert!(!explicit.rules["Type"].inferred_return);
}
#[test]
fn unsupported_inferred_graphs_reject_without_relaxing_execution() {
    for body in [
        "Missing",
        "ID?",
        "'x' QualifiedName?",
        "{k::Type} 'x'",
        "name=ID",
        "isAbstract?='x'",
    ] {
        let text = ORIGINAL.replace(
            "QualifiedName returns ecore::EString: ID ('::' ID)*;",
            &format!("QualifiedName: {body};"),
        );
        assert!(compile(&text, Profile::NativeV4).is_err(), "{body}");
    }
    let text = ORIGINAL
        .replace("name=QualifiedName", "isAbstract=QualifiedName")
        .replace("QualifiedName returns ecore::EString", "QualifiedName");
    assert!(compile(&text, Profile::NativeV4).is_err());
}
#[test]
fn inherited_forward_scalar_calls_keep_exact_declaring_source() {
    let base = DefinitionSource::new(
        "base.xtext",
        SourceKind::Xtext,
        lexical("grammar example.Base\nModel: 'package' name=Name; Name: ID;"),
    );
    let mut child = DefinitionSource::new(
        "child.xtext",
        SourceKind::Xtext,
        format!(
            "grammar example.Child with example.Base\n{IMPORT}\nEntry returns Model: 'child' name=Qualified; Qualified: Name ('::' Name)*;"
        ),
    );
    child.dependencies.push(base.id.clone());
    let bundle = DefinitionBundle {
        version: 1,
        sources: vec![child.clone(), base.clone()],
        namespaces: vec![],
    };
    let mut schema =
        intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
            .unwrap();
    let g = grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV4).unwrap();
    assert_eq!(g.rules["Qualified"].datatype, Some(Datatype::EString));
    assert_eq!(g.rules["Name"].datatype, Some(Datatype::EString));
    assert_eq!(g.declaration_sources["Name"].source, base.id);
    assert_eq!(g.declaration_sources["Qualified"].source, child.id);
}

#[test]
fn inferred_override_types_are_resolved_before_compatibility_checks() {
    let base = DefinitionSource::new(
        "base.xtext",
        SourceKind::Xtext,
        lexical("grammar example.Base\nModel: 'package' name=Name; Name: ID;"),
    );
    for (body, accepted) in [
        ("@Override Model: 'module' name=Name;", true),
        ("@Override Model: 'module';", false),
        ("Name: 'literal';", false),
    ] {
        let mut child = DefinitionSource::new(
            "child.xtext",
            SourceKind::Xtext,
            format!("grammar example.Child with example.Base\n{IMPORT}\n{body}"),
        );
        child.dependencies.push(base.id.clone());
        let bundle = DefinitionBundle {
            version: 1,
            sources: vec![base.clone(), child.clone()],
            namespaces: vec![],
        };
        let mut schema =
            intersect_text_acceptance::migration_fixture::compile_migrated_schema_fixture(ECORE)
                .unwrap();
        let result = grammar::compile_sources(&bundle, &child.id, &mut schema, Profile::NativeV4);
        if accepted {
            let g = result.unwrap();
            assert!(g.rules["Model"].inferred_return);
            assert_eq!(g.rules["Model"].datatype, None);
            assert_eq!(g.shadowed_declarations["Model"][0].source, base.id);
        } else {
            assert!(
                result
                    .unwrap_err()
                    .message
                    .contains("same model return type")
            );
        }
    }
}
