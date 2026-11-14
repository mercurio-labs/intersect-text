# Independent EMF graph oracle

This optional handwritten host loads native XMI into upstream EMF, resolves and
validates it, observes every stored structural feature, clears Intersect extension
metadata, and saves plain EMF XMI. Rust compares both the observation and the saved
graph against independently authored dynamic/text controls. Eight deterministic
seeds and two construction origins give 16 resource-set comparisons.

Use JDK 17+ and the pinned dependencies below (also used by the earlier Library
oracle). Set `EMF_CLASSPATH` to their JAR paths, separated by `;` on Windows or `:`
on Unix. No dependency is installed by this script.

```text
org.eclipse.emf:org.eclipse.emf.common:2.30.0
org.eclipse.emf:org.eclipse.emf.ecore:2.36.0
org.eclipse.emf:org.eclipse.emf.ecore.xmi:2.37.0
```

From the repository root:

```sh
cargo run -p intersect-xmi --example compatibility_corpus --locked -- oracle-export target/compatibility-oracle/input
java -cp "$EMF_CLASSPATH" tools/compatibility/Verify.java target/compatibility-oracle/input target/compatibility-oracle/emf
cargo run -p intersect-xmi --example compatibility_corpus --locked -- oracle-verify target/compatibility-oracle/emf
```

PowerShell uses `$env:EMF_CLASSPATH` in the Java command. Java source-file launching
is supported, or compile `Verify.java` into a separate `target` directory with
`javac` and run `Verify` with that directory added to the classpath.

The 2026-10-06 successful run used Oracle JDK 17.0.12+8-LTS-286. Exact JAR, schema,
oracle source and cached-output hashes are in
`fixtures/compatibility/emf-oracle/provenance.json`. The cache contains the eight
dynamic-origin sets (two XMI resources plus observation JSON each); both origins
were checked live. These are Intersect-authored model data serialized/observed
by EMF, distinct from the unchanged upstream fixtures under `upstream/`.

`cargo test -p intersect-xmi --test compatibility --locked` compares every cached
observation and plain EMF resource set against current dynamic/text construction.
`npm run test:compatibility` in `web` also checks fixture hashes. Normal builds,
tests and production need no Java. This oracle does not verify upstream Xtext
parsing, every Ecore construct, large-model performance or general conformance.
