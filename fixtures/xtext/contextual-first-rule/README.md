# Contextual first-rule evidence

This is an additive verification corpus for explicit NativeV61 source admission.
It retains 20 original source graphs and all 128 planned inputs. Validation,
generation, Java compilation, lexer, resource, named parser, native compilation,
model execution, archive rebuilding, and project transport remain separate stages.
There is no full Xtext/X03/X05/X06 gate closure or native Ecore import award.

`inputs/`, `oracle/`, and `oracle-producer/` preserve the pinned Xtext 2.38 original
record and independent replay. They include the two original rejected inherited
Until graphs, their unexecuted requests, the original enum metadata-helper NPE,
and the separate V2 helper recovery. All 16 generation failures and 94 unexecuted
runtime inputs per run remain present. See `oracle/README.md` for exact toolchain,
lineage and producer boundaries. Java is an optional independent oracle only.

`native/before-v60.json` and `native/after-v61.json` retain complete native
requests/results. The candidate replays all 240 original V60 responses and four
old inferred responses (including the fixed v56/v57 choices). Its 20 source graphs
compile 6/reject 14, while V60 compiled 12/rejected 8. Rejected-source input slots are
explicitly unexecuted. The 394 complete candidate calls include archive/model
replay, rehashed invalid source and cached-entry rejection, parameter companions,
and last-good source recovery. The actual full-generated runtime overlaps 29
candidate model inputs; all 29 acceptance decisions and all 7 accepted complete
object/class/feature projections agree. Fifteen other executed native library
inputs have no upstream runtime observation and are not awarded model equivalence.
Rejected partial/error streams are retained without claiming diagnostic equality.

`migration/` preserves the new authored Ecore provider, complete actual immutable
V60 migration report, draft, converted source, 14 correspondence records, issue
list, and explicit subsequent selection. All four identity-cohort native graphs
select the exact returned converted Rcore unchanged. The original Ecore is
preserved independently; migration is not native Ecore execution qualification.

Raw historical paths and bytes remain unchanged in recorded evidence. Only
non-executable capture metadata paths in portable `native/native-cases.json`
and parameter companions are adapted to relative owned paths. Full compiler
inputs are deep-equal to the original recorded recipes. Native consumers read
relative fixture paths and never execute a recorded artifact or Java path.

Run all portable integrity checks without Java or WASM:

```
python verify.py
```

Replay every complete native request through an explicitly selected WASM artifact
and 39 historical pure-project outcomes into a new directory outside this corpus:

```
python verify.py --wasm /absolute/path/intersect_web.wasm --replay /new/output
```

The replay uses byte-pinned copies of the observed pure-project modules. These
copies are verification-only historical inputs; actual Rust/Node consumers also
check the current Toolkit modules. Project transport is not a browser editor or
restart lifecycle qualification. A recorded `wasm.path` is never loaded implicitly.

The optional full Java oracle replay requires explicitly supplied pinned JDK and
Maven cache locations; jars are pinned, not bundled:

```
python verify.py --oracle-replay /new/oracle-output --jdk /jdk --maven-cache /cache
```

A null resolved classifier, scalar-fragment-only library, and syntax-clean
three-level object closure are not proven by this corpus. The original empty
middle grammar syntax failure remains unchanged. The separate same-named
`EString` parameter companion retains its prior imported-feature conflict in
both V60 and V61. None of these failures is repaired or relabelled as support.
