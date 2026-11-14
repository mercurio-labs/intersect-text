# Source-backed inferred computed collection contexts

Frozen native-extension acceptance plan, based on `c70229d3`. This is not an
independent JVM oracle and closes no parent gate. Existing scalar fixture bytes
and failure expectations remain unchanged.

Both cases retain the complete original authored Rcore structure and behavior,
namespace/dependency identities, Xtext source, and exact provider contracts. The
plain case and parameterized `Item<true>` case use `computed+=ID` over a writable
source-backed projection into an unsettable, nonunique, ordered String collection
with upper bound three. `Model` contains the imported `Item`; plain named
`Item` parsing must construct that same authored class without the parent. The
parameterized graph activates only `Item<true>` and retains its original
no-argument `Item` rejection; no named-parameter closure is claimed.

The original unique candidates are retained in `shape-conflicts.json`: scalar `+=` infers a nonunique collection, so inference rejects both bounded and unbounded imported unique collections before provider admission. Their original shape failures remain explicit.

Required consumers and outcomes:

- Default text and the plain no-argument named entry execute real Append plans and read back `alpha,beta`;
  the backing values and explicit assignment history agree.
- Script Add appends `gamma`; Unset restores empty default and clears explicit
  setting state; later Add and Set can reuse the same provider after Unset.
- Undeclared Set/Unset operations reject even though an executable writable
  projection exists. Append is mandatory for these grammars; omitted Append
  fails admission. Unsupported computed Remove/Insert/etc remain explicit.
- Invalid atoms, upper-bound violations and undeclared
  operations roll back and cannot publish. A new valid transaction succeeds.
- Fresh source replay, both source ZIP forms and actual workers reconstruct exact
  contracts. Bad context activation preserves prior package/model state. Explicit
  ZIP/context re-import after worker restart reconstructs the same source model.

`before.json` records actual requests and results from the preserved c70229d3
release. Some existing text and Unset controls may already succeed; their exact
results remain regression expectations. The extension is judged by the missing
script append consumer and exact contract enforcement, not by relabeling existing
capabilities. Re-import with an explicitly supplied archive/context is not durable
editor/project recovery. Full provider/resource/origin qualification stays open.
