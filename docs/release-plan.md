# Evaluation alpha release plan

Updated 2026-11-16. This fresh-history baseline contains the integrated alpha work
and approved cleanup. Earlier development history and component-design work remain
in private pre-dev storage and independently restored local backups. Release
publication and final artifact qualification are separate decisions.

## Release objective and scope

The coordinated `0.1.0-alpha.1` release is for evaluation and feedback.
Distribution includes public Model and Text repositories and three crates.io
packages: `intersect-build-support`, `intersect-model`, and `intersect-text`.
See [the registry procedure](registry-release.md) for standalone package preparation,
publication order, exact dependency pairing, and retained notices.

Model's application entry points are DynamicSchema/DynamicModel, Rcore packages,
checked editing, resources, persistence and supported execution. Text adds admitted
grammar compilation, parsing/linking, document analysis and model-to-text editing.
Publish only the feature/profile/platform combinations tested on the final pair.
Native and WASM are intended targets; minimum Rust is 1.88, and SQLite is native-only.
The browser demonstration does not establish a stable Web/Toolkit SDK.

Rust APIs may change between alphas, with migration notes. Application APIs receive
the strongest continuity effort; provider, preparation and raw-data contracts remain
provisional. Public visibility is not a recommendation for ordinary application use.
Keep reviewed API/signature inventories and independent consumer checks.

Saved formats remain explicitly versioned. Require within-release round trips and
clear refusal of unsupported versions. Cross-alpha data migration is not promised.
Retain authoritative definition sources and data so derived caches can be rebuilt.
This cleanup changes no format/profile identifiers, limits or decoding rules.

## Package count and public API decision

### Package and API boundary

The registry release includes `intersect-model`, `intersect-text` and the host-only
`intersect-build-support`. Other workspace crates remain development/host/example
components and are not registry products in this alpha. The release owner has
authorized public visibility of the active Model/Text repositories and crate
publication. Archived pre-dev repositories remain private.

Use the application facades documented in the guides. Advanced activation and host
contracts have an `extensions` entry point; specialized provider/storage modules
retain their owning paths. This is a reduced surface, not a claim of universal
minimality. API inventories count exported paths, including aliases.

## Candidate and evidence

The [baseline provenance](baseline-provenance.json) records the private source
revisions and reviewed export boundary. Model is established first; every maintained
Text dependency and lockfile must select that one exact fresh Model revision.
Release evidence records the resulting Model/Text commits without self-referential
commit IDs in their own source trees. A locally seeded Cargo cache is not proof that
the canonical GitHub repositories can serve these revisions; verify fresh fetching
after private cutover and before delivery.

Historical results belong to their original revisions and remain in pre-dev.
They are not fresh-baseline qualification. Existing source/browser ZIPs cannot be
relabelled as this tree. The [artifact inventory](alpha-artifacts.json) records what
must be rebuilt; it intentionally contains no final artifact hashes.

## Known limitations

* **Rhai historical accounting mismatch: non-blocking for this alpha.** Nine
  `binding_work` fields differ in the historical transaction replay (commit: 8,106
  versus 8,905); other report fields matched, and fresh native/WASM reports agreed
  in the earlier qualification. Exact historical accounting compatibility remains
  unqualified. Preserve the original goldens, failed comparison and execution limits.
  This exception does not waive other optional-feature or resource-limit failures.
* Grammar admission, parsing, execution, serialization and preserving edits have
  different boundaries. Do not claim arbitrary Xtext/Java customization compatibility
  or complete KerML/SysML semantics from generic library tests.
* Host adapters own physical I/O, cancellation, input bounds and atomic publication.
  Native SQLite and browser behavior need their own target-specific qualification.
* Large oracle corpora remain because current tests use them. Private historical
  documents, completed experiments and saved benchmark reports are omitted; those
  omissions do not authorize reducing test cases or changing expected outcomes.
* String errors and diagnostic prose are not stable machine-readable contracts.
  Advanced public APIs and large modules remain future maintenance work.

## Remaining release gates

1. Local preparation: the reviewed cleanup is integrated, required notices and
   fixture bytes are retained, and full-history plus working-tree backups have passed
   independent restore checks. Preserve component-design work separately from release.
2. Establish the fresh Model root, then pin Text and downstream consumers to
   that exact pair. Verify independent roots and canonical source identities. The
   private GitHub cutover must retain the old repositories as pre-dev archives;
   release evidence must confirm canonical fetchability after that operation.
3. Run formatting, strict lint, API/signature/serialization controls, scoped native
   and WASM tests, generated bindings, selected profile/optional-feature checks,
   notices verification and downstream consumers against the final source identities.
4. Rebuild source/package and focused browser artifacts. Verify extracted artifacts
   without original checkouts and requalify Rust 1.88; retain commands and results.
5. Measure the quick tier without competing builds. Broad workspace, oracle and
   language-library checks remain milestone work, with explicit scope and exclusions.
6. Finalize release notes, exact artifact hashes, notices and the advertised support
   matrix. Obtain release-owner authorization for remote cutover and publication.

No gate is satisfied merely by the presence of a workflow or historical report.
Hosted CI, untested platforms and full-gallery workflows remain unqualified until run.

## Roadmap

After evaluation feedback, prioritize API/error consistency and module decomposition
where actual consumer needs justify it. Preserve historical profile/source/format
contracts until a documented compatibility change permits retirement. Component-store
design proposals are separate future work and are not alpha release promises.
