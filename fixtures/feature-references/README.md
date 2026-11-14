# Checked reference feature maps

Compile references.ecore, references.xtext and references-behavior.rcore with the exact
conversions.json. The Library catalog/loans inputs remain usable. BookCopy has
entry and featureMap attributes; their selected Loan.copy feature targets BookCopy.
An immutable context retains the same Ecore source with empty inner feature-map
scopes, a validated Library model, and a detached SpecialCopy subtype. This finite
context avoids recursively embedding its own outer provider contracts.

Host feature-map/reference-bind takes package, datatype, feature and element;
feature-map/reference-read takes package, datatype, feature and reference. Worker
intersect/bindFeatureReference and intersect/readFeatureReference use the active
package. Both entry and map providers share the same checked member implementation.
Binding checks actual object conformance; readback returns the retained Element.
Missing/unrelated objects, wrong features, stale context digests and wire fields
asserting a runtime class reject. No filesystem or URL resolution is performed.

The native host test checks transactions and exported package reload. The SQLite
test checks persisted map validation, rollback and subtype readback after reopening.
The browser control checks native/WASM parity and Python publication/rollback.
These are detached immutable references: mutable ownership, feature-map model
lifecycle and ExtendedMetaData remain E13; Rcore v2 execution remains unimplemented.
