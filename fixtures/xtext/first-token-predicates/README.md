# Pinned first-token predicate requirement

`provenance.json` records the two exact `( -> FeatureSpecialization )+` spans in
the pinned EPL-2.0 Pilot KerML grammar. Owners are FeatureSpecializationPart and
PayloadFeatureSpecializationPart. Source remains in its original upstream package;
these short retained spans are imported evidence, not executable support.

An independent current-action teaching-grammar probe compiles unchanged and
rejects after prefixing its initial Literal call with `->`. NativeV5 reports
`Unsupported Xtext construct '-'` at 242..243. Host evidence in
`target/xtext-daily-artifacts/xtext-day01/predicate-baseline.json` retains both
requests, complete results and executable hash. No fixture or assertion was
weakened to bypass the rejection.

The next increment must establish first-token prediction behavior against an
independent Xtext oracle, distinguish it from full syntactic predicates and
ordinary backtracking, preserve profile contracts, and verify successful choices,
committed malformed alternatives, rollback, source spans and native/WASM replay.
Recognition alone cannot qualify prediction semantics. Actual KerML also retains
its package delegate/derived operand and other language-owned semantic gaps.


## Independent predicate decision evidence

The optional oracle uses the hash-pinned Xtext 2.38.0 `FirstSetComputer` and
actual predicate emitter, followed by cached ANTLR generation/runtime tools.
`oracle-provenance.json` records every JAR and harness hash, generator class
hashes, observation hashes and the read-only Mercurio source trace at revision
`32750d7c7799970d7d494e0e8e6e0bf1daae4689`. The Xtext binary must still match
the frozen upstream lock. No dependency is downloaded, and Java is used only
for this independent verification workflow.

Nine first-set observations cover nullable prefixes, alternatives, actions,
fragment calls, repetitions, assignments and cross-references. The actual Xtext
emitter produces `('a')=>` for a first-token predicate and `('a' 'b' )=>` for
a whole-syntax predicate on the same keyword group.

The 88 decision observations include ordinary alternatives, optional groups,
repetition and recursive ambiguity, with valid and malformed inputs. The first
three contexts use the emitted guards. Recursive guards are independently
authored ANTLR probes; these are not complete generated Xtext languages.
ANTLR may recover after an error and run branch actions: `accepted` therefore
requires zero syntax errors, rather than merely a branch marker or successful
process exit. All probe inputs use declared tokens, avoiding lexer-error-only
acceptance.

For the simple choice, both predicates accept `a c` through the fallback. For
the recursive choice, `->` rejects `a c` and `a a b c`, while `=>` accepts them
through the fallback. Both accept `a a b b` through the guarded branch. Thus
ignoring the predicate, unconditional first-token commitment at every decision,
and unrestricted backtracking all fail parts of this evidence. Native support
needs decision analysis as well as first-set recognition. The shared grammar
compiler currently requires disjoint FIRST/FOLLOW sets, and the model machine
uses journaled rollback; those are the implementation boundaries to extend.

Run from the repository root, supplying an installed JDK and a fresh artifact
directory (the Maven cache defaults to `~/.m2/repository`):

```text
python -B fixtures/xtext/first-token-predicates/verify.py --jdk PATH_TO_JDK --artifact-dir PATH_TO_OUTPUT
```

This verifies independent observations only. NativeV5 still rejects `->`;
no new native/WASM feature or conformance obligation is awarded. Full language
generation, model construction under predicates, and actual pinned KerML
execution remain unverified by this fixture. Existing acceptance fixtures,
limits, source identities and the released demo remain unchanged.

Verification checkpoint: the fresh oracle replay passes all 99 observations
(9 first sets, 2 emissions, 88 decisions). Exact pinned KerML spans and Mercurio
revision/source hashes were rechecked. A changed observation file is rejected by
its digest. The unchanged 515-obligation ledger and 28 integrity tests pass.
Generated parsers/classes/logs are confined to the supplied artifact directory;
`.gitattributes` preserves hashed fixture bytes across checkouts. Rust/WASM
execution gates do not apply to this oracle-only change, and no native support
claim is added.


The same verifier now also checks `visibility-provenance.json` and its five
independently authored ANTLR grammars. A visible entry guard resolves duplicate
alternatives; guards hidden behind an action, token or called-rule boundary
produce warning 200/error 201. Twenty-two valid/malformed parser observations are
retained for the admitted visible and branch-entry cases. The original 99 observations and their
hashes remain unchanged. These additional 27 observations characterize generator
visibility and a bounded parser probe, not full Xtext generation or native model
execution. `EXTRA_C` declares the rejection token explicitly, preventing lexer
recovery from silently hiding malformed input.

## Native execution fixture

`model.ecore`, `model.xtext` and `model-behavior.rcore` are independently authored native
execution examples, not imported KerML. In candidate NativeV6, `model { a b 12; }`
constructs a Model and a Left entry. The visible `->Left` guard resolves the shared
syntax prefix against Right. `model { a b 2147483648; }` rejects integer conversion
and rolls back the resource; it must not retry Right's string-valued branch.

`tests/acceptance/tests/language/xtext_predicates.rs` additionally exercises a
shared-prefix fallback and a typed-fragment variant with the pinned KerML-shaped
`(-> specialization)+ multiplicity? specialization*` sequence. Neither is full
KerML execution. See `docs/xtext-day15-design.md` for capabilities and exclusions.
