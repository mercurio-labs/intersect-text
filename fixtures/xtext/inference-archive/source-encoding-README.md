# Independent Xtext source-encoding observations

Three exact grammar sources use the unmodified pinned Xtext 2.38 FragmentOracle,
MetamodelOracle and CompositionOracle helpers. Plain source and a Unicode-comment
suffix pass complete inference validation; a leading U+FEFF BOM produces a syntax
error at UTF-16 offset zero. The complete three source-loaded observations are
frozen, including rejected partial output and diagnostics. Their expected outcomes
were captured before changing any native grammar/lexer algorithm; this archive
increment makes no native grammar/lexer changes.

The older fragment verifier intentionally accepts ASCII-authored fixtures only.
This separate read-only harness reuses its pinned Java helpers/JAR identities and
forces UTF-8 Java stdout/stderr so the unexpected BOM diagnostic is retained
without a platform encoding replacement. It does not rewrite existing fixtures.

Native archive controls compare acceptance/rejection and preserve exact accepted
UTF-8 source inputs. This does not qualify full native/upstream diagnostic-span
correspondence. The ZIP extractor separately preserves BOM bytes as payload data;
a rejected grammar source must remain an explicit rejection.

Run verify_source_encoding.py with --jdk, --maven-cache and a fresh --artifact-dir.
It never rewrites the frozen expectations. Java is an optional independent oracle.
