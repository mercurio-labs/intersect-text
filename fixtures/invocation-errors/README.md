# Invocation error provider example

`errors.ecore`, `errors.xtext`, `errors-behavior.rcore` and `sample.errors` form a runnable
bounded language. Compile with the explicit contracts in `conversions.json`.
The contracts pin the standard datatype and the local declared datatype to their
exact source revisions. Changing errors.ecore requires updating its source hash.
The retained Ecore source URI for this single-source compile is `memory:ecore`.

Text creates `attempt example` with null failure/localFailure and empty history.
Checked native or browser Python transactions can assign converted values. The
payload inside the standard contract's `kind: converted` envelope can be:

```json
{"message":"Invocation failed","target":{"domain":{"name":"model.validation","version":"1"},"code":"constraint_failed","message":"Required part is absent"}}
```

Both message fields may be null or empty, and the entire target may be null.
The standard and local datatype envelopes are distinct; they are not interchangeable.
`history` is a unique ordered list of standard error values. Missing/opaque fields
and invalid domains reject; failed transactions do not publish changes.

Ecore marks this datatype non-serializable. Source lexical defaults and string
conversion therefore reject. JSON/SQLite model snapshots use checked internal
values and are separate from lexical conversion. There is no Java exception
construction, arbitrary Throwable deserialization or operation execution here.
Rcore v1 is executable for this example; Rcore v2 model reads are type-checked
in the acceptance tests but v2 execution remains unimplemented.

Evidence: `intersect-language/tests/invocation_error_conversion.rs`,
`intersect-web/tests/invocation_error.rs`,
`intersect-storage-sqlite/tests/invocation_error.rs`, and the real browser/Python
invocation error control in `web/tests/workbench.spec.js`.
