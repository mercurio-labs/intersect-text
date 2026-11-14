# Full generated lexical evidence

This additive fixture preserves seven exact imported-Ecore/Xtext source graphs and
80 complete requests to pinned Xtext 2.38.0 generated languages. A separately
rebuilt replay matched every semantic observation. The verification path uses the
actual EMF GenModel/provider, StandardLanguage/XtextGenerator/MWE2, generated Java
lexer/parser, StandaloneSetup, XtextResourceSet and named IParser entry. Java is
an optional independent oracle; native production never executes it.

`observations.json` is the complete original record, including raw UTF-16 token
strings, invalid tokens, diagnostics, valid models, rejected partial models and
node leaves. It has 27 valid resources, 53 rejections and 29 rejected partial
models. The lexer syntax-error counter is not an acceptance test: Xtext reports
invalid tokens through channel 99 and resource/parser diagnostics even when that
counter remains zero. No rejected partial model is relabeled a success.
`positive-resources.json` is only a mechanically checked scalar-safe view of the
27 actual successful resources for native consumers; the full record is retained.

The original `!'ab'` and `!'🌍'` graphs really generate: their lexer consumes any
single UTF-16 unit. The former constructs models for a/b/c; two-unit inputs
produce rejected partial models. Astral input splits into isolated-surrogate
tokens and a rejected partial model. Until overlap `abc|b` on @abc yields @ab
and an invalid suffix; `ab|abc` yields a valid @abc. These are exact full-language
observations, not a universal shortest/longest delimiter algorithm.

`inputs/` retains all original source bytes and jar pins. `emitted/` keeps raw
full generated grammars/lexer/parser sources and verification workflow/GenModel;
absolute output paths and generator timestamp headers are not rewritten.
`native-cases.json` supplies the existing canonical Rcore migration-fixture adapter
with exact Xtext source identities. `before-v58.json` preserves full requests and
results from immutable release f7a8cc16, including all six unsupported graphs and
116 V1–V58 literal rejection requests. No old fixture or receipt was changed.

Run portable integrity without Java:

```sh
python -B fixtures/xtext/full-generated-lexical/verify.py
python -B -m unittest tools.test_xtext_full_generated_lexical_evidence
```

Optional fresh complete oracle replay, into a new directory outside fixtures:

```sh
python -B fixtures/xtext/full-generated-lexical/verify.py --replay target/full-generated-lexical-replay --jdk /path/to/jdk --maven-cache /path/to/.m2/repository
```

The producer takes explicit JDK/cache/output paths. It regenerates the verification
provider directly from the unchanged original EPackage; it never synthesizes a
simpler metamodel or repairs a rejected grammar. Complete replay compares every
observation, replacing only the explicit provider.genmodel output-location field
with a placeholder. Full X05/X06 qualification remains open.
