# Terminal cardinality and return-identity oracle

This verification-only corpus keeps thirty original source recipes and a distinct
same-source inherited-library workflow. It does not require Java in production.
The native source recipes and real consumers live in
[the shared native/WASM controls](../../../crates/intersect-web/tests/terminal_cardinality.rs).

`oracle-corpus/` is the immutable original extraction, including every original
record/replay artifact, failure, partial, timeout and unexecuted probe. Its own
manifest and verifier remain unchanged. `historical-workflows/` separately keeps
all 558 raw artifacts from earlier inherited workflow attempts: attempt1 fails
javac; attempts2/3 compile but their runtime processes fail without complete
lexer/parser/resource observations. These are not rejected model observations.
The distinct successful a4 workflow never erases those failures or the original
all-StandardLanguage workflow failures.

`portable-replay/` contains the unchanged frozen comparator plus a separately
pinned path-discovery wrapper. It invokes the original source-loaded producers,
original provider/GenModel generation, StandardLanguage/MWE2, javac, generated
lexers, named parsers and resources. `verification/` records the completed fresh
portable replay receipt, preexecution pins and commands. Its generated output
trees remain in their separately recorded ignored evidence directory and do not
replace the original corpus.

Pure integrity and complete original record/replay comparison need only Python:

```sh
python -B fixtures/xtext/terminal-cardinality/verify.py
```

Optional fresh full generation requires the originally pinned Windows JDK
21.0.6+7 binaries and all sixty pinned Maven jars. Use a new external output root:

```sh
python -B fixtures/xtext/terminal-cardinality/verify.py --replay NEW_OUTPUT --jdk PINNED_JDK --maven-cache PINNED_MAVEN_CACHE
```

The only generated-observation comparison exception is the exact absolute owned
`provider-<EcoreSHA12>/provider.genmodel` output location. Full raw UTF-16 token,
diagnostic, named-parser/resource model, partial and leaf observations remain
intact. Original thirty-case results are 212 planned inputs, 94 runtime attempts,
85 complete observations (31 accepted, 54 rejected, 11 rejected partials), nine
timeouts and 118 unexecuted probes. The separate inherited workflow contributes
ten complete observations (two accepted, eight rejected, one rejected partial).
These are scoped evidence counts, not closure of a parent gate.

Resolved-TypeRef metadata has its own unchanged original twelve-case record and
replay. A separately pinned loader-binding addendum maps each original logical
provider URI to relocated exact bytes through EMF URIConverter. Both the external
and portable CLI execute fresh compilation and replay with exact complete
metadata equality and no normalization. Duplicate mappings, mismatched bytes and
escaping physical paths reject. The optional metadata command is:

```sh
python -B fixtures/xtext/terminal-cardinality/oracle-corpus/oracle-producer/metadata-uri-binding/replay.py --output NEW_METADATA_OUTPUT --jdk PINNED_JDK --maven-cache PINNED_MAVEN_CACHE
```

Native compilation uses the explicit existing canonical Rcore structure adapter;
this corpus does not award native Ecore import, raw error/recovery equivalence,
full X04–X07/X15 acceptance, or S09 closure.
