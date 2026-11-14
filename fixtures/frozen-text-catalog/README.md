# Catalog: a non-KerML language through the frozen text path

A small Domainmodel-style language (packages with dotted names, `import a.b.*`
and `import a.b.C`, entities with `extends`, typed features) written for this
repository. It is not an upstream example. It checks that the frozen text path
works for an ordinary Xtext grammar and Ecore metamodel with no
language-specific Rust:

- `catalog.ecore` and `catalog.xtext` are the whole language definition; the
  grammar's parent is Xtext's unmodified common terminals.
- `catalog-scoping.rcore` is Xtext's default scoping
  (`ImportedNamespaceAwareLocalScopeProvider` over `DefaultGlobalScopeProvider`)
  for this metamodel, written in Rcore: relative to each enclosing package,
  then through the resource's imports, then fully qualified.
- `types.catalog` and `shop.catalog` exercise each scoping case.

`crates/intersect-xtext/tests/frozen_text_catalog.rs` prepares the language
with the ordinary compiler, parses both files into `FrozenStore` with
`OpenDraftCapability` and `GrammarPolicy`, links with the compiled Rcore
scopes, validates, and checks every reference against its expected target.

The scoping is written against this metamodel (its `name` attribute and
containment). Rcore has no reflective access to an arbitrary class's name
feature, so each language states its own version of these rules.
