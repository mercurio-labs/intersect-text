# Generated Xtext fragment construction oracle

This batch compares exact source-loaded inference with the actual Xtext 2.38
EMF/parser generator, Java compilation and model construction. Java is an
independent oracle only; no normal build or production dependency is added.

The ten source graphs preserve four original cases from inference-fragments
byte-for-byte and add six separately identified controls. The lowercase override
control changes only the grammar namespace identifiers and matching with clause;
its rule bodies and metamodel URIs stay unchanged. The original uppercase-named
graph is retained independently, including its default-generator naming failure.
The reachable override control exposes an inherited Root parser through a Holder
containment; the compatible/incompatible typed controls distinguish actual feature
admission from nominal caller inheritance. An unused assigned-fragment control
retains its body without creating an executable caller for it.

Before native changes, the batch contract is:

- Preserve every earlier frozen source and observation. Do not rename an original
  fixture to make generation pass or count a derived control as its exact source.
- Load all source resources, perform full inference/validation, generate actual
  parsers and EMF models from valid graphs, compile generated Java and execute the
  default resource parser plus explicitly requested rule-entry probes.
- Distinguish inference rejection, generator failure, compilation and runtime
  construction. Retain source identities, generated entry-method inventories,
  model class/feature values, object ordinals, resource contents and exception
  chains. Do not equate source inference acceptance with executable support.
- Reuse shared native fragment execution and checked model writes. Generic
  wildcard callers must not acquire fabricated supertypes/features. Verify actual
  feature writes and rollback against the oracle, including incompatible callers.
- Keep complete X03/X07/X10 and Rcore/S09 qualification open unless their original
  contracts are independently satisfied.

The read-only replay verifier has no expectation recording, downloads, generated
AST wiring or Java naming/typing overrides. The 60 cached generator/runtime JARs
are pinned, including the same Xtext/EMF versions as the earlier inference oracle.
Generated sources, classes, raw logs and source-loaded snapshots stay outside
fixtures. Positive controls must generate, compile and execute successfully.

Initial observations distinguish three important cases. A wildcard body can call
an ordinary typed fragment using a caller that owns compatible features even
without the fragment's nominal supertype. A richer override works in its child
Model entry, while the inherited Root entry is not emitted unless reachable.
When reachable, Root without tag parses, and requesting the missing tag fails.
The incompatible scalar write leaves Xtext's partial resource with its earlier
integer value; Intersect's existing atomic rejection publishes no partial model.
Keep that explicit host/model policy difference separate from outcome agreement.

The numeric-string control is particularly important for native adoption: WORD
returns EString even when its spelling is 42. Assigning it through a wildcard to
an integer feature must reject. A field-directed lexical fast path must not turn
that string into an integer and accidentally accept an incompatible rule value.

The frozen capture contains ten graphs and thirteen grammar snapshots. All ten
complete source graphs pass inference validation. Nine generate and compile;
the unchanged uppercase-named override graph fails the default generator's Java
package-name inference. Thirteen parser requests produce seven successful model
constructions and six rejected requests, including two unavailable named entries.
Four original complete inference observations remain identical to their earlier
frozen counterparts. Generated artifact hashes also agree across fresh output
roots for the repeated graphs.

Replay into a new artifact directory (existing directories are refused):

~~~sh
python -B fixtures/xtext/inference-fragment-runtime/verify.py --jdk /path/to/jdk --artifact-dir /path/to/new-output
~~~

This evidence does not itself award native adoption or close a language gate.
