# Full generated literal Until with a local suffix

This separate verification-only companion freezes the exact authored rule
`terminal VALUE returns ecore::EString: '@' ->'ab' 'c';` and seven inputs over
the unchanged imported Ecore schema. It does not extend or change the frozen
eighteen-graph `full-generated-until` corpus or award native support or a gate.

Both original full language generation and independent fresh regeneration/replay
completed. Each run executed seven separate JVM requests through the generated
lexer, actual Xtext resource loader, and named `Model` parser. Complete raw
observations match after validation of the sole owned GenModel path exception.
There are four accepted resources, three rejected resources, and two rejected
partial models per run; partial models are never counted as valid resources.

The observed program accepts `@abc`, `@abxabc`, `@xabc`, and `@ababc` as whole
values. An early `ab` whose following local `c` is missing does not end these
successful tokens. `@abx` is one invalid token with an EOF diagnostic.
`@abcc` and `@abcabc` yield a first `@abc` token followed by invalid input, with
a rejected partial model whose name is `@abc`. These are observations of this
exact program and input set, not a general Until policy theorem.

`inputs/` retains exact grammar bytes, source identity, original Ecore bytes,
and the frozen seven requests. `observations.json` retains all tokens through
EOF, invalid channels, complete resource and parser diagnostics/models, leaf
spans, and UTF-16 values. `positive-resources.json` is a checked view of only
the four accepted resources. `emitted/` contains the 37 actual newly generated
source files, including grammar, lexer/parser Java, generated setup/provider,
workflow and GenModel bytes. `recorded/` preserves raw record/replay observations,
commands, toolchain pins, artifact manifests, logs and original provenance.

The first ignored-location attempt copied the portable producer byte for byte.
Its fixtures-root output guard rejected that relocated path before Java or
output creation. The failure, a separate exact failure reproduction, and the
separately named output-location-only adapter remain preserved. The portable
`produce.py` uses the original producer and its original fixtures-root guard;
the semantic generation/runtime code is unchanged.

Run the byte, stage, and denominator checks without Java:

```powershell
python -B fixtures/xtext/full-generated-until-literal-suffix/verify.py
python -B -m unittest tools.test_xtext_until_literal_suffix_evidence
```

Optional fresh replay uses explicit JDK21.0.6+7, sixty checksum-pinned Maven jars,
and the pinned source archive. The producer generates the EMF GenModel/provider
from the original Ecore, uses StandardLanguage/MWE2 for the full grammar, compiles
fresh Java, loads the generated setup, and executes all three runtime routes:

```powershell
python -B fixtures/xtext/full-generated-until-literal-suffix/verify.py --replay D:/new-evidence/literal-until --jdk D:/dev/jdks/jdk21 --maven-cache C:/Users/YourName/.m2/repository
```

Output must be new and outside `fixtures/`. Only the exact validated absolute
owned `provider-<original-Ecore-SHA12>/provider.genmodel` output path is replaced
for observation comparison. No source, token, error, partial model, leaf or UTF-16
span is normalized. Original logs are independently pinned; timing/log equality
is not asserted. Three native-owned files are excluded explicitly from the oracle
manifest and have separate consumer evidence. Java is an optional verification
tool, never a production dependency.
