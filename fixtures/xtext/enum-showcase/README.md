# Cumulative KerML enum teaching example

Select **KerML enum rules** in the isolated Toolkit candidate. This independently
authored grammar extends the inferred-datatype example with visibility, visibility
filter and feature-direction enum rules. It is not the actual pinned KerML package.
Its namespace, grammar names and source digests are separate from the earlier
fixture; that fixture and its acceptance assertions remain unchanged.

Two resources construct six objects. Vehicles::'Wheel assembly' has public
visibility; Library::Spare uses `filter [` to select private. The front and spare
features select out and inout, while preserving their local and cross-resource
targets. Existing Boolean assignments, explicit actions, typed fragments, inherited
parser overrides, quoted qualified names, datatype inference, numeric spelling,
notes and significant documentation remain exercised. Missing enum assignments
use the schema's existing defaults; invalid enum keywords reject the resource.

Enum alternatives bind exact literal declaration IDs. Grammar order differs from
Ecore order, so out and inout must not be interchanged. Project and downloaded
language archive replay must retain all sources, enum bindings and model values.
This example does not implement arithmetic evaluation, actual KerML membership
visibility semantics, derived operands or language-specific Boolean converters.

`provenance.json` retains the hashes of the original teaching sources and all nine
unchanged pinned KerMLExpressions terminal declarations. The copied upstream
copyright header and EPL-2.0 LICENSE remain present. Run `python -B
fixtures/xtext/enum-showcase/verify.py` to check those source contracts offline.
Actual enum-rule upstream evidence is separately retained in `../enum-rules`.
