# Rule-specific Boolean conversion baseline

The original checkpoint is evidence and a rejection fixture. The NativeV8
candidate execution is described separately below.
The authored Model has its own namespace. Its Xtext source embeds the exact pinned
KerML Nonunique rule bytes. The actual Feature.isUnique declaration is separately
pinned at classifier 58, feature 12, including its true default. BooleanValue is
also pinned: the same EBoolean return type uses a different selected converter.

provenance.json retains six Pilot source hashes, both exact grammar slices,
converter/service/runtime-module identities, and Mercurio revision
32750d7c7799970d7d494e0e8e6e0bf1daae4689. Three Mercurio source artifacts retain
the native algorithm, generated dispatch and independent cached extraction.
Four resolved bindings and six cached upstream observations distinguish ordinary
true/false from Nonunique -> false in KerML and SysML contexts. These observations
are imported evidence, not a fresh oracle run or native execution qualification.

NativeV7 rejects the EBoolean datatype parser rule. Removing it preserves the
true schema default. Substituting a Boolean keyword assignment parses nonunique
but writes true: that successful parse is deliberately a semantic rejection
control for the proposed implementation. Native/WASM replay asserts both facts.
No existing fixture, default budget or earlier profile contract is changed.

Run verify.py with --upstream pointing to the pinned Pilot checkout, --mercurio
to Mercurio LATEST SPEC's mercurio-sysml checkout, and --native to rcore_behavior_parse.
The verifier checks byte hashes, exact slices, declaration identity, cached
observations and current native behavior without Java or network access.
The full native/WASM suite is toolkit/tests/xtext.node.js.

The copied upstream notice and EPL-2.0 license cover the grammar text. Mercurio
Rust is referenced by hash only; no executor or converter implementation is copied
in this checkpoint. See ../../../docs/xtext-day17-design.md for the next contract.

## NativeV8 candidate

`native-v8-behavior.rcore` is an authored language-package binding for the exact model.xtext
source digest and Nonunique declaration span. It selects the deterministic
intersect.xtext.keyword-boolean provider at version 1 and explicitly maps
nonunique to false. `example.kerml` exercises it; an absent modifier stays true.
Run the verifier with --native-v8 and a candidate rcore_behavior_parse binary to compare
this result with the retained independent observation. NativeV7's earlier
rejection/default controls remain unchanged. The mapping is not inferred from
the rule name, does not replace the EBoolean schema codec, and requires no Java.
