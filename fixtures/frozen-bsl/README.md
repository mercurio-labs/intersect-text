# Frozen BSL gate inputs

The Base / ScalarValues / Links graph that the K01 native pipeline admitted,
frozen as the input of `crates/intersect-kerml/examples/frozen_bsl.rs`
(`frozen_bsl fixtures/frozen-bsl --rcore --fast`, see
[docs/frozen-engine.md](../../docs/frozen-engine.md)):

| File | Contents | SHA-256 |
|---|---|---|
| `graph.json` | the admitted linked graph (269 objects, 4 resources) | `d6c2332afa70564aa287c92a916520fbb4dbe8bcdcc10f34dd6a3a28350cb766` |
| `schema.json` | the prepared SysML schema | `78ffa80fb64974b21122c84df0801a4e0b43e5ec3d0f088dbc5512aa72cd3fea` |
| `phases.jsonl` | the run's phase events, among them the 57 `candidate_context` scope requests | `5d5f3e70348035556f5aa1aa7547657252158e201c9a6933a63a49cc06f1295b` |

Produced 2026-11-06 on `claude/rcore-frozen-engine` at `03f08c67` (release
build) by

~~~sh
native_pilot_bench kernel-links fixtures/native-pilot/v1 OUT
~~~

and copied from `OUT` unchanged. `graph.json` and `schema.json` were
byte-identical to an earlier run's; `phases.jsonl` differs between runs only
in its timings.

These files cannot be regenerated: `native_pilot_bench` and the K01 native
pipeline it ran were deleted in migration step 3 of
[docs/old-path-inventory.md](../../docs/old-path-inventory.md). They are an
oracle of what that pipeline admitted, not of the Pilot; the KerML Pilot
comparison is `library_probe` with `tools/pilot-oracle/compare.py`.
