# Independent Xtext syntax observations

These 19 authored definitions exercise the pinned Xtext 2.38 parser directly.
They are not imported KerML definitions. The Java harness records syntax errors,
AST classifier/attribute observations and UTF-8 source offsets. String attributes
use UTF-16 code-unit arrays so isolated surrogate observations remain lossless.
No linking, metamodel inference, validation or generated-parser execution is
claimed. The parser's recovered trees for invalid inputs are evidence only;
Intersect returns an error rather than accepting those trees.

`provenance.json` pins every locally cached classpath JAR and all harness/input/
observation bytes. This parser fixture uses its recorded EMF 2.36 dependency;
it does not change the repository's separate Ecore compatibility target.
Production and ordinary Rust/WASM tests require no Java. The optional replay is:

```
python fixtures/xtext/syntax-ast/oracle.py --jdk JDK_ROOT --artifact-dir OUTPUT_DIR
```

`--record` deliberately regenerates observations and their hashes from the pinned
classpath. Review changes; passing a new snapshot is not automatic qualification.

The current native comparison checks acceptance, declaration counts, decoded
keyword values and exact keyword spans. All 19 cases now agree. Syntax strings
retain UTF-16 units, including isolated surrogates, without replacement characters.
Consumers needing scalar strings must explicitly request a checked conversion.
The previously recorded surrogate difference was closed against the unchanged
snapshot. General node-by-node correspondence and complete syntax qualification
remain open.

The additive `structure.xtext` fixture independently checks grammar, annotation
and named/positional argument spans, including surrounding comments and whitespace.
`structure-provenance.json` binds this evidence to the same Java harness/classpath.
Replay it by adding `--structure` to the command above. The original 19-case
observations are retained unchanged.

The additive 11-case `--coverage` suite records all 29 concrete grammar node
classes and lexical rejection boundaries. Its separate provenance leaves earlier
inputs/observations unchanged. Native counts are compared by class, with syntax
wrapper/condition-chain normalization; this does not establish complete attribute,
reference or execution equivalence. A schema integrity test accounts separately
for the seven common node families that the pinned parser does not instantiate.

`tree_oracle.py` uses the same pinned classpath and syntax inputs to compare full
attributes, ordered containment and converted reference spellings. It deliberately
does not resolve references. Run it with the same `--jdk` and `--artifact-dir`
arguments; `tree-provenance.json` pins this additive evidence. The 14 valid trees
among 31 inputs match native/WASM verification projections. Group wrappers and
condition chains are normalized explicitly in test support, not production code.
