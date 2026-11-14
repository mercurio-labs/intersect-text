# Explicit collection providers

Compile collections.ecore, collections.xtext and collections-behavior.rcore with the
exact conversions.json, then parse sample.collections. Catalog has standard and
declared list/map attributes with null defaults and exact operation type identity.
Standard lists contain EInt; standard maps contain String keys and nullable nested
standard lists. Declared lists contain Mode enum values (numeric aliases remain
distinct); declared maps contain nominal Count values. Source edits require
updating all exact declaration digests in the contracts.

Installed ecore.list/1 binds org.eclipse.emf.common.util.EList; ecore.map/1 binds
java.util.Map, matching the pinned Ecore metadata. Both are nonserializable:
lexical conversion rejects, while validated internal model snapshots persist.
List payloads use {"items":[...]}; maps use {"entries":[{"key":...,"value":...}]}.
These payloads live inside the standard converted-value envelope. Order is
preserved, duplicate list items are allowed, and duplicate map keys reject.
Key equality is exact checked wire equality, preserving decimal scale and float
bits. Null members require an explicit nullable member contract.

The native collection adapter implements atomic list insert/set/remove/move and
map get/put/remove plus clear. The catalog recursively reconstructs explicit
member providers under a 128-provider total and 16-level depth limit. Native
hosts may supply other implementations; no Java or ambient provider discovery is
used. Raw pinned type parameters are retained, not instantiated (E07). Rcore v2
execution and general collection expression algebra remain later work.

Host/worker model edits, export and SQLite use the same validators. JSON host `collection/step` and worker `intersect/stepCollection` expose the same
native operations through a tagged `operation` object. `empty` takes no collection;
other operations take the checked `collection` envelope. The result contains the
updated `collection` and `{present,value}`, so a present null map value differs
from a missing key. These are pure transitions; normal model/Python transactions
publish their results. Raw dictionary edits and these operations do not execute
Rcore collection expressions.
