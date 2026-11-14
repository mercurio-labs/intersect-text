# Independent terminal-negation oracle

This additive verification adapter parses the exact original Xtext source graphs,
loads the unchanged terminal-spellings Ecore fixture, resolves grammar and rule
references, records full Xtext validation, and passes actual terminal rules through
Xtext 2.38 `TerminalRuleToLexerBody`. The returned bodies and upstream rule names
are placed in an explicitly authored lexer scaffold, then compiled by the pinned
ANTLR generator 3.2.1 and executed with ANTLR runtime 3.2. Java is verification only.
This is a translator/lexer oracle, not a generated Xtext parser/model application.

`cases.json` has 22 original source graphs. `observations.json` preserves syntax,
post-link resource diagnostics and exact rule-call ownership, full validation,
translated bodies, generator exit/diagnostics, Java compilation, and lexer results
as distinct fields. Translation/generation are attempted after validation errors
for observation only. Such execution never establishes source-grammar acceptance.
Token results retain numeric and named identity, text, channel, UTF-16 start/stop,
all tokens through EOF, lexical errors and complete-single-VALUE acceptance.

The recorded stages are: 22 syntax-clean; 21 link-clean; 19 without validation
errors; 21 translated; 15 successful ANTLR generations; 67 runtime observations.
EOF is rejected by validation and generation; cycles by validation and generation;
unknown calls by linking/validation and translation. Nullable operands, explicit
sequences, wildcard and astral ranges retain their separate generator failures.

Two accepted larger-operand results remain concrete native-equivalence work:
`!'ab'` translates to `~('ab')`, generates successfully and accepts each one-unit
`a`, `b`, or `c`. `!'🌍'` translates to an escaped UTF-16 pair, generates
successfully and splits either globe input into isolated-surrogate VALUE tokens.
These are retained upstream successes, not described as unsupported upstream
syntax or generator failures. The current bounded native one-unit implementation
does not close these remaining obligations or general terminal/Xtext support.

The inherited-program case proves the parent VALUE operand call rebinds to the
child fragment. The explicit-parent case preserves SUPER_LETTER instead. The
committed-path case selects VALUE on `ad`/`ac`, then reports a lexical error and
emits only EOF; it does not fall back to the shorter SHORT rule. `adc` succeeds.

An initial batched runtime probe timed out after 60 seconds. Its inputs, producer
and generated artifacts are preserved in the isolated initial evidence directory.
The final adapter isolates each input and records the ten-second timeout for
repeated-negation input `AbC` explicitly. This observation is inconclusive runtime
nonprogress, not upstream acceptance or rejection. Other observations survive it.

`provenance.json` pins producer/input/output bytes, all 17 dependency jars, and the
existing dependency manifest/metamodel. `emitted-provenance.json` additionally pins
unchanged emitted grammar/Java snapshots for larger operands, partial failed
sequence generation, inherited dispatch and committed failure. A partial Java
file never overrides the generator exit status. Raw generated timestamps/paths
are retained in these snapshots. Replay canonicalizes generated Java text line endings, the timestamp header
and exact grammar path for its supplemental generated-Java digest; no terminal
bodies, diagnostics escapes or runtime token values are normalized. Every isolated
run also retains raw artifacts and an `artifacts-sha256.json` manifest.

Portable integrity and stage checks require no Java:

```sh
python -B -m unittest tools.test_xtext_terminal_negation_oracle
```

Replay into a new, nonexistent output directory:

```sh
python -B tools/xtext_terminal_negation_oracle.py --jdk /path/to/jdk --artifact-dir /new/isolated/output --maven-cache /path/to/.m2/repository
```

The recorded JDK is Temurin 21.0.6+7. Existing terminal-decisions, syntax-ast and
terminal-overrides sources, observations and pins are unchanged. These independent
observations are one evidence layer; native/WASM/host matching remains separately
owned and no full support gate is awarded by this adapter.

The first immutable replay retained exactly one unstable diagnostic: cyclic-fragment
ANTLR generation throws StackOverflowError with JVM-dependent stack cutoff/top
frames. The comparator requires the same exception class and both recursive
testSetElement/testSetRule frames, while preserving both complete raw traces. All
other fields compare exactly; original recorded observations remain unchanged.

## Separate nested-domain companion

The two additional exact source graphs are retained in `nested-cases.json`, with
separate `nested-observations.json` and `nested-provenance.json`. Neither changes
the original 22 graphs or observations. Direct `!(!'a')` and fragment-mediated
negation of a negated unit domain both pass syntax, linking, full validation,
translation and ANTLR generation. Their six runtime observations accept only `a`
with zero errors; `b` and `c` each produce one lexical error and only EOF. Raw
emitted sources are separately pinned under `emitted/`. A new isolated replay
matches the complete stage/body/runtime observations exactly:

```sh
python -B tools/xtext_terminal_negation_nested_oracle.py --jdk /path/to/jdk --artifact-dir /new/nested/output --maven-cache /path/to/.m2/repository
```

Native/WASM adoption of these graphs is a separate companion evidence layer.
