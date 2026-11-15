# Explicit performance benchmark hosts

These tools retain the accepted native benchmark host and optional pooled host
allocator. They require explicit source roots and an external fixture/oracle
directory; ordinary library builds and tests have no such dependency.

The allocator's focused memory-contract tests need no runtime or Java dependency:

```powershell
python -B tools/test_report.py run -- --manifest-path benchmarks/java-parity/allocator-tests/Cargo.toml --locked
```

`native.rs` verifies the graph's object/field/reference checksum, records cold and
warm loads, and supports Text and XMI modes. `run_pair.py` builds an explicitly
selected Model/Text pair and compares it with a separately supplied Java host and
scope overlay. Use `--help` for its required source and evidence arguments.

`host_pool.rs` and `pooled_host.py` add a host allocator and recycle freed chunks
between graph loads. They do not change library algorithms or semantic contracts.
For an explicit local host build, from this repository:

```powershell
python -B benchmarks/java-parity/pooled_host.py build --model-root ../intersect-model --text-root . --project target/java-parity/pooled-host --lock <benchmark-consumer-Cargo.lock>
```

Supply an existing compatible benchmark consumer lockfile. Historical fixtures,
their runner, Java overlays, raw reports, and original locks remain under the
workspace archive's `previous-workspace/performance/sysml-scale` and pre-dev
`benchmarks/java-parity` directories. Historical report source identities and
qualification limits remain unchanged; they are not current-revision results.

The paired runner preserves checksums, controls, source provenance, and serial
alternating process order. Broad comparisons and million-object loads remain
milestone work. The original adapter's linear-lookup Xtext measurements are not
valid parity targets; use the corrected and contract-audited Java overlay.
