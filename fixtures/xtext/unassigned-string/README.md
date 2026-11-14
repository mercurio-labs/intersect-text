# Unassigned EString parser helpers

This authored NativeV33 example reuses the unchanged compatibility graph Ecore.
Prefix and ReferencesKeyword are inferred EString parser rules consumed without
assignment. They do not create or replace the Node. The existing scalar parser
executes their bodies; all model assignments still use the shared runtime.
Controls also cover typed fragment and action contexts, rejection, replay,
source identity, budgets and native/WASM execution. Unassigned non-EString scalar
rules and general object replacement remain outside this increment.

The ReferencesKeyword spellings mirror the pinned Pilot KerML.xtext declaration
(commit 692170b71867353b8f90341e61556f49a5beb0e5; source SHA256
cc02be703955603d2daa91f878f299f950195fcd2e3c59563ae17643ba91dfaa).
The surrounding teaching grammar is authored here; it is not the pinned grammar.
