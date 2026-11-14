# Pinned definition conformance inputs

These files support the M01 inventory and subsequent independent tests. They are
not runtime dependencies and do not establish Intersect language support.

## Reproducible targets

| Target | Artifact | Bundle version |
| --- | --- | --- |
| Xtext | `org.eclipse.xtext:org.eclipse.xtext:2.38.0` | `2.38.0.v20250226-0658` |
| Ecore | `org.eclipse.emf:org.eclipse.emf.ecore:2.38.0` | `2.38.0.v20241018-1213` |

[upstream/artifacts.json](upstream/artifacts.json) records the official Maven
download URLs, archive SHA-256, member paths and individual file SHA-256 values.
It retains metamodels, Xtext's own grammar and common terminals, selected service
API sources, manifests, POMs and upstream notices. To recover a file, download its
listed archive, verify the archive hash, extract its exact named member, then
verify the member hash. Do not normalize line endings. The inventory generator
and checker use only the retained files; no Maven, JVM, network or local cache is
needed. Archive binaries are not committed.

All four model/grammar files exactly match the corresponding members of the
Mercurio oracle JAR with SHA-256
`b1ad9d64b1f0c75730facf25a5e2856bc9df2bb4bd39476df2fdf5ae68cd9350`.
The lock records this observation. It does not identify every dependency inside
the shaded JAR or certify runtime equivalence between arbitrary EMF versions.

The pinned Pilot POM declares Xtext 2.38.0; its Eclipse target separately declares
Xtext SDK 2.41.0 and Ecore feature 2.42.0. Both original files are retained under
`mercurio/`. These are distinct build declarations. This milestone deliberately
uses the named 2.38.0 artifacts whose definition bytes match the oracle; it does
not call them the latest releases. Upgrading the compatibility target requires
new pins, an inventory diff and additional obligations where behavior changes.

## Independent cached observations

[mercurio/artifacts.json](mercurio/artifacts.json) records original repository,
revision, path and byte hash for the unchanged preview contract and Pilot controls.
The observations include source text, nodes, reference targets, ancillary controls,
raw disagreements and original tool/source provenance. The preview selects 98
cases: 90 positive stored-model cases and eight syntax negatives, with 1,403
source-constructed node observations and 2,595 reference-slot observations.

The checker joins cases in their original order and verifies source hashes,
language, expected stage, node/reference counts and full oracle-file hash.
No archives, executable witnesses or absolute Windows paths are needed to access
these cases. This is portable **evidence availability**, not a portable rerun of
the historical Java oracle and not an Intersect test pass. Semantic validation
and transformation completion remain unassessed for this preview boundary.
The original full-release exclusions remain in the retained contract.

## Inventory and tests

From the repository root:

```sh
python -B tools/definition_ledger.py --check
python -B -m unittest discover -s tools -p "test_*.py"
```

Regenerate after an intentional reviewed input change with:

```sh
python -B tools/definition_ledger.py
```

[The generated ledger](../../docs/definition-languages-ledger.json) has 515 rows:
350 Ecore and 165 Xtext obligations. It enumerates 53 Ecore classifiers, 36 Xtext
classifiers, every nested metamodel node, 37 named constraints, 57 Xtext syntax
rules, seven common terminals and 20 explicit service obligations. Package nodes,
annotations, overloads, generic arguments and operation parameters retain exact
source selectors. Counts are inventory units, not a semantic completeness score.

[extension-obligations.json](extension-obligations.json) is the handwritten
service supplement. Source selectors must exist in pinned API source. Each row
has an owner/milestone and positive/negative/boundary case plans. These plans are
not executable per-feature tests; those are delivered with the implementation
milestones. Imported, consumed and semantically verified all begin unassessed,
including features of the old bounded profile which this ledger has not retested.
This prevents metadata extraction from silently becoming a qualification claim.

The source model inventory is exhaustive for the pinned files. The service list
is an explicit reviewed starting set, not proof that arbitrary extension code
has been implemented. Add obligations as implementation reveals further contracts.
Rcore v2's own normative grammar and cases are frozen in S01.

## Licenses and attribution

Intersect-authored tooling and case plans use the repository license. Imported
files retain their own licenses; they are not relicensed as Intersect MIT code.

- Eclipse EMF/Xtext files: EPL-2.0; retain the original copyright headers and
  [Ecore about notice](upstream/ecore/about.html) and
  [Xtext about notice](upstream/xtext/about.html). Ecore's notice also retains
  its third-party notices; the referenced runtime binaries are not included here.
- Mercurio-authored evidence: retain [Mercurio's MIT notice](mercurio/LICENSE-Mercurio).
- Pilot-derived material: retain [Pilot's EPL-2.0 license](mercurio/LICENSE-Pilot),
  original repository attribution and nested provenance. This also supplies the
  full EPL-2.0 text referenced by the Eclipse notices.

Java API source files are reference contracts only. They are not built, executed,
or loaded by Intersect. The upstream source archives are directly identified in
the lock for source redistribution and independent inspection.
