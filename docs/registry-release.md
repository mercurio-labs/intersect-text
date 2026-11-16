# Registry release procedure

The public evaluation alpha is `0.1.0-alpha.1`. Publish only
`intersect-build-support`, `intersect-model`, and `intersect-text`, in that order.
The other workspace crates remain unpublished. The archived pre-dev repositories
remain private; publish the active Model and Text repositories.

## Standalone package preparation

Prepare a clean, committed release source. From each repository run:

```sh
python -B tools/stage_text_packages.py --registry --output target/registry-release
```

Use a new output directory for each attempt. Model stages its runtime and support
crate; Text also stages the exact Model revision selected by all its maintained
manifests and locks. No implicit sibling checkout is used.

Staging resolves workspace metadata, gathers literal embedded inputs, relocates
their paths, and preserves original fixture bytes and license notices. Registry
packages omit unrelated oracle corpora; full source bundles retain those evidence
units. This does not change repository test cases or their expected outcomes. It records
every original/staged hash and relocation. The package's SOURCE_PROVENANCE.json
identifies its repository and commit. THIRD_PARTY_NOTICES.md and the associated
JSON index identify retained third-party inputs and exact notice hashes.

Registry staging explicitly opts these packages into crates.io and uses exact
internal prerelease requirements. Local patches are used only for prepublication
checks. Cargo removes patches and workspace tables from the uploaded manifest;
the published dependencies must resolve from crates.io. Ordinary source-bundle
staging, without --registry, retains publish=false.

## Verification and publication

Check packaging controls and notices, applicable formatting/lint/native/WASM
checks, Rust 1.88, documentation, and an independent consumer of the extracted
packages. Preserve the resulting commands, package hashes, and source receipts.
Check archive contents and size before uploading. Earlier runtime validation is
supporting evidence with its own source identities, not a replacement for
standalone package verification.

From Model, publish the support crate first:

```sh
cargo publish --manifest-path target/registry-release/packages/intersect-build-support/Cargo.toml --dry-run
cargo publish --manifest-path target/registry-release/packages/intersect-build-support/Cargo.toml
```

After the dependency is available in the registry, repeat the dry-run and publish
for target/registry-release/packages/intersect-model/Cargo.toml. From Text, repeat
for target/registry-release/packages/intersect-text/Cargo.toml after Model is
available. Do not use --no-verify for actual publication.

Verify a fresh consumer using exact registry versions, without Git dependencies
or local patches. Tag the source commits as v0.1.0-alpha.1 and retain release
notes and the exact .crate hashes. Verify GitHub public visibility independently
of authenticated access. Never include registry credentials in artifacts or logs.

## Support boundary

This is an evaluation alpha. APIs may change between alphas, saved formats remain
explicitly versioned, and cross-alpha migration is not promised. Retain authored
source and data so derived caches can be rebuilt. Generic Model/Text release
checks do not qualify arbitrary Xtext/Java customizations or KerML/SysML semantics.
