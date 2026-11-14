# Pinned KerML enum-rule requirement

`provenance.json` retains three exact declarations from Pilot revision
`692170b71867353b8f90341e61556f49a5beb0e5`, including byte spans and SHA-256,
and the corresponding untouched Ecore enum inventories. Classifier indexes are
source identities, not interchangeable names. Grammar alternatives are not in
the same order as Ecore literals: `FeatureDirection` lists in/out/inout while the
Ecore declaration order is in/inout/out. `FilterPackageMemberVisibility` maps
`[` to the private literal. Matching by token text, numeric value or ordinal
would therefore lose required information.

Imported snippets retain EPL-2.0 provenance and the upstream copyright header in
`UPSTREAM-NOTICE.txt`; the complete license is retained at
[LICENSE-Pilot](../../definition-conformance/mercurio/LICENSE-Pilot).
The independently authored `model.*` teaching fixture is separate from the
actual pinned grammar and uses its own namespace and source identity.

`baseline.json` records NativeV6's explicit enum-rule rejection and the separate
ID-to-enum assignment rejection. A control omitting the assignment constructs the
existing enum default with exact enumeration/literal declaration identities.
That proves existing enum infrastructure, not enum-rule consumption.

Replay without Java or network access:

```powershell
python -B fixtures/xtext/enum-rules/verify.py --upstream <pinned-pilot-directory> --native <rcore_behavior_parse.exe>
```

Consumption and enum-rule semantic verification remain unimplemented. The next
increment's acceptance contract is [D16](../../../docs/archive/xtext-day16-design.md).

## NativeV7 candidate

The isolated NativeV7 implementation consumes explicit imported enum return
rules, default keyword names and alternative keyword spellings. It binds names
through the existing Enumeration codec and returns EnumValue declaration IDs.
NativeV6's frozen source rejection remains unchanged. Four native tests cover
keyword-to-literal identity, alternate spellings, equal numeric values with
separate identities, defaults, invalid bindings, malformed resources, multi-source
rollback and exact execution/element budgets. All 22 native/WASM groups pass,
including enum archive replay and a rehashed forged-literal binding rejection.
Browser composition, broader final checks and complete D16 qualification remain
pending. No actual pinned KerML execution is claimed.

The official [Xtext grammar documentation](https://eclipse.dev/Xtext/documentation/301_grammarlanguage.html#enum-rules)
describes enum rules and multiple keyword aliases for one literal. The optional
`EnumEmissionOracle.java` additionally invokes the actual pinned Xtext 2.38
emitter over independent linked grammar objects. Five cached emission observations
retain the selected literal names/indexes and generated `.getEnumLiteral().getInstance()`
actions. Original-element and rule-name adapters reproduce generator prerequisites;
they do not substitute a native implementation or an invented expected emitter.
This is generator evidence, not a complete generated parser or a whole-language
Java comparison. All dependencies, harness bytes and observations are hash-pinned.

Replay it by adding `--oracle-jdk <jdk>` and `--oracle-artifact-dir <output>` to
the command above. Only cached Maven artifacts are used; no download or production
Java dependency is introduced. Existing Ecore duplicate-literal-spelling rejection
is preserved. Equal numeric values and distinct literal spellings remain separate
from duplicate spelling admission.
