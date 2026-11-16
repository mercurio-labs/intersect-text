# Intersect Text

**Turn language definitions into parsers, linked models, and model-aware text editing.**

Intersect Text was designed around the same conceptual ideas as
[Eclipse Xtext](https://eclipse.dev/Xtext/): describe a language's grammar and
connect parsing, linking, diagnostics and editing to its model. It implements
supported grammar profiles in Rust and WebAssembly.

It adds a textual language layer to
[Intersect Model](https://github.com/mercurio-labs/intersect-model). Define model
structure in Rcore and syntax with an Xtext grammar, then parse source files,
resolve references, report diagnostics, and work with the resulting models in
native Rust or WebAssembly. Production execution does not require Java.

The goal is one set of language services for command-line tools, editors, and
browser applications, with model edits and text edits sharing the same semantics.

## What it provides

- **Language compilation:** compile supported Xtext grammars over Rcore models,
  with explicit grammar profiles, retained sources, and inference services.
- **Text analysis:** construct models, link references, validate, and retain
  document locations for diagnostics and editing.
- **Model-to-text services:** generate text for representable models and prepare
  source-preserving edits, checking the result by parsing, linking, validation,
  and model comparison.
- **Working consumers:** the Library example and browser Toolkit exercise the
  same Rust services exposed to applications.

Grammar admission, execution, serialization, and preservation have distinct
support boundaries. Arbitrary Xtext languages and Java/Xtend customizations are
not automatically supported; custom semantics need explicit native providers.

## Install

The first registry release is `0.1.0-alpha.1`, for evaluation and feedback:

```toml
[dependencies]
intersect-text = "=0.1.0-alpha.1"
```

Model services are included as a dependency. Add `intersect-model` at the same
exact alpha version if your application also imports its APIs directly.
APIs may change between alpha releases. Native Rust and WebAssembly are supported.

## A small language

This example defines a language that reads `item first;` into an `Item` model:

```rust
use intersect_text::{Source, compiler, evaluate};

fn main() -> Result<(), String> {
    let structure = r#"
        rcore 1;
        package example version "1";
        metamodel "urn:example" prefix example {
            class Item { attr name: String [1,1]; }
        }
    "#;
    let grammar = r#"
        grammar example.Items with org.eclipse.xtext.common.Terminals
        import "urn:example" as example
        Item returns example::Item: 'item' name=ID ';';
    "#;
    let language = compiler::compile_rcore_project(structure, grammar)
        .map_err(|diagnostics| format!("{diagnostics:?}"))?;
    let result = evaluate(&language, &[Source {
        uri: "memory:example.items".into(),
        text: "item first;".into(),
    }]);
    if !result.diagnostics.is_empty() {
        return Err(format!("{:?}", result.diagnostics));
    }
    println!("Parsed {} object(s)", result.model.elements.len());
    Ok(())
}
```

The snippet uses `intersect-text` and prints `Parsed 1 object(s)`. The convenience
compiler selects the bounded `WorkbenchV1` profile; advanced grammars require
explicit profile and source-bundle configuration.

For a runnable example with two resources, linked references, text generation,
and edits that preserve comments and layout:

```sh
cargo run -p intersect-library --example library_text --locked
```

See the [Library guide](crates/intersect-library/README.md) and
[Text API guide](crates/intersect-text/README.md).

## How this relates to Xtext

| Concept | Xtext | Intersect Text |
| --- | --- | --- |
| Language definition | Xtext grammar and EMF model | Supported Xtext grammar profiles over Intersect Model/Rcore |
| Runtime | Generated parser and JVM language infrastructure | Rust compilation and language services, also compiled to WASM |
| Semantic services | Linking, validation and custom language services | Explicit native/Rcore providers with bounded profile support |
| Editing | Eclipse and language-server integration | Model-to-text services, preserving edits and a browser Toolkit |
| Customization | JVM integrations, including Xbase | Explicit portable contracts; Java/Xtend code is not loaded |

The [Xtext project](https://eclipse.dev/Xtext/) provides the conceptual reference.
Intersect Text does not promise arbitrary Xtext language or IDE-feature parity;
see the [language guide](docs/language-guide.md) for admission and execution limits.

## Try the browser Toolkit

Hosting is currently disabled. Download **examples-site.zip** from the
[verified preview build](https://github.com/mercurio-labs/intersect-lang-examples/releases/tag/examples-preview-2026-11-13), unzip it,
then run `python -m http.server 8000` in the extracted directory and open
`http://localhost:8000/`. The site links below are reserved for future hosting.
Future successful Browser examples workflow runs also provide an `examples-site`
artifact; the preview download is built and checked locally.

[Open the interactive example site](https://mercurio-labs.github.io/intersect-lang-examples/) for Petri simulation,
Catalog reference exploration, and the Library simulator's synchronized model
and text editing. Toolkit now lives in
[intersect-lang-examples/toolkit](https://github.com/mercurio-labs/intersect-lang-examples/tree/develop/toolkit),
with its application, pinned runtime build, fixtures and browser tests.
See its README to build and run it independently.

## How the projects fit together

**Applications → language packages → Intersect Text → Intersect Model.**

Model owns structure, storage, and shared Rcore execution. Text owns grammar
compilation, text analysis, and text editing. Language packages own domain rules,
scoping, and standard libraries. The
[KerML/SysML package](https://github.com/mercurio-labs/intersect-lang-kerml)
provides those languages separately.

This repository contains `intersect-text` (the library), `intersect-library`
(an example language), `intersect-web` (the WASM host), while the browser application lives in `intersect-lang-examples/toolkit`. Model is an immutable Git dependency; a sibling
checkout is not required for the normal build.

## Status and documentation

The repositories are preparing their first release after the Model/Text split.
Use the [release plan](docs/release-plan.md) for scope, known gaps, and acceptance
criteria. Unrestricted Xtext compatibility is not claimed. The registry release
set is `intersect-model`, `intersect-text`, and their shared host-only
`intersect-build-support`; see the [package boundary](docs/release-plan.md#package-and-api-boundary).
The [registry release procedure](docs/registry-release.md) explains standalone
packages, retained upstream notices, and release verification.

- [Architecture](docs/architecture.md) and [documentation index](docs/README.md)
- [Language guide](docs/language-guide.md) and [text editing guide](docs/text-editing-guide.md)
- [Repository development and dependency setup](docs/repository-development.md)
- [Compatibility contract](docs/text-compatibility.json)
- [Baseline provenance](docs/baseline-provenance.json)

## Contributing

The minimum Rust version is **1.88** (edition 2024), including the Model dependency.
The development toolchain remains pinned separately below. See
[repository development](docs/repository-development.md) for the minimum-version
and isolated artifact checks.

Use `develop` as the explicit branch base. Install Rust 1.99.1, Python 3.11+,
Node 22.15.1 for WASM execution, and the `wasm32-unknown-unknown` Rust target.
Run `cargo fetch --locked` once; ordinary builds use the committed lockfile.

### Test tiers

After a change, run only changed crates, without `milestone-tests`:

```sh
python -B tools/format_workspace.py --check
cargo clippy -p intersect-text --all-targets --locked -- -D warnings
python -B tools/test_report.py run -- --profile test-fast -p intersect-text --lib --tests --locked
cargo check -p intersect-text --lib --target wasm32-unknown-unknown --locked
```

Quick checks must finish in under ten minutes, build included. Whole-library,
language equivalence, binding rebuilds, Pilot and full workspace checks are
milestone-only. Preserve existing case names, assertions and ignore statuses.
The report dashboard is http://localhost:8765/.

Hosted CI must be qualified on the final baseline; workflow definitions
and skipped platform checks are not passing evidence.
Registry publication is a separate operation.

The public Model dependency requires no private read token. Hosted CI results
remain separate from local release verification.

## License

Intersect-authored code uses [MIT](LICENSE). Imported grammars, fixtures and
Model's generated Unicode data retain their upstream terms. Source bundles
include a third-party notice index; see
[notice coverage](docs/repository-development.md#source-bundle-notices).
