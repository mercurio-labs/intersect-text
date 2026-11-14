This is the independent upstream oracle portion of the contextual first-rule
fixture. It preserves twenty exact source graphs, two explicit provider Ecore
cohorts, the original complete record and independent replay, and the additive
metadata-only adapter repair. Java is an optional verification prerequisite;
production does not use it.

Each full run retains 775 raw artifacts plus its original manifest. Each
metadata V2 run retains 121 artifacts plus its original manifest. Files are
copied byte-for-byte, including generated Java, ANTLR grammars, class files,
GenModels, logs and original absolute paths. The two initially failed enum
metadata calls remain failed in both full runs. The separately named V2 adapter
fixes optional Boolean observation boxing and replays all twenty classification
rows without changing any source graph or validator.

There are 128 planned model probes per full run. Sixteen generation failures
leave 94 probes unexecuted; four grammars generate and compile, executing 34
probes: seven accepted models and 27 rejections, including 15 rejected partial
models. Class-fragment-first reports a validator error, generates and compiles,
then lacks resource and named parser entry methods. Terminal/scalar libraries
can pass the contextual first-rule check while the default standalone generator
fails. The empty middle source in one three-level graph has a syntax error.
An untyped fragment resolves to Ecore EString; it does not prove a null resolved
classifier. These stages and limitations remain distinct.

The four reused inheritance graphs retain their original source identities,
old complete observations and lineage receipts. Two remain upstream generation
failures. The separate identity provider has EClass EString and EEnum Choice;
Ecore EString remains a distinct EDataType. No provider or graph was repaired.

Run pure integrity checks without Java:

```
python oracle-producer/verify.py
```

To regenerate and independently replay all twenty grammars and the metadata
addendum into a new directory outside the corpus:

```
python oracle-producer/verify.py --replay NEW_OUTPUT_ROOT --jdk JDK21_PATH --maven-cache MAVEN_REPOSITORY
```

The optional route requires JDK 21.0.6+7, the sixty exact jars and pinned Xtext
source archive in `inputs/stack.json`. It generates the actual EMF provider from
the authored Ecore through GenModel and uses StandardLanguage/MWE2, generated
setup, lexer, resource and named IParser consumers. It compares complete fresh
observations with both frozen runs. The sole comparison exception is the exact
absolute owned `provider-<EcoreSHA12>/provider.genmodel` field, after checking its
root and digest. Tokens, UTF16 text/spans, errors, partial models, leaves and
all failure stages are not normalized.

The original producers and helpers remain under `oracle-producer/original/`.
Portable producer changes are enumerated exact path-discovery substitutions.
This oracle records upstream outcomes; it awards no native support or gate.
