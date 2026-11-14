# Pinned datatype-token conversion oracle

This Intersect-authored Java probe invokes Xtext 2.38.0 DatatypeRuleToken directly.
It establishes a narrow upstream contract before native datatype execution is
implemented. It is optional independent verification, never a production or
normal native/WASM build dependency. No upstream implementation is copied.
Source and binary archives, inspected members and observations have exact SHA-256
identities in provenance.json; the repository's frozen upstream inputs are intact.

Compile with a JDK and the pinned org.eclipse.xtext-2.38.0.jar on the classpath;
write classes into an isolated artifact directory. Run DatatypeTokenOracle with
that directory and the same jar on the classpath. The program asserts all seven
cases, returns nonzero on mismatch, and emits JSON lines. observations.json is
the structured captured result. No network, model files or host resources are
used by the probe. The protected merge operation is exposed by a small subclass;
this does not exercise a generated parser, metamodel conversion or name linking.

The controls distinguish compact spelling, normalized hidden gaps, raw quoted
escapes, nested versus flattened token merging, UTF-16 offsets and an empty
accumulator. For the source A/*gap*/::B., flattening produces A ::B. while merging
the A/*gap*/::B datatype result before the dot produces `A ::B .`. The extra
space is observed upstream behavior, not a proposed normalization policy.

The source EFactoryValueConverter delegates datatype conversion to its EFactory.
That conversion registration was inspected, but this token probe does not execute
it. Mercurio's qualified-name adapter separately interprets segments and quoted
names; those domain-specific semantics are not the default Xtext token contract.

Reproduce with `python fixtures/xtext/datatype-contract/verify.py --jdk <jdk-path>
--artifact-dir <isolated-output-directory>`. The runner checks source and binary
identities against the frozen upstream lock, verifies oracle/observation hashes,
runs seven upstream assertions and compares the captured JSON. An optional
`--maven-cache` selects an existing local Maven cache; it never downloads jars.
