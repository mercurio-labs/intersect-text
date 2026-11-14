# Library performance suite

This optional benchmark generates identical UTF-8 text for the production
Intersect evaluator and an independently generated Xtext 2.38.0 / EMF parser.
Defaults are 100 (control), 1,000, 10,000, 100,000 and 1,000,000 elements,
with three fresh-process trials per engine and size. One root Library counts as
an element; the other N-1 elements are Members named m1 through m(N-1).

Historical result files remain in private pre-dev. Measure new baselines with
explicit source/toolchain identities. Runner controls:
`python -m unittest discover -s benchmarks/library -p test_runner.py`.

## Run

Requirements: Rust, Python 3.11+, JDK 17 or later, Maven 3.9+, Windows or Linux.
Set JAVA_HOME and put Java and Maven on PATH. From the repository root:

```sh
cargo build --release -p intersect-text-acceptance --example library_benchmark --locked
mvn -B -f benchmarks/library/java/pom.xml package dependency:build-classpath -Dmdep.outputFile=target/classpath.txt
python benchmarks/library/run.py
```

Use `--java /path/to/java`, `--sizes 1000 10000 100000 1000000`,
`--repeats 5`, `--timeout 180`, `--java-heap 4g`, or `--output path` as needed.
The Java heap starts at 64 MiB; its default cap is 4 GiB. Each timeout kills
the worker and is recorded separately. Keep other heavy workloads idle.
Generated corpora, logs, raw JSON and REPORT.md go to `target/library-benchmark`.
Large inputs and generated Java are ignored by Git. The runner regenerates
corpora and overwrites results in its chosen output directory.

Use `--rust-mode stored` to retain the canonical indexed model without the public
map-based export. The default `--rust-mode export` retains the compatibility
model. The standalone Rust example accepts `--stored` after its path and count.
Both execute the same shared evaluation; the selected retained representation is
recorded in the result. The [canonical model release controls](../../docs/release-plan.md)
verify both paths through 1M elements and record peak/final RSS. Those are single
smoke observations, not a repeated performance or equivalent Java comparison.

## Interpretation

For native before/after memory work, save the baseline release executable before
changing source and compare it with the candidate:

```sh
python benchmarks/library/compare_native.py --baseline path/to/baseline --optimized target/release/examples/library_benchmark
```

On Windows include `.exe`. The default is three paired trials at 1k, 10k, 100k
and 1M elements, alternating order. Inputs, binary/source hashes, raw observations
and a median table are saved under `target/memory-comparison`. A failed or timed-out
trial makes the command fail; unsuccessful runs are excluded from successful medians.
Pass `--rust-mode stored` to compare retained canonical stores on both binaries;
both binaries must support the standalone example's `--stored` option.
To count allocations, build the example separately with
`--features allocation-profile --target-dir target/profile-current` and build the
baseline with the same example instrumentation. Pass both binaries with
`--profile-baseline` and `--profile-optimized`. These additional runs are separate
from timing trials. The host-only allocator records requested live bytes, peak
requested live bytes, allocation/reallocation calls and cumulative requested bytes
over evaluation, excluding verification. It does not include allocator bookkeeping,
fragmentation, or transient internal realloc copies. RSS is measured independently.
Neither allocator instrumentation nor clocks are added to library semantics.

**Native evaluation now has no fixed model-size or execution-step ceilings.**
`evaluate` uses `EvaluationLimits::native()`; hosts can set optional resource,
input-byte, element, token and step budgets through `evaluate_with_limits`.
The browser adapter explicitly uses `EvaluationLimits::workbench()` and retains
its original protective policy. Parser nesting remains bounded at 64 by default.
The runner timeout still applies. Historical reports are not measurements of
this staged tree.
Rejection latency is recorded in raw JSON but is never shown as successful parse
time or used to compute a speedup. The 100-element control must succeed before
interpreting scale observations.

Intersect timing covers `evaluate`: lexing, parsing, model construction, name
indexing, linking, opposite reconciliation, structural validation and semantics.
Xtext timing covers `XtextResource.load`: parsing, EMF construction and the
Xtext node model. It does not run a separate validator or Rcore behavior interpreter.
Startup, definition compilation/setup, input file reads, verification, and parent
process orchestration are outside both timers. These are first parses in fresh
processes, not warmed JVM throughput measurements. Do not infer equivalent-stage
speedups from these numbers.

Both workers verify the exact element count, class and name of every element.
The Java grammar is a benchmark-only Library/Member projection with an inferred
EMF metamodel; Intersect consumes the complete checked-in Library definitions.
The projection accepts the same generated text, but does not establish full Ecore
equivalence, inheritance, opposite, cross-resource, Loan, BookCopy, or Rcore behavior
performance. This small workload is an initial capacity baseline.

Memory is the OS process resident working set at READY (input and definitions
resident) and MEASURED (operation and verification complete, result still alive).
Peak RSS is the lifetime high-water mark through MEASURED. Windows uses
GetProcessMemoryInfo; Linux uses /proc. Neither engine forces GC. These numbers
include runtime, text, model, node/source information and temporary allocations;
they are **not deep model heap sizes**. Rejected rows represent a rejected process,
not the memory required for a successfully constructed large model. Allocator and
JVM GC policies make subtracting baseline RSS an unreliable model-size estimate.

`results.json` records each trial, diagnostics, corpus bytes and SHA-256, executable
hash, input/source hashes, Java/Rust versions, platform, commands and dependency
JAR hashes. The Xtext development BOM pins its associated EMF dependency versions.
Keep raw evidence alongside any published summary. This benchmark does not qualify
general Ecore/Xtext compatibility or make a framework-wide performance claim.

Java is used only by this optional comparison. No production or normal Rust build
depends on it. Parser and EMF implementations under `src-gen` are generated;
`src/bench/Main.java` and the Rust example are handwritten measurement hosts.

Generator configuration follows the upstream [Xtext Maven workflow](https://eclipse.dev/Xtext/documentation/350_continuous_integration.html)
and [Xtext configuration documentation](https://eclipse.dev/Xtext/documentation/302_configuration.html).
