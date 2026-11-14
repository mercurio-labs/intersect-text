# Shared recursive callee prediction

Authored NativeV38 example using the unchanged current-actions Ecore/Rcore.
Two alternatives share a recursive object parser preceded by a long literal
prefix. A first-token predicate resolves the recursive conflict. The compiler
proves a bounded recursion witness inside the shared callee; it never follows
that callee's caller suffix to invent a witness. Work and depth ceilings remain
unchanged. NativeV37 retains its prediction-work rejection for this long form.
Short forms compare against existing recursive-predicate behavior. Native/WASM
controls cover exact model identity/span/containment, predicate rejection, replay,
limits and recovery. This is not general predicate or pinned KerML qualification.
