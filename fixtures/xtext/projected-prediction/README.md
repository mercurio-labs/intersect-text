# Shared-prefix and significant-token prediction controls

This independently authored grammar has two alternatives, nullable shared calls,
ten branching fragment levels, ten forwarding fragments and a qualified-name
tail. The alternatives store different literal operator values. It exercises
the relevant call/decision structure without copying or replacing KerML rules.

The original `model.xtext` is 1,530 bytes, SHA-256
`e805338113fa890dd7ecaa4b4b0983942838e77e1c16742f6d8acedb0b57e292`.
The existing current-actions metamodel is explicitly migrated before execution;
its source file and behavior contracts remain unchanged.

NativeV52 retains its prediction-budget rejection. NativeV53 constructs the
independently expected left/right models through ordinary and materialized
compilation. Controls verify complete package/model replay within each schema,
exact original declaration identities across representations, literal values and
spans, ambiguous/left-recursive/malformed rejection, resource rollback and fresh
recovery. Additional authored controls cover standard STRING inheritance and
90 shared token spellings, exercising masks above 64 bits in real WASM.

```sh
cargo test -p intersect-acceptance --test language_xtext_projected_prediction --locked
node tools/test_xtext_projected_prediction_wasm.mjs
```

The model package's proof controls separately check the captured original
Connector graph, 841 independently specified finite-language pairs, exact caller
suffixes, nullable calls, 64/65 call depth, adjacent work limits, 128/129-symbol
fallback and unknown guards. The [batch report](../../../docs/archive/xtext-projected-prediction.md)
records the source revision and limits. Neither this fixture nor successful
prediction proves full KerML semantics or closes X13/X18.

The independently authored `nary.xtext` adds ten alternative owners, including
two alternatives with a shared trailing keyword. Its 2,166 LF bytes have SHA-256
`e1d3d0f9f3acc97cdab352310c49bdae0cce41dc0160efec01b9948c32600938`.
NativeV53 retains its budget rejection; NativeV54 uses retained
pair certificates. The same Rust controls check every branch's expected literal
model, nullable/shared-prefix forms, exact source/declaration identities,
ordinary/materialized parity, complete replay, malformed prefixes, budget
rollback/recovery and ambiguity/left-recursion rejection on native and raw WASM.
They also check inherited STRING boundaries and existing 90-symbol models.
The [n-ary proof contract and evidence](../../../docs/archive/xtext-nary-prediction.md)
retain the original-input and bounded-work requirements. This authored source
does not replace the pinned KerML grammar or grant KerML domain qualification.

The independently authored `filtered.xtext` moves the closing delimiter to its
caller, leaving a repeated member and optional result at the end of the body.
Its 1,621 LF bytes have SHA-256
`f5aaa853d7c2021e3307ee3cf2bca71065f9cc0a91e1ef72b7e2099bdf309cf2`.
V54 retains budget rejection; V55's conservative FIRST filter and grouped exact
closures construct the literal empty/member/result models on native and raw WASM.
The body owns its opener and member/result text; its exact element span excludes
the caller's closing delimiter. Shared controls preserve full source/declaration
identity, ordinary/materialized parity, replay, rollback, malformed rejection and
recovery. Recursive and overlapping variants remain rejected; an exhausted proof
is inconclusive rather than an ambiguity diagnosis. See the
[acceptance contract and evidence](../../../docs/archive/xtext-filtered-prediction.md).
This fixture does not replace the pinned input or qualify KerML domain behavior.

The independently authored `discovery.xtext` has 48 sequential nullable keyword
fragments followed by distinct operator assignments. Its 4,215 LF bytes have
SHA-256 `3b042d8e4868cc2baac21f5eb876d1592dd9397fbf70143e10d18edc28719666`.
V55 retains prediction-budget rejection. V56 bounds initial
discovery and certifies the complete shared callee alphabet before using its
disjoint continuations. All 22 independent section/width variants execute,
including every previously accepted member and signature-overflow cases. Shared
native/WASM controls verify exact identities, ordinary/materialized parity,
replay, malformed and unfenced/ambiguous/recursive rejection, budget rollback
and recovery. The [acceptance contract](../../../docs/archive/xtext-bounded-discovery.md)
records the verified bounded gate and remaining scope. No KerML domain behavior
or full framework support is awarded by this fixture.
