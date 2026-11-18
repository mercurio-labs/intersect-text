# Retained component sessions (prototype)

Enable the additive `intersect-text/components` Cargo feature. Model and Text
remain language-neutral: a host installs a `ComponentLanguage`, whose `analyze`
method parses, scopes, completes and validates sources over mounted Model images.
KerML provides the first real language adapter and hosted KPAR shell.

`ComponentSession::open` retains each saved source and its current editor text.
`edit` changes only the current buffer. Incomplete input stays available with
diagnostics and no checked model. An operational analyzer error also retains
the edited text, invalidates the previous model, and advances the revision.
The prototype reanalyzes project sources on each edit; it does not claim an
incremental parser or incremental project graph.

Adding a dependency has three stages:

1. The host acquires and prepares the artifact independently of the editor.
   This stage captures no project buffers and performs no project save.
2. `prepare_locked_usages` checks the exact image identities against a resolved
   dependency lock and analyzes the session's latest unsaved sources.
3. `adopt_usages` atomically installs the candidate model, diagnostics,
   references, usages and lock. It rejects candidates from another session or
   an earlier revision. Both current text and saved baselines remain unchanged.

Invalid project text can still adopt an admitted usage and retain its errors;
finishing the edit then analyzes it against that mounted library. Acquisition,
preparation and candidate failures leave the active session unchanged.

`prepare_usages`/`add_usage` are conveniences for unbound in-memory proofs. Once
a session has a dependency lock, replacement requires an explicit updated lock.
No API in this module reads files, accesses the network or saves editor text.

The opt-in `DraftLanguage::parse_component_into_with_settings` method uses the
ordinary parser and checked settings. It retains exact pending reference owners,
members, collection positions, spellings, target types and source spans for the
language adapter. The existing parser API still returns its original pending
count. No global-name fallback or SysML-specific rule was introduced in Text.

Focused controls live in `crates/intersect-text/tests/component_sessions`.
They isolate buffer and adoption guards with a deliberately minimal analyzer.
Real SysML parser/scoping and HTTP/archive tests belong to the language/host
integration harness in KerML; these guard tests do not substitute for it.

## Develop integration and API review

The exact committed Model revision is selected in every maintained source
manifest and standalone lock. Registry-only host-adapter qualification remains
separate, as recorded in text-compatibility.json. No sibling path patch is
required. Component-enabled native tests, strict lint, WASM and reviewed API
contracts have an explicit CI lane.

The default inventory also records the unresolved-reference occurrence API
already present on develop: StoredEvaluation has a defaulted, empty-omitted
unresolved_references field and the language-neutral UnresolvedReference type.
Its occurrence metadata does not claim resolution. The component feature adds
43 named exports without changing any existing default-feature declaration.
The adoption proposal keeps its owner and revision guards private.
