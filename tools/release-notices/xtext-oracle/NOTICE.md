# Experiment provenance and notices

The authored `Portable.xtext`, cases, preparation adapters and handwritten Rust
executor are Intersect experiment code. `runtime/` is an isolated unpublished
consumer, not generated code or a replacement production language backend.

`generated/InternalPortable.g` is the exact output of Xtext 2.38.0 for the authored
grammar. `generated/program.json` retains its compiler-owned control flow,
prediction graphs and template-derived action text. These generated artifacts are
distinct from the handwritten Rust executor. Their source and complete ordered
producer classpath coordinates/digests are in `generated/provenance.json`.
The Xtext notice is retained in [xtext-generator-about.html](notices/xtext-generator-about.html).
It identifies the upstream Eclipse Public License 2.0 terms; importing material
does not relicense upstream sources as Intersect MIT code.

The preparation tool uses `org.xtext:antlr-generator:3.2.1` and
`org.antlr:antlr-runtime:3.2`. Their retained notices are
[ANTLR generator license](notices/antlr-generator-LICENSE.txt) and
[ANTLR runtime DFA notice](notices/antlr-runtime-DFA-notice.txt). No JAR is
distributed by this experiment.

The executor's control-flow design was informed by ANTLR 3's tool Interpreter;
display-alternative mapping remains in the Java exporter. EOT consumption and
EOF priority follow `org/antlr/runtime/DFA.java` from the pinned 3.2 source JAR,
with two independent executable oracle observations. The runtime notice above
retains the attribution for that behavior adaptation. The Rust implementation
is handwritten and does not execute Java source. The second-stage typed
construction adapter resolves generated getter chains through the actual grammar
access object during preparation. It executes bounded native effects through
existing Intersect model/transaction APIs and scalar codecs. Predicate/group-state
evaluation, custom providers and recovery remain outside this profile.

`generated/oracle.json` retains independent generated-Java model and node
observations. Successful normalized containment graphs now have native/WASM
comparisons; source-node and error-recovery equivalence remain open. Broader
cached Pilot probes are separately reported
with their actual source-generator versions; they are not the 2.38 fixture.
