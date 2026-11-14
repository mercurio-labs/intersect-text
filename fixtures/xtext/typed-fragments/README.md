# Typed parser fragment acceptance fixture

Status: seven native fragment groups and actual native/WASM parity pass with
explicit `grammar::Profile::NativeV1`. Direct-model values, identity and assignment
state, caller rollback and reference ownership are covered. Three web examples
and nineteen optimized controls passed before profile plumbing; fresh browser and
full-workspace gates are running. No complete Xtext/KerML claim.

Identification mutates a concrete Type or Feature caller without creating an
object. Target stages a reference on its Feature caller. The first declaration
is a fragment; the executable entry is Model. Expected object ordinals are
Model=0, Type=1, Feature=2, with wheel.target referring to Vehicle at ordinal 1.

The source is Intersect-authored simplified syntax with a separate namespace.
Its `feature wheel to Vehicle` spelling and unique-name linking are illustrative,
not claims of actual KerML grammar/scoping equivalence. No upstream bytes were
copied or rewritten. Existing Boolean and dispatch fixtures remain unchanged.

The positive/negative native and actual WASM controls cover
source replay, concrete current object, assignment markers, pending references,
rollback, no phantom allocation, caller FOLLOW conflicts and all rejection cases
listed in docs/xtext-day03-design.md.
