# Repository development and verification

Model owns modeling, definitions, Rcore execution, code generation and host adapters.
Text owns grammar services, Library, Web and combined integration harnesses. Browser
application sources and browser tests live in `intersect-lang-examples/toolkit`.

Use `develop` explicitly as the base for new work. Development uses Rust 1.99.1,
Python 3.11+ and Node 22.15.1 for WASM execution. The minimum Rust version is 1.88;
the target is `wasm32-unknown-unknown`. Install the rustfmt and Clippy components.

## Source acquisition and dependency pins

Normal repository builds use committed manifests and locks. Text obtains Model,
build support and shared harness packages from one canonical Git URL and exact
revision. Different URLs are different Cargo sources even if GitHub redirects them.
Run `cargo fetch --locked` in the checkout being built, then use locked builds.
Reading a private dependency requires credentials with repository access.

This fresh-history repository contains the reviewed current sources. Earlier
development history is preserved separately in private pre-dev storage. During
local preparation, the new pins become fetchable from canonical GitHub URLs only
after cutover. Cache-backed checks must be followed by fresh-fetch verification.

Establish Model's new commit first, update all maintained Text dependency kinds and
standalone consumer manifests together, and regenerate their locks. Then update
language/application consumers. Before release, `cargo metadata --locked` and
`cargo tree -d` must show one intended identity for each Intersect package.

## Coordinated local changes

An explicit temporary Cargo configuration can override the Model packages used by
a Text consumer. Include runtime, build support and test support as applicable:

```toml
[patch."https://github.com/mercurio-labs/intersect-model.git"]
intersect-model = { path = "C:/work/intersect-model/crates/intersect-model" }
intersect-build-support = { path = "C:/work/intersect-model/crates/intersect-build-support" }
intersect-model-test-support = { path = "C:/work/intersect-model/tests/support" }
intersect-rust-bindings-tests = { path = "C:/work/intersect-model/tests/rust-bindings" }
```

Pass it explicitly with `cargo --config path/to/local-model.toml ...`. Standalone
consumer workspaces also need the configuration. Never commit local paths as default
dependency declarations, and remove overrides for canonical Git-pair qualification.

## Test tiers and commands

After a change, check the changed crates without `milestone-tests`. Substitute the
owning crate name for `<crate>`:

```sh
python -B tools/format_workspace.py --check
cargo clippy -p <crate> --all-targets --locked -- -D warnings
python -B tools/test_report.py run -- --profile test-fast -p <crate> --lib --tests --locked
cargo check -p <crate> --lib --target wasm32-unknown-unknown --locked
python -B tools/check_text_release_contract.py
python -B tools/check_text_separation.py
```

Model additionally has `tools/check_model_boundary.py`. Independent consumers are
under `tests/model-only` or `tests/text-only`; their native tests, doctests and Node
WASM runners verify the public interface without application adapters. API surface
and declaration baselines live in `tests/api-surface`, with tools in `tools/api_*`.

Quick checks must finish in under ten minutes including the build. Whole-library,
binding regeneration, language equivalence, full workspace and oracle comparisons
are milestone-only. Preserve test names, assertions, expected data and ignore states.
Use `tools/test_report.py` for Rust test suites; the optional dashboard is localhost
port 8765. A full portable-evidence lane discovers `tools/test_*.py` and must not be
silently dropped when old files are omitted.

The durable-document evidence control also compares browser fixtures now owned by
Examples. Set `INTERSECT_EXAMPLES_ROOT` explicitly to a qualified
`intersect-lang-examples` checkout when running that control or the full portable
evidence lane. It fails with an actionable message when the path is missing;
it does not silently search sibling directories, skip comparisons or change goldens.
Record the selected Examples revision and fixture hashes with the result.

Use Rust 1.88 for separate native and WASM minimum-version checks. Model's optional
features are `xmi`, `codegen`, `rhai` and native-only `sqlite`; combinations need
qualification on the final pair. Rhai's documented historical mismatch has the
narrow alpha exception described in the [release plan](release-plan.md).

## Ownership and retained evidence

Shared test infrastructure belongs to Model; production libraries do not depend
on harness crates. Text's pinned infrastructure snapshots and immutable Model
evidence are described by its shared-tooling/shared-model-inputs manifests.
Duplicated fixtures are not a second semantic implementation.

Fixture histories, embedded upstream sources, raw observations and golden data keep
their original bytes. Their internal historical paths and links may refer to pre-dev
material. They are evidence, not current setup instructions or executable authority.
Do not repair an evidence hash to conceal omitted bytes. Migration utilities remain
for review; run only commands that are documented for the current tree.

## Source-bundle notices

Intersect-authored code is MIT. Imported sources, generated tables and retained
captures keep their separate terms. `docs/repository-notices.json` selects reviewed
content families and exact notice hashes. Against a committed tree, run:

```sh
python -B tools/check_repository_notices.py --out target/repository-notices.json
```

The checker verifies the selected Git tree, not uncommitted files. Before a
commit, pass the explicit file manifest to its shared `check` function.
Never remove attribution still needed by retained generated sources or traces.

## Release artifacts and recovery

The package staging/verification tools and `tools/release-isolation.Dockerfile`
remain maintained. Use each script's `--help` for its exact supported invocation.
Qualify fresh extracted source/package artifacts with no original checkout access;
run actual WASM consumers and the focused Examples browser build on the same pair.

Historical release receipts and saved benchmark runs remain in private pre-dev.
New receipts must identify exact final commits, toolchains, features and exclusions.
The [artifact inventory](alpha-artifacts.json) and [release plan](release-plan.md)
are the current work list. Repository renaming, new root commits, remote pushes and
publication are separate operations requiring release-owner authorization.
