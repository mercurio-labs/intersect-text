# Simple leading action fixture (X-D06)

This authored example targets concrete construction through abstract declared
returns: Type returns Element starts with {Type}; Feature similarly selects
{Feature}. A shared Identification fragment writes the caller. The origin field
belongs to Type alone, so successful admission must use the action-selected class.
The installed constant initializer writes origin=constructor before authored fields;
Vehicle overrides it with authored, while Wheel and Spare retain constructor.

Verified by focused native and native/WASM controls: four objects in example.kerml and two in more.kerml,
with no temporary Element objects. The wheel Feature links to Wheel. Construction
receipts, assignment markers, ordinals, source spans and exact source replay are
part of acceptance. Failed models must remove constructed objects and references.
Additional controls cover action-only roots, incompatible/abstract target types,
multiple or misplaced actions, assigned-current actions, initializer selection,
and exact object/step/nesting limits.

The pinned KerML root uses a leading Namespace action; Mercurio has a separate
simple-action executor control. Provenance records their hashes and licenses.
Neither alone establishes Intersect support. Before implementation the native compiler rejected
this fixture at its first action; the complete request/result and executable hash
are retained in xtext-day01/actions-fixture-red.json. No existing fixture changes.

The native profile now imports and executes this bounded leading-action pattern.
See docs/xtext-day06.md for implementation, verification and remaining gaps.
