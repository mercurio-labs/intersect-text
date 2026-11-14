# Full generated Until oracle corpus

This verification-only corpus preserves eighteen exact authored Xtext graphs
and the original imported Ecore bytes. It combines the immutable fourteen-graph
Until context study with four separately frozen call-context discriminators.
It does not implement a lexer or award native support, full X05, or a parent gate.
The original seven-graph `full-generated-lexical` corpus is unchanged.

There are 228 planned requests. Sixteen graphs completed actual full language
generation and Java compilation, then executed all 204 corresponding requests in
both the original record and an independent fresh regeneration/replay. Each run
has 48 accepted resources, 156 rejected resources, and 60 rejected partial
models. The other two original graphs retain their exact validation error,
`The first rule must be a parser rule.`, failed MWE2 generation, and all 24
unexecuted requests. Separately named valid inheritance companions do not repair
or replace those failed graphs.

`inputs/cases.json` retains exact source identities, grammar order, entry grammar,
and every planned input. `observations.json` contains complete raw observations:
syntax/linking/validation/translation, generation and compilation stages, all
lexer tokens through EOF, invalid tokens and channels, resource diagnostics and
partial models, leaf spans, and the named Model parser result. UTF-16 values are
preserved through JSON escapes; no scalar substitution or token/error projection
replaces those observations. `positive-resources.json` is only the mechanically
checked view of the 48 actually accepted resource observations.

`emitted/` preserves 464 selected original record files byte for byte, including
ANTLR grammar and lexer/parser Java, generated setup/provider sources, GenModel,
workflow, and authored generator inputs. `recorded/` preserves each cohort's
original receipt plus raw record/replay observations, commands, toolchain pins,
artifact hash manifests, and complete process logs. `lineage.json` names those
immutable producers and receipts. No cached provider or inferred substitute
metamodel stands in for the authored imported Ecore.

Run the portable byte/stage/denominator checks without Java:

```powershell
python -B fixtures/xtext/full-generated-until/verify.py
python -B -m unittest tools.test_xtext_full_generated_until_evidence
```

An optional independent full replay requires an explicitly supplied JDK
21.0.6+7 and the sixty checksum-pinned Maven jars and pinned source archive in
`inputs/stack.json`. It performs direct EMF GenModel/provider generation from
the original Ecore, full Xtext StandardLanguage/MWE2 generation, fresh Java
compilation, generated standalone setup, raw generated lexer execution, actual
Xtext resource loading, and named `Model` IParser execution. Each runtime request
uses a separate Java process. Output must be new and outside `fixtures/`:

```powershell
python -B fixtures/xtext/full-generated-until/verify.py --replay D:/new-evidence/until-replay --jdk D:/dev/jdks/jdk21 --maven-cache C:/Users/YourName/.m2/repository
```

Comparison permits exactly one field difference: the absolute GenModel path
must first be validated as the exact owned output root followed by
`provider-<original-Ecore-SHA12>/provider.genmodel`. Only that field is replaced
for comparison. All tokens, errors, partial models, leaves, source identities,
UTF-16 spans, and failure stages must remain equal. Timing and raw generator log
equality are not asserted; the original logs remain pinned independently.

`provenance.json` pins the portable oracle files. Five separately owned native
consumer/control files are explicitly excluded from those oracle pins and have
their own native evidence; they do not change the eighteen-graph oracle contract.
The optional Java route is a verification tool only. Normal builds and native or
WASM production execution do not require Java. Repeated oracle agreement on these
finite examples does not establish a general Until decision algorithm.
