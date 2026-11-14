# S03 typing acceptance fixture

This independent model exercises typed Rcore functions, explicit module imports,
multiple inheritance, optional navigation, Result patterns, captures, exact numeric
literals, a label derivation and a score validation. The programs are type-checking
inputs; Rcore v2 execution remains unimplemented. They do not replace Library's
collection/inverse-query semantics or establish SysML qualification.

`bundle.json` retains these four exact source texts, SHA-256 identities and direct
dependencies. Source text is normalized to LF when generating this fixture. The
bundle is a `BundleCompilation`, so the host and browser can exercise the same
type-checking context and then report the explicit unavailable execution stage.
Native tests verify every retained source against its sibling file after newline
normalization, preventing a stale self-contained bundle from hiding fixture edits.
