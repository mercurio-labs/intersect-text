# Boolean keyword example (X-D01)

These are Intersect-authored, deliberately simplified KerML-shaped fixtures,
not copied upstream definitions or a claim of KerML conformance. The namespace
`urn:intersect:xtext:mini-kerml:boolean:1` is distinct from all upstream namespaces.
The real pinned grammar uses `isAbstract ?= 'abstract'` and
`isStandard ?= 'standard'`; provenance.json identifies that reference. This
fixture places both flags on Type and uses a true default on isStandard solely
to prove absence preserves defaults and unset state. That is not KerML's model.

Compile model.ecore, model.xtext and model-behavior.rcore, then evaluate example.kerml.
The example contains three Types. Only Vehicle is abstract. All three read
isStandard=true, but only Scalar has explicitly assigned it. Source bytes and
identities survive package reconstruction. No Java is needed.

Current new construct: a keyword RHS for `?=` on a writable single EBoolean
attribute. Other RHS categories and nominal custom Boolean codecs remain
explicitly unsupported. Actions, parser fragments, unassigned parser dispatch,
custom terminals, datatype rules, general prediction and KerML scoping remain
outside this checkpoint. This does not close full X08.

Native controls: tests/acceptance/tests/language/xtext_boolean.rs and the Boolean
rollback control in src/parser_tests.rs. Actual native/WASM controls:
`node --test toolkit/tests/xtext.node.js` from the repository root, or
`npm run test:xtext` from toolkit. The latter also executes the unchanged Library
example through the same native/WASM services.
