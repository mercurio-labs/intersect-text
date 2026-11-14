# Documents and model-to-text editing

Text editing uses the same grammar, model operations, linker and validation
services as analysis. The goal is to publish changes whose source and model
agree, while preserving authored text where the selected service can do so.

## Choose the operation

| Service | Use it for |
| --- | --- |
| `document::DocumentLanguage` | Prepare a language and analyze source with document/model associations |
| `serialization::CanonicalSerializer` | Produce text for representable models and verify the result |
| `source_edit::SourceEditSession::plan` | General grammar-driven preserving edits under admitted contracts |
| `plan_with` and bounded helpers | Explicit language-controlled spelling/placement for supported token and leaf edits |
| `TextServices` | Native customization: conversions, scope/inverse scope, computed settings, transient/unassigned text and formatting |

Do not infer general subtree support from a bounded helper or restrict the
general service to the older leaf-only contract. Check the selected method,
grammar profile and provider requirements. Custom Java/Xtend implementations
require equivalent native services.

## Generation and preservation

Canonical generation selects grammar spellings for representable state. Output
is parsed, linked, validated and compared with the intended persistent model.
Unrepresentable state should produce a contextual error rather than a guessed
serialization. Customized conversions and reference spelling must work in both
directions.

Preserving edits start from an accepted source/model baseline. They reuse
authored spelling, comments and layout where verified correspondence allows it.
They must preserve stable model identity through reanalysis and reject stale
baselines. Multi-resource operations need explicit ownership and resource revision
checks; unchanged resources should remain byte-for-byte unchanged.

## Publishing an edit

1. Capture the active language revision, source revisions and accepted model.
2. Prepare a candidate using checked model operations and installed text services.
3. Generate/plan and inspect diagnostics; failed work must not advance the baseline.
4. Verify the proposal against the current baseline, including host language and
   source revision checks.
5. Publish editor/disk changes and adopt the refreshed session within the host's
   atomic boundary. Opaque checkpoints support the documented undo/redo contract.

Library code cannot make arbitrary editor or filesystem writes atomic on behalf
of a host. Prepare/finish/adopt are not a substitute for the host's concurrency
and publication checks. Keep cancellation and memory controls at that boundary.

## Try it

```sh
cargo run -p intersect-library --example library_text --locked
```

The example generates text from a model, adds a Loan, and exercises edits while
retaining comments and layout. Toolkit's `/library-text.html` and
`/model-text.html` are reference consumers of the shared services. Native, Node
WASM and production browser evidence remain separate release gates.

## Provider and publication contracts

`DocumentServices` borrows executable providers for a request. `SourceEditContext`
binds the selection to an accepted session; `TextServices` owns a selection for
retained hosts. These capability identities are not serialized. Providers must
remain semantically stable while installed; replacing the language/provider
selection requires a newly accepted session. Missing or foreign contexts must be
rejected before inverse callbacks run.

Conversion providers must support the intended parse and inverse-format boundary.
A custom forward scope needs explicit inverse spelling for the exact target object.
Old spellings may be reused only when verification reconstructs the intended graph.
Computed grammar-assigned values need matching getter/setter or derived services;
provider names alone never activate executable semantics.

Custom formatters remain subject to byte limits, parsing, linking, validation and
graph comparison. Existing authored trivia takes precedence where it remains valid.
If a moved comment has no legal destination, report a refusal rather than dropping
it silently. Preserving edits reuse semantic identities and actual grammar origins;
they do not introduce a second parser or language-specific mutation engine.

Transient selections define the persistence view without mutating the complete
checked model. The full graph is validated first. Comparison includes persisted
values, references, assignment state and grammar-assigned computed values. Runtime-only
state can survive an edit even when it requires no text patch; resource/session
revisions still matter. Transient state must not conceal incorrect generated values.

Resource changes use the proposal's complete source set; a list of byte patches
alone is not authority to add or remove resources. Removed resources must be empty.
Representable empty object entries retain their resource and optional unassigned
syntax; datatype scalar entries are a different contract from an empty object model.
An arbitrary multi-root model is not silently wrapped in an invented syntax object.

Canonical generation and preserving edits remain bounded searches with explicit
refusal. Undo/redo uses opaque checkpoints tied to the accepted session. A host
must reject stale language/model/source state and publish atomically before adopting
the proposal. Invalid authored text may remain visible in an editor without being
accepted as a valid model or replaced by an older pending proposal.

These contracts apply only to admitted grammar and service combinations. The
[language guide](language-guide.md) describes profile/inference boundaries; the
[release plan](release-plan.md) records current qualification work.
