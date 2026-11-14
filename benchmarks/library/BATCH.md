# Library scale comparison contract

Measure the existing native evaluator and a generated Xtext/EMF parser on identical
UTF-8 Library text containing one Library and N-1 uniquely named Members. Sizes:
1,000, 10,000, 100,000 and 1,000,000 total model elements. A 100-element control
establishes successful consumption below native resource limits.

This batch adds host-only measurement tools, not language optimizations or new
compatibility claims. Keep production limits unchanged. Report rejection, timeout,
and process failure independently; never present rejection latency as parse time.
Intersect's measured operation is evaluate (parse, construction, linking, structural
validation, semantics); Xtext's is resource load (parse and EMF construction).
There are no references in this workload. Their timings are not equivalent stages.

The Java comparison uses a generated, inferred Library/Member metamodel for this
two-class syntax projection. It is not qualification of the complete imported
Library Ecore model, opposites, loans, or Rcore behavior. Generation and Java dependencies
are confined to this optional benchmark; native production/builds remain Java-free.
No temporary production Java exception is introduced.

Evidence: deterministic corpus hashes, exact counts and names, raw per-process
results, tool versions, process memory, and reproducible commands. Positive control
and production budget rejection are required. Run repository Rust checks and WASM
build. A successful Java generation/run is required before claiming Java results.
