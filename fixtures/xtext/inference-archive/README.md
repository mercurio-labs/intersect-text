# Ordinary archive compatibility baseline

Frozen before inferred archive integration at a7ea7155575214029d47905157ea55c43cb4c80a.
This is a compatibility observation, not an independent semantic oracle and not
a support qualification. Existing independent model expectations remain separate.

The unchanged compatibility_corpus host produced the ordinary Graph distribution.
The baseline retains its complete manifest and exact UTF-8 payload hashes/lengths.
The existing Toolkit packageArchive produced deterministic ZIPs both without and
with generated Python assets; exact byte counts and SHA-256 hashes are frozen.

Do not regenerate these expectations after implementation. The native and real
ZIP/WASM consumers compare fresh results to these records. Provenance records
the producer commit, source hashes and capture command. Java was not involved.
