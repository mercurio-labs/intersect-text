# Pinned parameter forwarding and empty resources

Independent Xtext 2.38.0 parser/model observations, frozen before the native
forwarding/empty-resource repair. These two additional exact source graphs do not
replace or reduce the existing 74-graph parameter matrix or full X03/X07/S09 gates.

Four requests retain a rooted success, two syntax errors, and one error-free
empty resource. RuntimeOracle's `accepted` means a non-null root without errors;
therefore the empty resource has `accepted:false`, `errors:[]`, `model:null` and
`resource_contents:0`. This is neither a syntax rejection nor a constructed root.
Native parsing must preserve the error-free empty resource as an empty model,
and a nullable forwarded child must remain null. Rejected JVM partial models
remain separate from native atomic rejection.

The original direct optional-guard sources reached independent inference but
failed Java generation (`Unhandled parameter types: [null]`). A separately
recorded spelling using `||` rejected at grammar syntax; the corrected single
`|` condition retained the generator failure. The final sources add the explicit
`node` keyword before the optional guard and retain forwarded boolean use.
Those variants do not qualify the original generator failures. Exact plans,
logs, generated sources and capture observations remain in the primary checkout's
ignored support evidence directory (`parameter-empty-*`, `target/p-empty-01` to
`target/p-empty-04`). No earlier fixture or verifier was edited.

The shared construction observer initially rejected the error-free empty result
because its older matrix requires a rooted success or an explicit parse failure.
This supplemental verifier checks complete generation prerequisites with that
same frozen observer, validates each ordinary outcome unchanged, and separately
admits only the explicit error-free empty-resource shape. No original expectation
or parser rejection is converted into a success. Java remains oracle-only.

Portable integrity: `python -B fixtures/xtext/inference-parameter-empty/verify.py --integrity-only`.
Independent replay: pass `--jdk D:/dev/jdks/jdk21 --artifact-dir` with a new short
output directory outside fixtures, using the pinned local Maven cache.
