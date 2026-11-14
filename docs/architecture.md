# Architecture

Intersect Text owns: Grammar compilation, inference, text editing and interpretation over Intersect Model; Library, Web and combined integration harnesses. The browser Toolkit lives in intersect-lang-examples.

The dependency direction is host/application → language package → Model → definitions.
Model owns structural services, native resource storage and execution. Text
owns grammar compilation, inference, scoping adapters and text interpretation.
Language packages own domain algorithms, standard libraries and delegates.
Unsupported constructs remain explicit. Importing data does not qualify native
semantic support; imported, consumed and semantically verified are separate.

Model-only acceptance, Rust bindings, semantic bindings and shared Model test
support live in Model. Text acceptance, model/text parity, resource-lifecycle
integration, Library and Web live in Text. Toolkit and its browser tests live in intersect-lang-examples. Production libraries do not
depend on harness support. The shared harness helpers are implemented once in
Model and consumed at the same Model Git revision as the runtime library.

Clocks, I/O, scheduling and host resources use explicit adapters. Libraries
remain compatible with wasm32-unknown-unknown; Java is an independent oracle,
never a production requirement. Generated and handwritten boundaries are marked.

See [repository development](repository-development.md) and the current
[release plan](release-plan.md) for source pairing and qualification.

## Consumer boundaries

| Component | Responsibility |
| --- | --- |
| `intersect-text` | Source-authoritative grammar compilation, inference, analysis, linking and text/model correspondence |
| `intersect-library` | Independent example language and consumer of shared model/text services |
| `intersect-web` | WASM request/response host and explicit session contracts |
| `intersect-lang-examples/toolkit/` | Browser UI, workers, source drafts, transport and publication |
| External Model dependency | Shared structure, storage, Rcore execution and checked mutations |

Keep original source bundles and selected profiles authoritative. Generated
definitions and prepared output are derived artifacts; loading them must retain
the appropriate identity and replay checks. JavaScript does not implement a
second grammar or semantic engine.

The [language guide](language-guide.md) explains ordinary/inferred/profile paths.
The [editing guide](text-editing-guide.md) distinguishes canonical generation,
general preservation and bounded helpers. Hosts own cancellation, memory
admission, editor/disk writes and final revision checks. Native library, Node
WASM and browser workflows need separate qualification evidence.
