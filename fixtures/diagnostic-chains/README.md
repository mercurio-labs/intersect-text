# Explicit diagnostic chain example

Compile `diagnostics.ecore`, `diagnostics.xtext` and `diagnostics-behavior.rcore` with the
exact selections in `conversions.json`. Parse `sample.diagnostics` to construct a
Report whose standard and declared chain attributes default to null. The Ecore
source URI is `memory:ecore`; changing the source requires updating its contract
hash. Standard and declared types remain distinct, including in Rcore typing.

A present chain payload is `{ "diagnostics": [] }`, inside the provider's checked
`kind: converted` envelope. A diagnostic has all of these fields:

```json
{
  "severity": "error",
  "source": "example.validation",
  "code": 12,
  "message": "Required part is absent",
  "data": [{"kind":"reference","resource":"model.sysml","revision":"exact-revision","fragment":"part/0"}],
  "children": []
}
```

Severity is ok/info/warning/error/cancel. Source and message may explicitly be
null. Data may be a scalar with an existing builtin datatype and exact wire value,
an inert reference with resource/revision/fragment, or a checked invocation_error
value. References record evidence; they never load or resolve resources. Other
opaque data rejects. Children and chain entries preserve order and duplicates.

`DiagnosticChain::add`, `add_all` and `merge` share native algorithms. The JSON
host's `diagnostic/collect` request takes a retained package, checked chain,
operation and diagnostic and returns a checked chain plus reported severity.
The worker exposes `intersect/collectDiagnostics` using the active package. This
is a pure snapshot transformation: publish its result through a normal checked
model transaction. Invalid input leaves the existing model untouched.

The pinned Ecore datatype is non-serializable; lexical defaults and string
conversion reject. Internal JSON/SQLite snapshots are supported, separately from
lexical serialization. This is an explicit portable adapter, not Java object
import or an automatic host DiagnosticChain. Rcore v2 reads have static checks;
v2 execution is still outside the implemented profile.

Evidence: diagnostic_chain integration tests in intersect-language, intersect-web
and intersect-storage-sqlite, plus the native/WASM accumulation and Python control
in `web/tests/workbench.spec.js`.
