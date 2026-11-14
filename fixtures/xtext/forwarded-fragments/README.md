# Forwarded objects with typed fragment continuations

Authored NativeV35 teaching grammar using the unchanged current-actions Ecore and
Rcore. An abstract Expression rule forwards to a concrete OperatorExpression;
a following typed fragment appends operands to that same object. The existing
current-object flow proves construction before fragment access and rejects
replacement or missing-object paths. The runtime is shared without a new algorithm.
Controls cover alternative constructors, scalar prefixes, exact identities/spans,
containment and values, replay, limits/recovery, invalid ownership and native/WASM.
This is not full pinned KerML or general Xtext qualification.
