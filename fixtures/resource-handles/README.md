# Scoped resource handles

Compile `handles.ecore`, `handles.xtext` and `handles-behavior.rcore` with the exact
contracts in `conversions.json`, then parse `sample.handles`. The Catalog has
standard and declared Resource/ResourceSet attributes, all defaulting to null.
The four contracts explicitly select the retained `memory:ecore` document by its
source digest and empty document-root ordinal path. Source edits require updating
both declared datatype identities and selected source identities.

A resource payload is `{ "resource": <SourceId> }`; a set is
`{ "resources": [<SourceId>, ...] }`, inside the checked converted-value envelope.
Null differs from an empty set. Set order is retained; duplicate/stale/unselected
identities reject. Standard and declared datatypes remain distinct nominal types.
The installed adapters are `ecore.definition-resource/1` and
`ecore.definition-resource-set/1` over immutable retained Ecore sources.

Native `ResourceConverter::document` returns actual retained content.
`ResourceConverter::resolve` delegates to the existing E02 resource resolver,
checking that both the referring and resolved resource belong to the handle.
Cross-resource references additionally require exact declared dependencies; no
URI is opened implicitly. Single-resource handles resolve within that resource.

JSON host requests `resource/read` and `resource/resolve` take the retained package,
checked handle and exact `from` SourceId; resolve also takes `href` (for example
`#//Catalog`). Worker requests `intersect/readResource` and
`intersect/resolveResource` use the active package. Requests return the retained
source or resolved declaration, without changing definitions or model state.

Ecore marks both datatypes non-serializable, so lexical defaults and string
conversion reject. Checked internal model/SQLite snapshots and package exports
preserve identities. These adapters do not implement mutable model resource
lifecycle, save/load, notification or timestamp behavior. Those remain later
milestones. Native hosts may supply other explicit resource providers.

Evidence: resource_conversion tests in intersect-language; resource_handles tests
in intersect-web and intersect-storage-sqlite; and the real-worker/Python resource
access control in `web/tests/workbench.spec.js`. Rcore v2 checks reads statically;
v2 execution is still unsupported.
