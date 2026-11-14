# Independent source-to-Ecore inference observations

Status: frozen acceptance inputs; **native X03 support remains open**.

The independently authored adapter loads 41 exact Xtext grammar sources with the
pinned Xtext 2.38.0 standalone resource loader, resolves their references and runs
its full resource validator. This invokes the actual upstream Ecore inference
path. The matrix records 27 grammars with no error diagnostics and 14 rejected
grammars. Warnings and errors remain separate in every complete observation.
This establishes neither native Intersect inference nor generated parser/model
execution or general Xtext qualification.

The observations preserve generated package aliases/names/URIs, classifier
identities and order, declared inheritance, feature types and flags, multiplicity,
containment, enum names/literals/values, available upstream source-adapter spans
and complete diagnostic outcomes. Rejected grammars retain their partial Ecore
observations separately from acceptance. One exact authored imported Ecore
package is loaded afresh for each case; its complete before/after structural
comparison stays unchanged in all 41 cases.

The sources cover attributes, containment and cross-references, alternatives and
current-type flow, optional and unassigned calls, simple and assigned actions,
feature lifting, multiple generated packages, shared return types, enums and
mixed imported/generated classes. Rejections cover feature kind/type/cardinality
conflicts, imported-package mutation, unresolved types/aliases, duplicate generated
aliases, cycles and conflicting inherited features.

Pinned observations include details a native implementation must preserve:

- An optional unassigned model call still contributes a subtype relationship;
  the following assignment returns to the caller's current type.
- Assigned alternatives with unrelated model types use the Ecore EObject domain.
- Duplicate enum literal names reuse the first literal/value and preserve both
  source origins.
- A new explicit parser return is an EClass in this pinned path; a new custom
  terminal datatype is rejected. Current descriptive documentation must not
  override these inspected version-specific outcomes silently.
- Lifting equal features to two independent supertypes can produce a rejected
  inherited-feature collision. Multiple inheritance without that collision has
  its own accepted control.

The harness and inputs are repository-authored; upstream sources stay in their
original archives and licenses. `provenance.json` pins all fixture bytes, the
existing standalone-parser runtime dependency set, the upstream source archive
and inspected inference members. Auxiliary dependency versions are recorded;
they do not alter the repository's frozen qualification stack.

With the cached Maven artifacts and an installed JDK, replay without network:

```powershell
python -B fixtures/xtext/metamodel-inference/verify.py --jdk D:/dev/jdks/jdk21 --artifact-dir D:/dev/repos/mercurio-labs/intersect/target/xtext-inference-replay
```

The verifier is read-only with respect to these expectations and has no recording
mode. Compiled classes, replayed inputs and output stay in the explicit artifact
directory. Java is an optional independent verification oracle only; production,
native builds, inference implementation and WASM must never require it. Pure
Python integrity tests run with the normal tooling suite.

The [full implementation contract](../../../docs/archive/xtext-metamodel-inference.md)
keeps native import/inference, semantic consumption and qualification separate.
