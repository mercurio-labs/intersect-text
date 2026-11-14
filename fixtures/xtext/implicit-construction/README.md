# Implicit construction at first assignment

This independently authored NativeV28 example uses the unchanged metamodel at
`fixtures/compatibility/graph.ecore` and empty unique-name semantic definitions.
It creates a Graph and four Nodes using scalar, Boolean and cross-reference first
assignments alongside an explicit action. The graph links are explicit to satisfy
the metamodel's reciprocal-reference contract. This is not imported KerML syntax.

`tests/acceptance/tests/language/xtext_implicit_construction.rs` verifies identities,
spans, defaults, links, validation, replay, exact element limits, malformed input,
recovery and abstract/interface/feature/current-flow rejection. Separate root-node
controls verify the existing constant initializer for implicit and explicit objects.
All four groups also run through `tools/test_xtext_implicit_construction_wasm.mjs`.

Current boundary: initializers combined with deferred reciprocal graph links fail
validation before linking completes. That rejection is retained explicitly; this
increment does not qualify the full initialization/linking lifecycle or X09/X18.
NativeV27 and older admission remain unchanged.
