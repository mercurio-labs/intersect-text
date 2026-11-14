# Building a text language

Start with the complete [small-language example](../README.md#a-small-language).
It combines an Rcore metamodel and an Xtext grammar, parses a source, and reports
the resulting model. Production compilation and execution do not require Java.

## Select the compilation path

### Application and extension boundaries

Use `compiler` and `evaluate` for application compilation/analysis, `document`
and `source_edit` for editor state, `serialization` for checked generation, and
`distribution` for source-authoritative archives. These are the starting APIs.

Host and language extensions have separate obligations: `extensions::RcoreGetters`
activates source-checked getters; `extensions::evaluate_runtime_guarded` and
`evaluate_runtime_guarded_into` consume runtime admission and the caller's budget.
`RuntimeGuardedEvaluation` also lives under `extensions`. The former
`rcore_getters` and `bounded_runtime` implementation modules are private.
Existing `linking`, `rcore_scopes`, `rule_conversion`, `text_services`, `setting`,
`preparation`, `grammar`, `inference` and `draft` contracts retain their paths;
the extension rustdoc describes their source, ownership and accounting duties.

Scope-replay convenience wrappers used only by internal tests, the combined
lexer-policy helper and conversion-aware default-spelling plumbing are private.
Conversion-aware serialization remains available through the checked serializer
and `TextServices`. KerML's rule-conversion validator remains public because it
is a real language extension contract. This reduction changes visibility and
paths, not grammar profile IDs, serialized formats or execution limits.

| API family | Purpose |
| --- | --- |
| `compiler::compile_rcore_project` | Combined Rcore structure/behavior plus text syntax |
| `compiler::compile_rcore_language` | Explicitly separate Rcore structure, grammar and behavior sources |
| Profile/bundle compilation | Exact inherited/imported source graph, selected profile, providers and budgets |
| `inference` / `ordinary` | Supported grammar-driven definitions and source-authoritative preparation |
| `document::DocumentLanguage` | Prepared document analysis and model-aware editing |
| `draft` / `preparation` | Advanced source-bound runtime contracts used by language packages |

These paths share Model services but do not promise identical feature admission.
Convenience compilation defaults to `WorkbenchV1`. Advanced grammars select a
Native profile explicitly. Retain that profile through save/reopen, archive
transport and replay; do not silently upgrade an old language to a newer profile.

### Qualified profile boundaries

The standalone [profile consumer](../tests/text-only/src/profiles.rs) uses public
APIs without the workspace test harness. These bounded controls run both
natively and in two fresh zero-import Node WASM instances:

| Profile | Positive control | Rejection / limit |
| --- | --- | --- |
| `WorkbenchV1` (default) | Independent Rcore/Xtext language parses an item and retains its name | Missing required name produces diagnostics |
| `NativeV80` (explicit) | The same ordinary language parses under the selected profile | Malformed input fails; the NativeV81 typed scalar return is refused |
| `NativeV81` | `ELong` maximum value survives as tagged, lossless `int64` JSON | Invalid and overflowing integers fail; NativeV82 unordered groups are refused |
| `NativeV82` | Bounded unordered members parse in either order with the same values | Duplicate members fail; no claim of arbitrary Xtext compatibility |
| `NativeV1`–`NativeV79` | Existing serialized profile IDs round-trip unchanged | This lane adds wire-identity coverage only, not semantic qualification for every older profile |

All 83 retained IDs (`WorkbenchV1` plus 82 Native profiles) round-trip. Unknown,
wrong-case and out-of-range IDs fail instead of silently selecting a default.
The independent native/WASM consumer and integration controls exercise these
paths. Their test sources remain in this baseline; qualification must be rerun
on the final pair. Browser archive/save/reopen workflows are a separate gate.

## Signatures and serialized packages

`Package`, `BundleCompilation` and `XtextInferenceInputs` are data contracts.
Their public fields support authoring, inspection and cross-crate adapters;
`Package` being deserializable does not grant admission or executable authority.
`PreparedLanguageDefinition` has private checked state and cannot be obtained
with `Deserialize`. Preserve borrowed-view lifetimes and explicitly supply host
providers and limits when rebuilding. Diagnostic text and `String` errors are
human-readable explanations, not stable machine identifiers.

| Data | Compatibility and validation policy |
| --- | --- |
| `Package` JSON | Version 2 is accepted by replay. The DTO tolerates unknown top-level fields, which are discarded; duplicate known fields fail. Decoding an unsupported numeric version can succeed, but replay rejects it. Cached schema/program data is rebuilt from retained sources. |
| Ordinary/inference recipes | Unknown fields fail. Optional fields retain their declared defaults/omission rules. Source identity, selected profile and host limits are checked by the matching compiler. |
| Grammar profile | Preserve the exact retained ID. Unknown identifiers fail; a higher profile number is not a blanket compatibility promise. |
| Language archive | Current ordinary manifest/package format 2; strict manifest and runtime requirement records. Import validates file inventory/digests and compiler/profile requirements, then recompiles retained sources. Keep legacy descriptors separate from the current crate version. |
| Model data and binary caches | Follow Model's separate snapshot/interchange contracts. A source archive is not permission to trust arbitrary cached execution metadata. |

Hosts should use the supported import/replay/preparation boundary for loaded
data, rather than treating a decoded DTO as a checked language. `usize` byte and
work limits must fit the receiving target and remain below independently chosen
host ceilings. The new controls exercise valid round trips, future/duplicate
versions, unknown recipe/profile fields and cached-schema tampering on native
Rust and WASM. No existing wire ID or format version changed in this review.

The [release plan](release-plan.md) records the signature and Serde declaration
guards. They do not inspect custom serialization code bodies, prove ABI/semver
compatibility or qualify every profile and browser workflow.

## Source authority and inference

Keep original text, source URIs/digests, declared dependencies, selected grammar
and namespace choices together. Resolve external files in the host and supply
the exact bundle to the compiler. Replayed source must agree with the recorded
identity; deserialized derived metadata alone does not establish executable
support.

Inference produces model definitions for admitted grammar forms. Generated
previews are outputs, not replacements for the authoritative input grammar.
Retain the original inference request when saving projects or rebuilding after
worker restart. Native generation, provider activation, browser source editing
and full language qualification are separate capabilities.

## Analysis and diagnostics

`Source` pairs a logical URI with its text. Evaluation constructs models, links
references and reports diagnostics under the selected policies. Use prepared
document APIs when locations, syntax recovery and subsequent edits matter.

Check diagnostics before treating a result as a valid model. Unresolved or
ambiguous references, unsupported syntax and exhausted work limits need distinct
handling. Editor offsets must respect UTF-8/UTF-16 boundaries. A syntactically
accepted grammar is not evidence of domain semantics or arbitrary Java/Xtend
customization support.

Select limits for the host. Native evaluation defaults remove several size/work
ceilings; these are not memory quotas. Hosts still own admission, cancellation,
provider lifetimes and atomic publication.

## Runnable consumers

```sh
cargo run -p intersect-library --example library_text --locked
```

The Library consumer demonstrates multiple resources, references and text/model
round-trips. Toolkit uses the shared WASM host; see the
[browser startup instructions](../README.md#try-the-browser-toolkit).
KerML/SysML domain semantics belong to the separate language repository.

## Detailed reference and evidence

For the prerelease, prefer the compiler/document/serialization facades for normal
integration. Draft/preparation and versioned prediction internals remain advanced
contracts; exporting a Rust type is not a promise that all its modes are stable.
The API review currently has these concrete controls:

| Path | Positive / refusal evidence | Release boundary |
| --- | --- | --- |
| Ordinary and inferred packages | [Text-only consumer](../tests/text-only/src/lib.rs): independent ordinary/inferred language controls | Default-feature native and Node WASM artifact rehearsal |
| NativeV80 guarded draft language | [Release feature corpus](../crates/intersect-text/tests/release_features.rs): enum rules, actions, linking and missing/ambiguous references | Exact corpus and production draft path; not all Xtext forms or profiles |
| General model-to-text editing | [General text controls](../crates/intersect-text/tests/general_text.rs): subtree/resource edits, retained comments/IDs, explicit scope contexts, stale proposals and bounded search | Requires representable syntax and the original provider context |
| Source-bound preparation | [Preparation controls](../crates/intersect-text/tests/preparation.rs): source/correspondence tampering rejects without mutating valid inputs | Advanced contracts; source identity remains authoritative |
| Browser Toolkit / archives | Profile-aware help and native/Web compilation | Browser save/reopen/restart and delivered production assets still need their own qualification |

The [release plan](release-plan.md) records the remaining qualification gates.
Native regression evidence does not qualify browser interactions or every Native
profile. Preserve the exact profile and source bundle; never infer support from
the highest profile number alone.
