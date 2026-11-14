# Composed Xtext web examples

Authored examples reuse unchanged Library and current-actions metamodels and
semantic definitions. Existing advanced-web fixtures and assertions are retained.

Library combines grouped object assignments, object forwarding, nullable typed
fragment writes, explicit/implicit construction, inferred string Name, unassigned
string End and cross-document references. Library language semantics own defaults
and duplicate-loan rejection.

Expressions combine assigned-current actions, nesting, typed fragments after
capture, mixed explicit/implicit construction, grouped operands and fragment
writes after forwarding. This constructs a KerML-shaped model; it does not
evaluate arithmetic or qualify actual pinned KerML grammar/domain semantics.

NativeV42 tests check six Library and seventeen expression objects, identities,
spans, values, references, schema validation, package replay, malformed input,
missing references, duplicate loans, resource limits and recovery. The shared
artifact verifier compares complete native/WASM responses. Browser tests cover
project and ZIP replay. See the daily audit for executed verification status.

These are authored fixtures, with no newly copied upstream implementation.
Mercurio source provenance remains in docs/xtext-day15-design.md and the gate
audit. Inheritance, enums, converters and predicates retain separate examples.
Full X02-X18 obligations remain open.

## V52 web follow-up

The web selector now runs the composed Library and expression examples under
NativeV52. The original library.xtext and catalog.library remain unchanged and
are still tested under V42 through V52. The separate library-v52.xtext and
catalog-v52.library add standard quoted STRING titles alongside bare Name titles.
`"Dune\nVolume 1"` constructs a title containing a newline; `"bad\q"` rejects.
The six objects, defaults, two cross-document links and duplicate-loan policy
remain covered. The expression example retains its seventeen objects and all
operator/literal assertions; it constructs expressions rather than evaluating them.

Browser controls verify project and exported-package replay, malformed-source
rejection and recovery. Production controls inspect the converted title and
exported V52 manifest. The artifact verifier retains V27/V42 cases and adds V52
cases with exact native/WASM response equality. No original source identity is
reassigned to the new fixtures. Standard STRING conversion remains bounded:
isolated UTF-16 surrogates explicitly reject at the scalar model-string boundary.
Full X06 and actual pinned KerML qualification remain open.
