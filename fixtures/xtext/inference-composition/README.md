# Source-loaded Xtext inference composition evidence

This independently authored matrix extends the full X03 contract; it does not
replace the 41 frozen parentless cases or close X03. Pinned Xtext 2.38.0 loads
31 complete multi-source inputs (66 grammar sources): 20 accepted entry/package
outcomes and 11 rejected outcomes. All 35 parent structural before/after snapshots
are unchanged. Four parent metadata snapshots gain source-origin correspondence;
those additions are preserved separately from structural mutation. The original
imported Ecore structure also stays unchanged; one case adds an Assignment origin
to an original imported feature. The complete after-snapshot and the original
unchanged/mutated metadata classification are both retained.

The oracle uses real XtextResourceSet loading, linking, inferred definitions and
full grammar validation. It never wires parser AST references or manufactures
Ecore outputs. Initial and final source snapshots preserve all diagnostics, parent
and call bindings (with declaring grammars and spans), rule return types and full
generated package/classifier/feature/enum metadata. Referenced packages retain
name/namespace/prefix identity; parent definitions have their own full generated
snapshots and standard Ecore is independently hash-pinned. Partial outputs of
rejected inputs do not award acceptance. All authored sources are ASCII, so the
captured UTF-16 offsets also equal UTF-8 byte offsets for this matrix.

Controls cover inherited generated model/enum calls, implicit and explicit
parent-return overrides, subtype creation, reused inherited features, original
imported types, super calls, same-name classifier identities, shared aliases,
import-only children, three levels of generated packages, terminal overrides,
nonunique cross-references, and rejected sealed mutations, enum additions,
terminal type changes, namespace duplication, unresolved short-name imports,
empty syntax and multiple grammar parents. Multiple-parent partial inference is
retained alongside the pinned validator rejection, not claimed as support.

Native follow-through must infer every selected grammar in its own exact
source/metamodel context, preserve parent generated identities and sealed
features, and share the existing source composer and runtime services. Add exact
dependency, rejection/recovery and real native/WASM model consumers. Generic
archive/browser adoption and the complete original X03 audit remain required.
The 31 cases are a minimum evidence matrix, not a smaller gate definition.

`provenance.json` pins every local fixture component, all seven original
parentless fixture files, the cached dependency JARs, inspected upstream source
members and the frozen upstream lock. The metadata serializer is reused from the
unchanged parentless oracle. The read-only verifier checks those identities and
compares every complete observation; it has no recording or dependency-download
mode. Replay to a separate artifact directory with an installed JDK:

~~~sh
python -B fixtures/xtext/inference-composition/verify.py --jdk /path/to/jdk --artifact-dir /path/to/oracle-output
~~~

Portable integrity controls run with the ordinary Python tooling suite. Java is
only an independent verification oracle. This evidence changes no production
compiler and awards no native/WASM/browser inference-composition support.
