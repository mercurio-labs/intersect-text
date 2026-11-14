# Recursive predicate branch-entry evidence

These independently authored ANTLR probes record branch selection before its body
executes. They use the pinned first-token and full-syntax guard forms established
by the existing Xtext emitter oracle. They are not the generated parser for the
actual KerML language and do not qualify native support.

All 22 fresh observations distinguish selection from successful completion. For
`a c`, the recursive first-token case enters g and rejects; the full-syntax case
enters f and accepts. For `a`, the former enters g and the latter f, but both reject.
For `b`, neither enters a branch. Both select g for `a a b b` and accept. Trailing
input can reject after a successful branch selection. This rules out substituting
ordinary backtracking, whole-body recognition or successful-parse observations
for first-token commitment.

The exact two grammar files, observations, harness and dependency hashes are
retained through provenance.json and the unchanged parent oracle provenance.
The grammar probes and Python runner are independently authored under the
repository MIT license. No upstream grammar bytes or Java implementation were
copied here; the existing separately authored DecisionOracle harness is reused.

Run oracle.py with --jdk, --artifact-dir and --verify (and --maven-cache if needed)
for an optional offline replay. Jars are hash checked; nothing is downloaded.
NativeV8 continues to reject recursive guarded bodies explicitly. A future native
implementation must match branch entry as well as acceptance, preserve budgets,
rollback and source identities, and pass real native/WASM execution checks.

## NativeV9 teaching example

The separate authored model.ecore/model.xtext/model-behavior.rcore inputs now exercise
recursive first-token commitment. `a a a b b b 12` constructs Left containing
two nested Inner objects. `a a b c` rejects the resource after choosing the guarded
branch; it does not retry Right. NativeV8 retains its explicit rejection.
Select **Xtext recursive predicates** in Toolkit for this example plus a second
resource `a b 4` (four objects total). Edit, recover, save/reopen or export its ZIP.
The fixture is a parser-semantics teaching model, not actual KerML domain modeling.

Run verify.py for portable source/evidence identity checks, or add --native with
the rcore_behavior_parse host executable for all eleven acceptance observations and exact
model containment checks. Native/WASM, archive forgery, budgets and rollback
controls are separate executable tests; source hashes alone award no runtime support.
