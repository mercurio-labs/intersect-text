# Explicit empty-package imports

Authored NativeV44 example. packages.ecore defines Model in urn:main and an empty
nested package urn:empty. The grammar explicitly imports both and parses
`model valid`. No class or domain behavior is fabricated for the empty package.

Compile a DefinitionBundle selecting packages.ecore/urn:main, model.xtext and
model-behavior.rcore. Grammar and semantic sources declare their exact packages.ecore
dependency. The compiler exports checked, reachable package identities from its
existing resource set; grammar admission uses them without reparsing Ecore.
Both ordinary and materialized compilation follow this contract. Namespace
bindings select exact source identities when multiple resources share a namespace.

Run `cargo test -p intersect-language --test xtext_empty_package_imports --locked`
and `node tools/test_xtext_empty_package_imports_wasm.mjs` for native/shared-WASM
acceptance, replay, limits and rejection controls. Tests retain the NativeV43
rejection as a compatibility boundary. See the daily audit for executed results.

Remaining X02 obligations include ordinary enum-valued parser/terminal rules,
broader datatype contracts, implicit imports and the full composition audit.
This fixture is authored; no upstream source or runtime implementation is copied.
