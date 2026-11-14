# Source-loaded terminal-library inference evidence

The 21 independently authored graphs contain 46 grammar sources. Sixteen complete
graphs have no upstream errors; five reject. Nineteen entry resources appear
error-free in isolation: three inherit invalid parents. Preserve that distinction
and reject invalid complete graphs instead of awarding entry-only acceptance.
All 25 parent metamodel before/after snapshots and the original imported package
are unchanged.

Every common.Terminals source is byte-for-byte org.eclipse.xtext 2.38.0,
SHA-256 3e3d37770a731b0d5270b456743fc70331aa7aaf1c99afa6728896c64bbe401a.
The exact source and EPL-2.0 license header remain in ../terminal-parents/Terminals.xtext;
they are also preserved in the authored graph inputs. Source loading, linking,
inference and full validation reuse the unchanged CompositionOracle and metadata
serializer. No AST links or generated metamodels are fabricated by an adapter.

Coverage includes terminal-only parents without a model, common scalar terminals,
references, enum children, STRING datatype wrappers, inherited hidden policies,
WS/ID overrides and explicit super calls, terminal fragments, and three-level
terminal/scalar/generated-package chains. Negative observations retain missing
Ecore imports, invalid aliases/types, malformed unused parent declarations and
incompatible return-type overrides, including all partial generated metadata.

Native adoption must prepare libraries without inventing a schema, metamodel or
model entry, then use the existing language inference and shared executable source
composer for actual generated models. Preserve exact terminal and fragment source
identities, source-request replay, rejection/recovery and host policies. Existing
STRING conversion is a separate shared service. ID conversion, scalar UTF-16
representation, arbitrary providers, full X03/X06/S09, generic archives/browser
adoption and general qualification retain their original open obligations.

The optional read-only verifier pins all earlier frozen evidence, JARs, inspected
source members, both unchanged Java helpers and the frozen upstream lock. It has
no expectation recording or dependency-download mode. Replay outside fixtures:

~~~sh
python -B fixtures/xtext/inference-terminal-parents/verify.py --jdk /path/to/jdk --artifact-dir /path/to/oracle-output
~~~

Java is an independent oracle only. This evidence awards no native inference or
execution support by itself. Original 41-case and 31-case expectations are unchanged.
