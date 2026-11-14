# Enum-valued ordinary parser rules

Authored NativeV46 example, using the unchanged Ecore/base grammar from
`../enum-composition`. The child replaces the inherited enum rule with an
ordinary parser rule returning the same Shade type. `color red;` constructs the
same canonical Red enum value. `color missing;` matches parser syntax but fails
enum lexical conversion. An ordinary parser returning Shade cannot invent a
`warm` spelling; an explicit enum rule may map `warm` to Red.

The shared tests exercise all four parser/enum parent-child combinations in
ordinary and materialized compilation, explicit parent calls, nested enum-valued
parser calls, nominal values, source/shadow identities, exact package replay,
model identity/spans, limits and rejection/recovery. Earlier profiles continue
to reject the new ordinary enum-returning parser form.

Run `cargo test -p intersect-language --test xtext_enum_datatype_parsers --locked`
and `node tools/test_xtext_enum_datatype_parsers_wasm.mjs`. The WASM probe executes
the same three Rust control groups. It supplies no JavaScript semantics.

Provenance: the thirteen cached category observations and scoped adapter in
`../terminal-overrides/category-provenance.json` concern only
Xtext2EcoreTransformer.checkSupertypeOfOverriddenDatatypeRule, not full grammar
validation or execution. For default parser conversion, the pinned Xtext 2.38.0
source JAR's AbstractDeclarativeValueConverterService.java lines 228-233 selects
EFactoryValueConverter; that converter delegates to EFactory.createFromString.
Source JAR SHA-256:
aa3811517807cb0f55dcea62c5f85d7dcc3423262f8d5fa6a2fabe9c3e72a47b.
No upstream source is copied and no new Java oracle run is claimed. Intersect
reuses its existing imported-enum codec across native and WASM consumers.

Enum-valued terminals, datatype fragments, nullable/recursive datatype rules,
general value-converter extensions and scalar entry execution remain unsupported.
This increment does not close full X02 or establish full pinned KerML execution.

## NativeV47 terminal extension

`terminal.xtext` uses the same metamodel and example but overrides Choice with
an enum-valued terminal. NativeV47 retains its exact nominal enum key and uses
the same Enumeration::parse codec. Parser-to-terminal, enum-to-terminal and
terminal-to-terminal replacements work when the enum type matches; terminal to
parser/enum replacements and different enum identities reject. The existing
NativeV46 parser controls remain and also execute their matrix under V47.

Run `node tools/test_xtext_enum_terminals_wasm.mjs` for all thirteen shared Rust groups.
The native test target is unchanged. Tests include declaration-only inherited
entry, qualified parent calls with distinct spellings, parser calls to terminals,
canonical values, source/shadow identities, replay, limits and lexical failures.

Primary-source evidence for default terminal conversion is
org/eclipse/xtext/conversion/impl/DefaultTerminalConverter.java in the same pinned
source JAR: its factory retains the terminal EDataType and toValue delegates to
EFactory.createFromString. The cached category oracle remains limited to its
documented method; it is not an end-to-end lexer execution oracle.

An explicit limitation is preserved: if parent and overriding child terminals
recognize identical text, child token selection masks the retained parent call.
The test records its runtime rejection without claiming that the source is
invalid upstream. Distinct-spelling retained-parent calls pass. Broader lexical
arbitration, general datatype conversion contracts and the full X02 audit remain
open. The earlier statement that enum-valued terminals are unsupported applies
through NativeV46; NativeV47 supports the bounded forms described here.

Fresh verification refines that limit: terminal-overrides/terminal_identity_oracle.py
records Xtext's distinct child/parent token names and runs authored lexers with
the pinned ANTLR tool. Identical ordered definitions reject generation (error
208). Partial overlap generates and chooses the child for the shared spelling;
the parent is available for its unique spelling. The seventh native/WASM group
matches that priority. The remaining identical-case gap is early admission of
an unreachable terminal, not a need to retry a parent after child tokenization.
This is scoped naming/lexer evidence, not full Xtext grammar generation.

NativeV48 adds bounded admission-time diagnosis of never-selectable terminals,
using the same LexerPrediction graph as execution. It rejects the identical
retained-parent example at the original parent declaration span. Disjoint and
partially overlapping overrides still execute with the same priority, nominal
values, source identities and replay. Two earlier terminals can jointly shadow
a later alternative; the new union control verifies that case on ordinary and
materialized compilation. Exhausted proof work reports an explicit inconclusive
diagnostic, never a false unreachability conclusion. Older profiles retain their
existing controls unchanged.

This is a sufficient unreachability check, not a complete body-language proof:
early commitment, UTF-16 ranges and the existing Until approximation are retained.
A potentially selectable terminal is not certified to recognize every predicted
input. The new proof has a 10,000-work cap; existing NFA/runtime ceilings are
unchanged. No production Java dependency or runtime retry was added.

NativeV49 admits the upstream default empty hidden-token policy. The saved
default-hidden.xtext is byte-equivalent to the tested parentless source: it
accepts `colorred;` and rejects `color red;`, because its WS token is visible.
Declaring hidden(WS) hides whitespace; an absent child inherits that policy,
and hidden() in the child explicitly clears it. Six pinned flattening cases
execute in both ordinary and materialized compilation. Source bytes and exact
identities remain available after effective-policy normalization and replay.
NativeV48's absent-policy rejection remains as a historical control. Missing
parent dependencies and unknown hidden tokens still reject. Single-source
compilation does not invent the unavailable common.Terminals parent's policy.
