# S04 static acceptance corpus

This standalone Ecore/Rcore pair is consumed by `rcore_behavior_s04_corpus.rs` through the
shared source checker. It does not execute functions, closures, derived providers
or SysML algorithms.

`mercurio-shapes.json` retains the three exact feature records and five selected
class records from LATEST SPEC's `ecore-effective.extract.json`. The pinned checkout
HEAD was read again as `32750d7c7799970d7d494e0e8e6e0bf1daae4689`; artifact SHA-256 is
`7c4290959d5687e5245ee7d4c6b018ab337d950d3552402e18977252541fb8dd`.
The records preserve qualified owner/target/feature IDs and original derived,
volatile, transient and other flags. The upstream behavior is explicitly metadata
only. No upstream files were edited and no Java oracle was executed.

`model.ecore` is a generated minimal static projection, not the full metamodel:
it retains the original namespace, selected names/targets, bounds, ordering,
uniqueness and derived flags. It intentionally omits inherited graphs, operations,
volatile/transient execution and all other features. Target classes are identity
stubs in this projection. Their full selected source records remain in the JSON;
projection source digests are native projection identities, not upstream file IDs.
Importing the retained records does not qualify their omitted behavior.

The native consumer proves `ActionDefinition/action` has OrderedSet shape,
`AcceptActionUsage/payloadArgument` is optional and `payloadParameter` is required.
Negative source programs reject kind and multiplicity overclaims. Independent
structural programs exercise Sequence/Set/Bag, tuples, records and generic calls;
these are handwritten controls, not claimed Mercurio algorithms. Tests also keep
source replay and exact feature provenance visible.
