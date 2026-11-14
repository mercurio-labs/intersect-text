# Executable semantic binding fixtures

`model.rcore` declares a Meter and its `doubled` operation. `behavior.rcore`
supplies verified references. `expected.json` independently specifies answers;
comparing only two implementations is not the oracle.

- `native.rs` is handwritten Rust, called by both manual registration and
  generated adapters. Its local helper call is not nested registry dispatch.
- `behavior.rhai` receives exact integers or admitted read-only model handles.
- `contract.rs` rebuilds exact contracts for native consumers and build scripts.
  Generated adapters live in Cargo's `OUT_DIR` and never overwrite handwritten code.

`external-project.rcore` declares bodyless external functions in a combined Rcore
project; `external.rhai` supplies their handwritten implementations. Open the Rcore
file in the Toolkit, choose File → Create Stub, and fill in the generated bodies.

Run `cargo run -p intersect-semantic-bindings-tests` for the generated native
example, and its tests with `--features rhai` for Rhai/mixed packages, including
external contracts without reference bodies. See
[instructions and limits](../../../docs/semantic-bindings.md).
