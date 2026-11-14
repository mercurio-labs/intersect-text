# Explicit local imported type scope

Authored NativeV45 example. `model valid red ref valid` constructs a Model,
stores the exact Red enum value, and links target back to that Model. The
explicit class return, action, cross-reference, enum return, datatype parser
return and terminal return all use unqualified names from aliased local imports.

The shared Rust test also exercises qualified names, the existing inferred
return path, a secondary-package-only grammar, inherited rules, and a terminal
override child without local imports. Ambiguous names, missing classifiers,
missing local imports and parent alias leakage reject. Ordinary and materialized
compilation share the same checked source classifier inventory; unused runtime
types cannot disappear from ambiguity checks. Exact definition sources, enum
identity, model identity/span, reference target, replay, malformed text and
execution limits are asserted. NativeV44's historical scope behavior is retained.

Run `cargo test -p intersect-language --test xtext_unqualified_imports --locked`
and `node tools/test_xtext_unqualified_imports_wasm.mjs`. The latter executes the
same four Rust control groups in real wasm32-unknown-unknown WebAssembly.
The fourth preserves both parent names and spans in the syntax AST but rejects
multiple parents during native admission, matching the pinned validator.

Independent primary-source reference: Xtext 2.38.0
`org/eclipse/xtext/xtext/XtextScopeProvider.java`, classifier and local metamodel
scope methods, in the Maven source JAR with SHA-256
aa3811517807cb0f55dcea62c5f85d7dcc3423262f8d5fa6a2fabe9c3e72a47b.
No upstream implementation is copied and no new Java oracle execution is claimed.
For multiple parents, `XtextValidator.checkGrammarUsesMaxOneOther` in the same
source JAR rejects every parent after the first (lines 189-198). A syntactically
representable list therefore does not establish valid multiple inheritance.

This does not close full X02, provide general datatype converters, infer complete
generated metamodels, or establish full pinned KerML execution. Production uses
the shared Rust compiler/runtime and requires no Java.
The existing inferred-return path still requires the selected package import;
the relaxed header rule applies to explicit type scope and inherited entries.
