# Repetition with a shared following prefix

This authored NativeV30 example reuses `fixtures/compatibility/graph.ecore` with
empty unique-name semantics. Repeated `a b` pairs append `b` labels; the final
`a c` writes the required name. Prediction distinguishes enter and exit using the
following token instead of trying a model mutation and backtracking.

The same native/WASM controls cover optional, zero-or-more and one-or-more forms,
exact model values/spans, source/package replay, malformed input, budget rejection
and recovery. Ambiguous repeat partitions and non-consuming repetitions reject.
This is not a full pinned KerML or general Xtext prediction qualification.
