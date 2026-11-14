# Pinned default STRING converter evidence

The authored 16-case matrix invokes Xtext 2.38.0 STRINGValueConverter.toValue
with a null node. Ten cases produce exact UTF-16 units; six report converter
errors. Cases cover both quote styles, empty strings, standard escapes, Unicode
escapes and pairs, an isolated surrogate, unknown/octal/malformed escapes and
unclosed strings. This is converter evidence, not generated lexer/model proof.

`provenance.json` pins all executable JARs, cases, adapters and observations.
`source-review.json` pins the EPL-2.0 source JAR and reviewed member hashes.
DefaultTerminalConverters binds STRING and
org.eclipse.xtext.common.Terminals.STRING. This binding observation is source
review, separate from the executed converter observations. No upstream Java
implementation is copied; adapters are Intersect-authored and verification-only.

The initial command-line transport changed a supplementary Unicode character
to question marks on this Windows/JDK combination. That result was discarded.
The harness now writes exact UTF-16 units in ASCII to an isolated artifact file
and asserts every returned literal equals its source case before recording or
replay. Both final recording and a separate non-recording replay passed.

Replay using `string_oracle.py --jdk <jdk> --artifact-dir <isolated-output>`.
Java is not needed for production, normal builds or the native/WASM controls.

## Current native comparison: deliberately failing semantics

`pinned_string_converter_observations_expose_native_v51_gap` in
xtext_terminal_parents.rs consumes these observations. Ordinary/materialized
compilation and exact package replay retain source/model values and identity.
The seven terminal-parent groups pass natively and in real WASM, but this group
asserts current disagreements rather than awarding converter compatibility:

- Direct STRING retains quotes/escapes instead of the ten observed values.
- Four closed invalid-escape cases are lexically recognized without the
  converter errors observed upstream.
- Two unclosed cases already fail lexical recognition.
- The isolated-surrogate value is accepted by the upstream converter. Current
  runtime EString uses scalar Rust/JSON strings, so full value parity needs a
  shared representation decision; replacing the surrogate is not acceptable.

A `Text returns ecore::EString: STRING;` wrapper is deliberately tested beside
direct STRING. Datatype-parser accumulation preserves raw source spelling and
must not eagerly apply the child terminal converter. Existing independent
datatype-contract observations and assertions remain unchanged.

Next implementation should reuse the existing UTF-16 syntax string decoding
authority and add default converter selection in the language package's shared
text execution path. Preserve old-profile disagreement controls and distinguish
lossless decoding, model value support, converter errors and datatype raw text.
The shared-runtime lane has been asked about UTF-16 EString representation;
no core/schema behavior was changed by this evidence increment. X06 remains open.

## NativeV52: standard quoted STRING execution

V52 adds default conversion for STRING and explicitly selected
org.eclipse.xtext.common.Terminals.STRING calls. The language-package text
machine selects this policy; decoding delegates to the syntax importer's existing
UTF-16 implementation through `grammar::syntax::decode_string_literal`.
The latter requires exactly one complete quoted literal and preserves isolated
UTF-16 units. There is no second escape decoder or production Java dependency.

The shared native/WASM control compares all 16 oracle rows: ten decoded UTF-16
values match exactly and six invalid literals reject. At model execution, nine
representable values match; the isolated surrogate receives an explicit
unsupported model-representation diagnostic, with no replacement or publication
of partial model state. The shared-runtime lane confirmed that its current
batches do not change EString representation. Full lossless model parity needs
one design across scalars, core, semantic execution and wire adapters.

Both normal and explicit ancestor STRING calls execute through ordinary and
materialized packages. Exact source/package replay, model identity/spans,
rejection rollback, execution limits and recovery are checked. Datatype Text
still accumulates raw spelling; the same quoted terminal renamed QUOTED also
remains raw because its name has no default STRING converter binding. The
V50/V51 disagreement assertions remain unchanged. Existing composed Library and
KerML example assertions also run under V52.

Scope remains standard quoted STRING literals, not all custom rules named
STRING, arbitrary converter services, ID conversion, upstream partial-value
recovery, serializer behavior or full X06. No model value representation or
live browser deployment changes are included.
