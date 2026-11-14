# Assigned-current action acceptance fixture

This independently authored grammar/metamodel is executable in the isolated NativeV5 candidate.
Its expected tree is `(1 + 2) - 3`, with one Model, three Literals and two
OperatorExpressions. The outer operator owns the previous plus expression and the
last literal, preserving left association. No arithmetic evaluation is claimed.

Unlike the actual pinned metamodel, this independent teaching model intentionally
has a writable operand containment. It does not replace or edit upstream input.
`provenance.json` retains every exact assigned-current action span in the pinned
KerMLExpressions grammar and the actual InvocationExpression.operand flags. That
upstream feature is derived, transient and volatile; its eventual update behavior
must live in the KerML language package. Generic parser support must not turn a
derived feature into a writable one or embed KerML relationship algorithms.

Required controls: single literal, repeated plus/minus with exact tree/identities,
normal and initializer-backed construction, failed speculative capture rollback,
exact object/step limits, incompatible/abstract action class, wrong multiplicity,
missing current object, non-containment and derived/non-changeable target rejection.
Native construction, initializer and rollback controls pass; native/WASM and exact
archive replay pass. Browser and remaining daily acceptance checks are tracked in
`docs/xtext-day13-design.md`.

Upstream snippet strings are short EPL-2.0 excerpts retained solely as source
identity evidence. Full source remains in the pinned Pilot worktree. This fixture
and the generic runtime implementation are independently authored repository code.

Run the frozen NativeV4 baseline with an existing built native transport:

```powershell
python -B fixtures/xtext/current-actions/verify_baseline.py --native target/debug/examples/rcore_behavior_parse.exe
```

This verifies assigned-current rejection, execution of the corresponding existing
simple-action control, and derived-feature rejection. It retains the older profile rejection contract independently of NativeV5. The detailed integration design is in
`docs/xtext-day13-design.md`.
