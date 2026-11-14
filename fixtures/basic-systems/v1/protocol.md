# Receipt and adapter protocol v1

The protocol is draft host orchestration. Language semantics belong in the
language package and shared model APIs, never this runner or a browser test.

## Full grammar prerequisite

`qualifications` has `admission[LANE][LANGUAGE].receipt` containing an absolute
file path, byte count and SHA-256. Each immutable receipt must contain:

```json
{
  "state": "pass",
  "ok": true,
  "observation_complete": true,
  "lane": "native",
  "language": "kerml",
  "projection_or_reduced_grammar_used": false,
  "authored_rules": 284,
  "full_original_sources": {"relative-source-path": {"bytes": 1, "sha256": "EXACT"}},
  "dependency_edges": {"kerml": ["ecore", "expressions"], "expressions": ["ecore"]},
  "source_postguard": {"state": "pass"},
  "artifact": {"path": "ABSOLUTE", "bytes": 1, "sha256": "EXACT"},
  "raw_admission_result": {"path": "ABSOLUTE", "bytes": 1, "sha256": "EXACT"}
}
```

This is a shape illustration, not usable evidence: all expected source entries
from the exact language closure must be present. SysML has 551 authored rules
and dependency key `sysml`. Browser receipts also require a pinned static manifest
and a passing static postguard. The current ordinary admission example exits zero
for an observed compiler rejection, so exit code alone cannot satisfy this shape:
the actual raw report must have both `observation_complete:true` and `ok:true`.

Admission adapters must retain original raw reports, complete source manifests,
explicit policies, migration/correspondence, all setting/provider contracts and
Cargo JSON or normal WASM/static artifact provenance. Constructing this small
receipt cannot manufacture qualification absent the authoritative raw proof.

## Workflow execution

`adapters[LANE]` is an explicit argv array for an existing executable. One fresh
process receives one JSON request on stdin and returns one JSON response on
stdout. Stderr and stdout are retained, including nonzero exits and invalid JSON.
The request schema is `intersect.basic-systems.request.v1`, with `case`, `lane`,
all source texts/pins, contract and standard-library manifest pins. The portable
runner adds `upstream_input_root` only from the caller's explicit `--pilot-root`;
the committed contract contains no machine-specific default checkout path.
Without that option a baseline remains BLOCKED and no adapter runs. Per-case
steps run in order in one isolated project/session. Project replacement is atomic
and language validation is an explicit acceptance condition. State cannot leak
between cases.

The response schema is `intersect.basic-systems.response.v1` and contains
`case_id`, `lane`, `state:"pass"`, `observations` exactly equal to the complete
ordered expected list, `imported:true`, `consumed:true`,
`source_postguard:{"state":"pass"}`, and `excluded_result_fields:[]`.
Its `evidence` array contains entries `{role,file:{path,bytes,sha256}}` for at
least these roles: complete-raw-model, complete-raw-result,
complete-raw-diagnostics, language-context, native-or-WASM-artifact and
independent-semantic-truth. Source identities and spans use exact UTF-8 bytes.

Expected graph queries expose language concepts over the real full model;
they are not a substitute parsing grammar or a second semantic model. Query
name lists and direct/transitive generalization lists are lexicographically
ordered; connector endpoints retain declared order. `transitive_contains`
asserts the named basic ancestors while full ancestry stays in raw evidence.
Source target spans identify the declared-name token, not a fabricated source.
Qualified paths with `::` select declaration identities; usage feature paths
with `.` require language-owned feature-chain interpretation. A `bind-element`
step binds a stable real identity to a session handle; all post-rename steps use
that handle instead of looking up a stale qualified name.

Text emission occurs after editing the real model. Reparse must retain a complete
explicit correspondence between original/current/emitted model identities and
source revisions, with every old/new exact URI/hash/span preserved in evidence.
Reference targets must remain equivalent through that map. Changed source bytes
must never be relabelled with the previous source hash. XMI identity/provenance
retention remains an independent, stricter round-trip observation.

`failure.phase/category` are classification fields proposed for this harness,
not claims about current Rust error enums or exact upstream wording. Classification
must retain every original raw diagnostic, source identity, location and detail;
no generic unsupported error may masquerade as the requested semantic rejection.
Exact expected locations are authored requirements awaiting independent review.
Compiler-wide diagnostics must remain explicitly unlocalized until real ownership
is available; a runner must report this case failed or blocked rather than invent
the expected declaration span.

Before/after equality assertions require complete raw model comparisons, not
object counts or selected names. Interchange must preserve all supported objects,
identities, memberships, ordered references, types and validation results. Lossy
serialization or unsupported constructs must fail explicitly. The runner checks
literal observations and evidence pins; independent review of adapter implementation,
raw comparisons and truth receipts remains mandatory before adopting this draft
as a production gate. The executable therefore reports a matching adapter as
`candidate_pass_pending_review` and leaves imported/consumed/semantically_verified
false; a file carrying a truth role cannot award support by itself.
