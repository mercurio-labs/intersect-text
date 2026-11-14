# Exact terminal-only parent composition

Terminals.xtext is copied byte-for-byte from org.eclipse.xtext 2.38.0's
org/eclipse/xtext/common/Terminals.xtext. Its original EPL-2.0 header is retained;
provenance.json records the Maven JAR/member and both digests. No production
Java dependency is introduced. model.xtext is an Intersect-authored child using
the unchanged terminal-spellings metamodel.

NativeV50 consumes the exact parent dependency and validates its terminal bodies
without requiring it to supply the child's parser entry. A selected executable
package still needs an entry. Direct and transitive inheritance and a child WS
override execute through the shared compiler/runtime, in ordinary and explicitly
materialized compilation. `model /* note */ Alpha;` constructs Model.name=Alpha;
with the authored WS override, `model~Alpha;` does too. Invalid text, malformed
shadowed parent bodies, missing/stale sources, standalone entry absence and the
original declaration/execution limits have rejection controls. Recompilation
retains exact source identities, bytes, model identity and spans.

Imported does not mean semantically verified: STRING recognition executes but
the current value includes its quotes. The test explicitly records `"Alpha"`
(including quotes) versus the upstream semantic value Alpha as an X06 gap,
not a compatibility pass. DefaultTerminalConverters.java binds STRING and its
qualified common.Terminals name to STRINGValueConverter; that converter removes
quotes in Xtext 2.38.0. These primary sources were inspected in the pinned source
JAR (SHA-256 aa3811517807cb0f55dcea62c5f85d7dcc3423262f8d5fa6a2fabe9c3e72a47b).
Full standard-terminal conversion, general Xtext and pinned KerML execution
remain unqualified. NativeV49's terminal-only parent rejection is preserved.
Declaration-only enum/datatype/fragment parents are not qualified by V50's
terminal-only path and remain separate X02 work.

Verification commands:

```text
cargo test -p intersect-language --test xtext_terminal_parents --locked
node tools/test_xtext_terminal_parents_wasm.mjs
```

The three shared Rust/WASM groups include the explicit converter disagreement.
Their passing status must not be interpreted as full upstream equivalence.
