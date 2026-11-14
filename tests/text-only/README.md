# Isolated Xtext public consumer

This standalone native/WASM crate compiles original Rcore/Xtext definitions and
uses the same model and language services as application consumers. Its explicit
versioned dependencies are model, Xtext and JSON support.

The two shared controls exercise ordinary compilation, cross-resource linking
and validation, canonical generation, exact preserving edits, stable identities,
stale proposal refusal, undo/redo, native inference, source-authoritative replay,
compiler/provider cache keys, explicit rejection and recovery. Native tests and
the exported WASM entry points call the same Rust functions. The WASM runner
checks zero imports and executes both controls in two fresh instances.

From the repository root:

```sh
cargo clippy --manifest-path tests/text-only/Cargo.toml --target-dir target/xtext-only --all-targets --locked --offline -- -D warnings
python -B tools/test_report.py run -- --manifest-path tests/text-only/Cargo.toml --target-dir target/xtext-only --profile test-fast --lib --locked --offline
cargo build --manifest-path tests/text-only/Cargo.toml --target-dir target/xtext-only --lib --target wasm32-unknown-unknown --locked --offline
node tools/test_text_boundary_wasm.mjs
```

For portable distribution acceptance, stage both isolated consumers and their
packages with vendored locked registry inputs:

```sh
python -B tools/stage_text_packages.py --output target/xtext-stage-001 --vendor
python -B target/xtext-stage-001/verification/verify_text_stage.py --stage target/xtext-stage-001 --output target/xtext-stage-001/evidence/local
```

Use a new output directory for each attempt. The local command records whether
the original checkout remains available. On a worker without that checkout, add
`--require-original-unavailable`; presence of the original sources fails the
check. The isolation workflow downloads this payload in a separate job without
checking out the repository. Runtime source retention, distribution integrity
and wider language/downstream qualification remain separate evidence.
