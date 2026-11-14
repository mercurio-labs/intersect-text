# Actual pinned KerML requirement inventory

This report is generated from Mercurio's retained Xtext AST export at revision
32750d7c7799970d7d494e0e8e6e0bf1daae4689 and Pilot revision
692170b71867353b8f90341e61556f49a5beb0e5. It retains exact raw source hashes,
UTF-16 declaration spans and UTF-8 slice digests. Sources are read as bytes;
CRLF is never normalized. The cached exporter used Java/Xtext. This audit is not
native grammar extraction, compiled schema admission or semantic verification.

The effective KerML grammar has 282 declarations: 270 parser rules, nine terminals
and three enum rules. It includes 65 fragments, 360 assignments (240 +=, 94 =,
26 ?=), 34 actions (13 simple and 21 operand+= current captures), 51 cross-references
and two first-token predicates. There are no full syntactic predicates, rule
parameters/arguments or unordered groups in this pinned effective input. Those
remain eventual full-Xtext requirements, not immediate pinned-KerML syntax needs.

Both first-token sites call FeatureSpecialization, whose 25-rule closure is
acyclic. Each containing specialization-part closure has 45 rules and is also
acyclic. Therefore NativeV9 recursive predicate support must not be described as
having unlocked those particular pinned predicates. Executable actual metamodel
admission, derived operand construction, complete grammar analysis and language
scoping remain separate obstacles. Existing teaching examples do not prove them.

Run verify.py --mercurio <Mercurio checkout> --upstream <Pilot checkout> to replay
all hashes, spans and counts against the unchanged snapshots. --emit prints a
fresh observation for review and never rewrites the retained expected file.
The upstream source license is EPL-2.0; source notices remain in the pinned files.
The inspection script is independently authored under the repository MIT license.
