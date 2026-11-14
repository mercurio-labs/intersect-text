# Declaration-only grammar libraries

These Intersect-authored sources exercise NativeV51 composition of a child model
grammar with enum, datatype and typed-fragment parent libraries. They reuse the
unchanged enum-composition Ecore/rcore_behavior definitions. The fragment case replaces
the child's `shade=Choice ';'` with `Tail` in both native and upstream controls.

Imported: exact source bytes and dependency/declaration identities survive
package replay. No model entry is invented for a declaration library.

Consumed: the selected child executes `color red;`, producing the nominal Red
enumeration value through the shared native/WASM implementation. Tests cover
ordinary and explicit materialization paths, malformed input, standalone entry
rejection, invalid shadowed declarations, identities/spans and existing budgets.
NativeV50's historical rejection is retained.

Independent evidence: LibraryOracle invokes pinned Xtext 2.38.0 IParser and
GrammarUtil.findRuleForName. It supplies the exact authored parent link because
IParser alone does not link proxies. All three sources parse and inherited
selection identifies the parent declaration. This is syntax/selection evidence,
not full upstream validation, generated execution or semantic parity.
provenance.json records every JAR, adapter, source and observation digest.
Java is used only for this optional verification workflow.

Run `library_oracle.py --jdk <jdk> --artifact-dir <isolated-output>` to replay.
The native test is `xtext_grammar_libraries`; real WASM uses
`node tools/test_xtext_grammar_libraries_wasm.mjs`.

This bounded increment does not close X02 or qualify arbitrary datatype
converters, scalar model entry execution, full Xtext or actual pinned KerML.
