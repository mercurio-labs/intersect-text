# Imported feature assignment compatibility

This corpus qualifies the private explicit NativeV66 source-compatibility increment. It does not close a full Xtext, KerML/SysML, or Rcore gate. Original sources, failed attempts, stage distinctions and old profile responses remain unchanged.

Three original grammars differ only in imported `Model.refs` uniqueness: omitted/default true, explicit true, or false. Each has six model inputs. `inputs/` preserves exact Xtext/Ecore bytes and identities. `oracle-record/` and `oracle-replay/` retain the complete pinned Xtext 2.38 generated-language stages and 18 runtime observations each. The actual pinned `EClassifierInfo` checker is preserved under `source-evidence/`; imported compatibility does not compare the uniqueness default used to create a new inferred feature.

Runtime distinctions remain explicit. The native and zero-import WASM results preserve imported flags and attempted reference order. Native validation reports a duplicate-value diagnostic for four true-unique duplicate inputs; the explicit-false inputs accept duplicates. The resolved upstream resources retain repeated references without that diagnostic, including after `IResourceValidator` and EMF validation. The isolated generated-list mutation probe rejects a second identical target even when its metadata says false. These are different observed stages, not full duplicate-handling equivalence. The unchanged original `FullRuntime` reads resource errors before its model observer triggers lazy resolution; its prelink `accepted` flag is not the final validation result. `validation-addendum-v2/` records the later linking and validation; v1's failed Java compilation remains preserved.

`negatives/` preserves six independent shape/mutability graphs. All six upstream language generations fail after grammar validation; their six runtime requests remain unexecuted. The original native nonunique-containment graph fails migration and remains unexecuted. The separate valid unique-containment companion is additive, not a repair.

The two original profile corpora retain 22 complete calls each, labelled V64 and V65 separately. Another 16 original negative-corpus calls remain exact. The current private public-WASM record has 116 complete calls and a fresh-instance replay, including all 18 complete native model results, five V66 source rejections, and three rehashed cached-schema forgeries followed by valid source replay. The native six-test command also checks computed providers through the existing explicit native context and parameter frontend; the older context-free ordinary JSON reconstruction rejection remains separate.

The actual application worker inherits V65's durable-document path under V66. Two controls pass in development and static production: all three source/model/archive recipes and original-source project save/fresh-worker reopen. True-unique duplicate analyses cannot publish an invalid replacement; false-unique duplicates remain valid. The legacy-dispatch v1 failures and complete observations are retained under `history/`.

The full-source probes preserve exact upstream inputs and independent closures. KerML uses `SysML.ecore`, `KerMLExpressions.xtext`, and `KerML.xtext`; its V66 next obstacle is `Grammar prediction budget exceeded`. SysML directly extends `KerMLExpressions`; its measured 108 + 443 authored declarations give a separate source policy of 551. Its next obstacle is the original `DefinitionBodyItem` clause `=> ownedRelationship += Import`, reported as unsupported `=` at byte 13768. Neither full grammar activates. The initial SysML probe selected a stale same-name KerML executable; it is explicitly invalidated and retained. The genuine v2 uses a unique package/bin name plus Cargo-reported source/executable identity. No source repair, arbitrary limit increase or domain-support award follows.

Run `python verify.py` for pure integrity/stage checks. Java is optional verification infrastructure, never a production dependency. With the exact pinned JDK 21.0.6+7 and the 60 pinned Maven artifacts, the unchanged producers can run into new output directories:

```text
python oracle-producer/produce.py --output NEW_ROOT --jdk JDK --maven-cache CACHE
python negatives/oracle-producer/produce.py --output ANOTHER_NEW_ROOT --jdk JDK --maven-cache CACHE
```

Only the exact owned provider GenModel location may differ when comparing complete main oracle observations. Historical absolute paths are provenance, not native compilation authority. Optional standalone native source-probe manifests use repository-relative dependencies and unique root package names; their source bodies are unchanged, but these portable path adapters have not received an additional build award.
