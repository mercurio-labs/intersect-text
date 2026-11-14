# Parameter program source-correspondence controls

These are authored native diagnostic controls, not independent JVM observations
or new parameter execution qualification. The original upstream fixtures remain
unchanged. Full parameterization, X03/X07/X10 and S09 stay OPEN.

`cases.json` is the initial plan frozen against `e61308ba`, before implementation.
It contains four failing graphs and six exact source records: a reversed terminal
range, the same range after UTF-8 text with CRLF, an unsupported predicate
assignment, and an inherited range with an unselected revision sharing its URI.
`before.json` contains the original requests and actual old release-WASM results,
including generated error ranges and whole-rule authored correspondences.

The first plan guessed whole-group/assignment expected scopes. It did not claim
independent upstream positions. The actual compiler's generated position is at
the right terminal endpoint and at the predicate prefix. The final
`expectations.json` records their smallest containing authored AST scopes,
`'a'` and `=> b=ID`, with exact UTF-8 bounds. The initial plan, sources and before
capture are retained verbatim. This correction leaves the acceptance requirement
of the narrowest complete containing AST scope unchanged; it awards no new
syntax acceptance or upstream span equivalence.

The source rewriter now retains nested scopes with the generated text. Negative
controls compare the complete legacy diagnostic arrays and generated identities
against `before.json`, then compare the exact new authored identities/scopes.
Native and fresh zero-import WASM consume the same requests. Actual-worker/UI
controls exercise the resulting marker navigation and preserve the active
package. Separate native controls cover specialized call/condition scopes and
whole-declaration diagnostics crossing smaller mappings.

Integrity and source/UTF-8 checks:

~~~sh
python -B -m unittest tools.test_xtext_parameter_origin_evidence
cargo test -p intersect-web --test parameter_origins --test diagnostic_identities --locked
node --test toolkit/tests/parameter-origins.node.js
~~~
