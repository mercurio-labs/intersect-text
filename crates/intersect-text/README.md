# intersect-text

Xtext grammar compilation and text interpretation over `intersect-model`.
The model package owns shared structure and Rcore behavior; this package adds
text parsing, linking, and the supported grammar profiles.

Use `compiler::compile_rcore_project(rcore, xtext)` for a combined Rcore
structure-and-behavior source, or `compiler::compile_rcore_language(rcore,
xtext, behavior)` for explicitly separate sources. Both retain exact inputs
and compile Rcore directly into the shared definitions.

`evaluate` constructs and validates models through the same services used by
the model API. Unsupported grammar or semantic constructs return diagnostics;
successful extraction alone does not establish execution support.

This package supports native Rust and `wasm32-unknown-unknown`. It does not
require Java at runtime.

Application entry points are `compiler`, `evaluate`, `document`, `source_edit`,
`serialization` and `distribution`. The `extensions` module owns explicit
`RcoreGetters` activation and `evaluate_runtime_guarded` /
`evaluate_runtime_guarded_into` with `RuntimeGuardedEvaluation`. Their former
`rcore_getters` and `bounded_runtime` module paths are private. Existing linking,
scope, conversion, settings and preparation contracts retain their owning paths;
the extension rustdoc explains their source, lifetime and work obligations.
Internal scope-replay wrappers, combined lexer-policy wiring and conversion-aware
default-spelling plumbing are private. Use the checked compiler, serializer or
`TextServices` contract instead. See
[API boundaries](../../docs/language-guide.md#application-and-extension-boundaries).

This package has its own release version and declares `intersect-model`'s
compatibility requirement directly. The [compatibility record](../../docs/text-compatibility.json)
names the minimum tested model version/revision; a Cargo range alone does not
qualify untested revisions or language profiles. `SOURCE_IDENTITY` covers this
package's inputs. Use `compiler_identity()` for the installed model/Xtext compiler
combination and `compiler_cache_identity` for profile/provider-specific caches.
The historical ordinary archive descriptor stays frozen across model releases.

The [staging commands](../../tests/text-only/README.md) prepare independent
package/consumer inputs, locks, notices and optional vendored registry sources.
The [release plan](../../docs/release-plan.md) records final-pair native/WASM,
browser, minimum-Rust and isolated-artifact gates. Earlier qualification remains
with its exact pre-dev revision; the curated tree needs its own final evidence.

`serialization::CanonicalSerializer` and `SourceEditSession::plan` provide
shared grammar-driven model-to-text generation and preserving edits for admitted
Xtext languages. Defaults come from compiled grammar/scalar definitions; every
output is parsed, linked, validated and compared with the persistent model.
Native `TextServices` install conversions, forward/inverse scope, computed
getter/setters, transient selections, unassigned text and formatting.
Preserving edits reuse authored spellings, comments and layout across structural
and resource changes, with stable model IDs and opaque undo/redo checkpoints.
See [the general contract](../../docs/text-editing-guide.md) and Toolkit's
`/model-text.html` for the independent-language consumer. Grammar admission and
full language qualification retain their own explicit limits.

The bounded compatibility `source_edit` helpers prepare changes to existing
single-token stored primitive attributes and noncontainment references, including
incoming spelling updates on rename. `SourceEditSession` retains an accepted
model/source baseline through `DocumentLanguage`; `plan_with` asks a language-owned
callback for each replacement spelling and reparses, relinks and validates every
source. `PreparedSourceEdit::finish` checks the exact current source/model baseline
before returning source patches, output text and refreshed model/document observations.
`SourceEditSession::adopt` advances a session with checked source maps and resource
revisions for repeated edits. Hosts must also check their active language revision
and publish within one atomic boundary.

`plan_append_with` adds one new leaf object to an existing ordered containment
collection using a checked document anchor and language-owned placement. It
reuses the canonical renderer, inserts text without replacing any original bytes,
and verifies that existing tokens retain their semantic owners. Parser identities
are reconciled to stable candidate IDs, with an explicit correspondence for raw
parse-stage evidence; subsequent edits retain model handles and source maps.

`plan_delete` removes a leaf's significant tokens while retaining hidden text;
`plan_move_with` moves an exact leaf declaration within one resource. Both use
the checked core structural operations and preserve surviving token ownership.
Opaque checkpoints support verified restoration with monotone resource revisions.

`serialization::CanonicalSerializer` adds bounded grammar-driven output for
API-created models, with language-owned terminal/reference spellings, full
parse/link/validation and complete graph correspondence. Run the two-resource
Library demonstration with:

```sh
cargo run -p intersect-library --example library_text --locked
```

It generates text, adds a Loan, and preserves authored text through repeated
scalar edits and an insertion-only Loan patch. The `intersect-library` package
also exercises reference edits/renames, leaf deletion and same-resource movement.
Toolkit's `/library-text.html` hosts the shared services with retained proposals
and undo/redo. Python and disk publication remain future work.
See [the batch contract](../../docs/text-editing-guide.md) for the supported profile.
