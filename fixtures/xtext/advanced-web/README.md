# Advanced Xtext teaching examples

These newly authored examples use NativeV27 and existing metamodel/semantic
fixtures unchanged. They are not imported upstream grammars.

Library uses ../../library/library.ecore and library-behavior.rcore (relative to the
fixtures root: fixtures/library). It demonstrates shared-prefix choices,
keyword assignment, typed fragments, simple actions, parenthesized forwarding,
integer defaults, containment/opposites, cross-document references and the
existing availability/count/loan validation semantics. The Library starter is
unchanged. Titles here are identifiers or the keyword untitled, not free text.

Expressions uses ../current-actions/model.ecore and model-behavior.rcore. It demonstrates
ordinary delegation, grouped forwarding, assigned-current capture, branch-local
simple actions, keyword assignment and left-associated containment trees.
This is a KerML-shaped syntax demonstration, not full KerML execution. Actual
pinned grammar admission, implicit construction, general scope and X01-X18
qualification remain separately tracked.

Direct native/WASM transport verification against existing artifacts:

```sh
node tools/test_xtext_advanced_web_artifacts.mjs PATH_TO_RCORE_BEHAVIOR_PARSE PATH_TO_INTERSECT_WEB_WASM
```

Run from the repository root after building those artifacts from the same source.
The verifier performs no builds. It compares complete compilation and evaluation
responses, validates example values and links, rejects missing references,
duplicate loans and malformed expressions, and checks exact recovery afterward.
Production browser controls separately verify public UI and downloaded ZIP output.
