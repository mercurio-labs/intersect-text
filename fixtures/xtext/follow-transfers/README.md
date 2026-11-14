# Compiled FOLLOW transfers

Authored NativeV37 example reusing compatibility/graph.ecore unchanged. Thirty
typed fragments each consume 120 keywords and ultimately assign one Node name.
The same FOLLOW walk derives constant tokens and caller dependencies once per
rule, then propagates token deltas over those dependencies. The work ceiling
remains 10,000; NativeV36 retains its repeated-walk budget rejection.
Tests cover exact source/model identity and span, replay, malformed input, runtime
limits/recovery, genuine ambiguous continuations and native/WASM execution.
This is not full Xtext or pinned KerML qualification.
