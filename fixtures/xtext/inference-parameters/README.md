# Pinned Xtext parameter inference and execution evidence

Independent Xtext 2.38.0/EMF observations, authored before native parameter
adoption at `a22cf147`. Native support and full X03/X07/S09 remain OPEN.

The fixture preserves three separately identified variants: 33 original graphs,
33 lowercase-namespace counterparts, and eight additional boundary graphs. The
counterparts change only namespace capitalization; the verifier checks every
source, dependency and proposed model request. They do not replace the originals.

All 58 proposed original model requests are retained as `planned_runs` and remain
unexecuted: 22 originals pass inference but fail Java generation's package-name
heuristic, while 11 reject at inference. Metadata acceptance is not a parser.
The lowercase counterparts and added controls retain complete generated-parser
outcomes, successful models, rejected inputs, partial error resources and named
entry exceptions. Rejected JVM partial models are not native atomic rejection
equivalence. Complete source/origin and model projections remain explicit.

Coverage includes boolean/named/positional arguments and ordering, forwarding,
negation, conjunction/disjunction and precedence, guarded assignments/actions,
optional/repeated groups, scalar rules, typed/wildcard/inherited fragments,
recursive environments, overrides, missing/unknown/duplicate arguments and
parameter identities. Pinned Xtext accepts duplicate declarations, including a
used name resolved to the first declaration. Resource entry uses false parameter
values; named parameterized parser entry throws a missing-entry-method exception.
Those observations must not be guessed from AST import or inferred structure.

Java is an independent verification oracle only. The verifier reuses the pinned
fragment runtime pipeline, validates its immutable files/dependencies and JARs,
and additionally pins parameter validation/evaluation source members. It neither
downloads dependencies nor regenerates expectations. Existing frozen fixtures
and the 515-obligation/98-oracle-case ledger remain unchanged.

Portable integrity:

~~~sh
python -B fixtures/xtext/inference-parameters/verify.py --integrity-only
~~~

Independent replay with cached pinned dependencies:

~~~sh
python -B fixtures/xtext/inference-parameters/verify.py --jdk D:/dev/jdks/jdk21 --artifact-dir D:/dev/repos/mercurio-labs/intersect/target/p-param-replay-01
~~~

Use a new, short artifact directory on Windows. The initial long-path observer
failure is retained independently; the same constant-guard source and requests
pass at a shorter path. Exploratory ID-only lexer timeouts remain unresolved and
are not relabelled as rejections or replaced by this fixture's explicit WS and
ANY_OTHER terminal inputs. Original plan/source/log hashes and failure receipts
remain in the primary checkout's ignored support evidence directory.
