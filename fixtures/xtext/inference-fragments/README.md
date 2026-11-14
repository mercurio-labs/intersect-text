# Full-source parser-fragment inference observations

The 45 independently authored graphs contain 60 exact grammar sources. Pinned
Xtext 2.38 accepts 28 complete graphs and rejects 17. Thirty child entry resources
appear error-free: two inherit invalid parents. All 15 parent metamodel snapshots
and the original imported package remain unchanged, including origins.

The independent FragmentOracle loads source resources, resolves references, runs
native Xtext metamodel inference and full validation. It follows the earlier
CompositionOracle resource workflow and reuses the unchanged MetamodelOracle
metadata serializer. It adds actual parser fragment/wildcard flags to declarations
and resolved call targets. Default cross-reference calls synthesized by Xtext
are retained with `synthetic: true` and null start/end spans; authored calls have
real node spans. No grammar AST wiring or generated Ecore is manufactured.

Coverage includes implicit/explicit typed fragments, optional and nested calls,
shared caller types, multiple callers, enums, cross-references, recursive typed
fragments, imported return types, scalar fragment returns, wildcard features and
nested wildcard/typed contexts, unused fragments, inheritance and three-level
chains. Rejections include assigned fragment calls, invalid actions, conflicting
feature types, empty generated packages, fragment/production category changes,
typed/wildcard override changes and redeclared fragment return types. One negative
parent has only a wildcard fragment as its first rule; another has an invalid
assigned fragment call unused by its accepted child.

Inference acceptance is separate from executable support. In particular,
`wildcard_typed_call` infers no Common supertype on Model even though the wildcard
body calls a typed fragment. `typed_datatype_unassigned` infers an EString fragment
return. Preserve these observations; executable compilation must make any missing
runtime contracts explicit. Do not rewrite the upstream expectations to fit the
current native compiler.

The next implementation contract is exact native type/category/feature projection
for these complete source contexts, preserving null wildcard return identities,
caller ownership and source origins. Reuse the existing source composer, fragment
interpreter and model services; extend those shared consumers where required.
Verify actual models and source-request replay wherever execution is claimed,
with negative/recovery controls and native/WASM agreement. Existing 41/31/21-case
expectations remain immutable. Parameter/guard inference, full X03/X07/X10,
archive/browser lifecycle adoption and full Rcore/S09 qualification remain open.

An initial capture exposed a missing-node assumption in the old composition
adapter for default cross-references. Only this new adapter was repaired before
freezing; earlier helpers and observations are byte-identical. The verifier has
no record mode, downloads or production Java dependency. Replay outside fixtures:

~~~sh
python -B fixtures/xtext/inference-fragments/verify.py --jdk /path/to/jdk --artifact-dir /path/to/oracle-output
~~~

Java is an independent oracle only. These observations alone award no native
fragment inference, model execution, browser support or qualification closure.
