# Repetition resolved through fragment callers

This authored NativeV32 example uses the unchanged compatibility graph Ecore.
The typed Part fragment assigns the name and appends b labels. Its caller ends
with a c or a d, sharing a first token with the fragment's a b repetition.
The shared prediction graph must prove the repeat decision disjoint across both
caller continuations before compilation succeeds. Controls include ?, * and +,
an ambiguous partition, malformed text, budgets, replay and native/WASM execution.
Nullable unquantified continuation conflicts remain unsupported by this increment.
This is not full pinned KerML or general Xtext qualification.
