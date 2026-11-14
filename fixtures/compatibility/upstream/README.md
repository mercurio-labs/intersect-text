# Pinned upstream controls

Files in `emf/` and `xtext/` are unmodified Eclipse project test/example inputs,
retained under their upstream EPL-2.0 license (see the adjacent licenses/notices).
`provenance.json` records repositories, exact commits, original paths and byte
digests. They are upstream examples/regressions, not an OMG certification suite.
They are not relicensed under Intersect's MIT license.

`john.xmi`, `mary.xmi`, and `family.xmi` use schemas constructed programmatically
in EMF's `tests/org.eclipse.emf.test.core/src/org/eclipse/emf/test/core/change/
SpecialCasesTest.java`, methods `testNoChange` and `testLoadChangeDescritpions`,
at the pinned EMF commit. The Rust controls reproduce their structural declarations
and compare known names and relations; they do not implement EMF change recording.

`xrefs0.xmi` and `xrefs1.xmi` are tested only against an explicitly labelled stored
projection of `xrefsmodel.ecore` (name, others, nonOthers). The original metamodel
contains an unimplemented derived, volatile, transient reference and must fail
native full-schema import. Passing its XMI projection is not support for that
derived behavior. Other metamodels/grammars are retained as unsupported controls,
without silently deleting or translating their unsupported constructs.
