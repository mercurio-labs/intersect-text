# Pinned KerML operand construction evidence

The observations are copied byte-for-byte from Mercurio LATEST SPEC's pinned
mercurio-sysml worktree. provenance.json preserves both repository revisions,
raw file hashes and the original observation provenance. The original oracle
pins Pilot source hashes (LF-normalized), helper hash, JAR and class hashes.
Mercurio evidence is MIT; upstream Pilot semantics are EPL-2.0. Both licenses
are retained. No Java implementation has been copied into production.

Seven concrete invocation classes append two distinct equal-valued literals.
The result is an ordered pair of private ParameterMembership -> in Feature ->
public FeatureValue -> LiteralInteger containment chains, with checked ownership
inverses. The operand facade reads empty. It is a derived containment parsing
adapter, not stored operand data or normative argument evaluation.

Reuse the existing Mercurio adapter design through Intersect's checked setting
plan/store interfaces; do not create a second containment transaction engine.
Preserve fresh/uncontained operands, deterministic identities and collision
rejection. No native Intersect provider or full pinned grammar qualification is
claimed by importing these observations.

Run `python -B verify.py --mercurio-sysml <checkout> --pilot <pinned-checkout>`
to verify exact local evidence, original source/JAR/class identities and graph
shape. This is integrity verification, not Java execution. Fresh oracle replay:
`python -B <mercurio-sysml>/tools/run_operand_construction_probe.py --check
--java-bin <JDK21-or-newer-bin> --pilot <pinned-checkout>`.
The initial JDK17 attempt failed class-file version validation, before execution.
