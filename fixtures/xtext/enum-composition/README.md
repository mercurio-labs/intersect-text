# Enum composition acceptance example

Authored X02 fixture, using the NativeV43 composition and enum codec.
The child replaces Choice spellings warm/cool while preserving the exact Shade
literal declarations Red/Blue. Both annotated and implicit overrides are tested;
explicit super::Choice retains red/blue. Same-spelled OtherShade literals are
nominally distinct and must not permit an override. Final and orphan controls
use the existing shared override validator.

This verifies enum-to-enum composition only. Enum-valued ordinary parser rules,
enum-valued terminals, empty-package imports and general datatype conversion
remain separate unsupported obligations. Existing thirteen-case upstream
checker evidence is documented in docs/xtext-gate-audit-2026-10-13.md; these
fixtures are authored and do not copy upstream sources or claim new oracle runs.
Native and WASM controls share the same Rust assertions.

NativeV43 additionally admits an inherited parser entry when the child has only
enum or terminal declarations. Earlier profiles reject these child shapes.
Standalone missing-entry and no-local-rule grammars retain explicit rejection.
