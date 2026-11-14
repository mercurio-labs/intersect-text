# Replayable definition-tree iterators

Compile `iterators.ecore`, `iterators.xtext` and `iterators-behavior.rcore` with
`conversions.json`, then parse `sample.iterators`. Standard and declared iterator
attributes default to null and remain distinct nominal types. The provider scope
selects the Session declaration at exact source path `[1]`; source edits require
updating both source hashes and declared datatype identities.

`ecore.definition-tree-iterator/1` traverses retained definition nodes in preorder,
including each selected root. The wire snapshot contains exact `roots`, `position`
(the number of successful next calls) and sorted unique 1-based `pruned` positions.
Every snapshot is replayed against the immutable source tree before use. Cursor
revisions, forged positions and invalid prune histories reject.

The native adapter exposes start, has_next, next and prune. Exhaustion returns no
item without advancing. Pruning skips descendants of the most recently returned
node, is idempotent and rejects before any result. Removal rejects because this
backend is immutable. Overlapping selected roots intentionally repeat their
subtrees. Iterating retained extension nodes does not make them executable.

The JSON host method `iterator/step` takes a retained package, exact datatype,
action and (except start) checked converted cursor. It returns the checked cursor,
optional item declaration and has_next. `intersect/stepIterator` in the worker uses
the active package. These pure transitions do not publish model edits; normal
checked model/Python transactions publish the resulting cursor.

Ecore marks ETreeIterator non-serializable. Lexical defaults, parsing and formatting
reject, while checked internal JSON/SQLite snapshots can resume at the next exact
node. The pinned generic E parameter is retained raw; generic instantiation is
still E07. This provider is not an implicit Java iterator or a mutable model tree
adapter. Native hosts can supply other explicit iterator providers. Rcore v2
reads are statically checked; v2 execution remains unsupported.

Evidence: tree_iterator_conversion tests in intersect-language; tree_iterator tests
in intersect-web and intersect-storage-sqlite; and the real browser lifecycle and
Python snapshot control in `web/tests/workbench.spec.js`.
