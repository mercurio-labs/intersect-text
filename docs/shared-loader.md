# Shared frozen source loading

The user approved the shared-loader/cache boundary correction on 2026-11-18.
`intersect-load` is an unpublished, language-neutral Text-owned package below
both headless language runtimes and graphical hosts. Text itself does not depend
on it. It consumes Text/Model, never Views, KerML, an application, clocks or I/O.

The loader orchestrates parsing, unique linking, implicit/default completion,
structural validation and language-rule checks. Hosts supply the parser, prepared
Rcore scopes, finite budgets and an optional phase observer. Model publication
and graphical candidate transactions remain responsibilities of the consuming
host. This extraction adds no language-specific semantics or execution engine.

`FrozenLoad`, `LoadDiagnostic`, `LoadPhase`, `load_frozen`, `check`,
`violation_code` and `violation_object` retain the previous shared host signatures
and behavior. The initial implementation bytes were copied unchanged from Views
a4816a6, `crates/intersect-view-host/src/load.rs`, SHA-256
8990593de89562daa09839bd56407cde0cafad561c355c7cd0caa94a5c29094f.
The only appended production declaration is the new crate's `SOURCE_IDENTITY`.
`crates/intersect-load/PROVENANCE.json` preserves the earlier port provenance.
Views can provide compatibility reexports; headless KerML imports this crate
directly and no longer requires the view host for loading.

The loader opts into Model's checked scoped compiler identity. Its actual
normal/build dependency context and source bytes contribute to language-program
compatibility. Builds use `python -B tools/compiler_build.py -- <cargo command>`;
this repository's thin launcher delegates to the single implementation in the
immutable selected Model source. `--report -- test ...` retains the shared test
report runner. A missing or stale context refuses instead of weakening pack
compatibility. See Model's docs/scoped-compiler-identity.md for the bounded
native/WASM profile, features, target checks and unsupported custom build flags.

This isolated branch retains the recorded Text 4247679 runtime source baseline
and updates the paired Model source/locks for the approved host-only helper.
Newer develop component-session changes remain preserved on their own branches;
they are not incorporated into this correction. Original worktrees, M13 artifacts
and unfinished M14 changes remain untouched.

Qualification is recorded separately. Real catalog-language fixtures cover
multi-resource construction/link navigation, parse/link/structural errors,
language-rule error and warning diagnostics, and exact phase callbacks. The same
fixture is compiled in a separate executable against the immutable old host to
compare complete outputs without retaining a second production implementation.
Full language packs and consumers still require their own acceptance evidence.
