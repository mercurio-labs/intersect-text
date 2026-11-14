# Explicit lexical policy and pinned terminal declarations

An independently authored parser/metamodel composes five exact declarations from
pinned KerMLExpressions: ID, UNRESTRICTED_NAME, STRING_VALUE, REGULAR_COMMENT and
WS. Original source header, license, source hash and per-declaration byte ranges
and hashes are preserved. The parser has its own grammar/source identity.

The example has two resources and five objects. A significant documentation
comment becomes a Model attribute. A quoted name identifies a Type and a Feature
reference; its quotation marks and escape spelling are deliberately retained.
Only the declared WS token is hidden. These raw spellings are not qualification
of KerML value converters or namespace scope.

Six native groups pass, including all 735 selected cached upstream terminal-body
observations. Native/WASM parity passes 12 groups. The cached observations are
reference evidence, not a fresh Java oracle execution or proof of token selection.
The native test also exercises acyclic terminal fragments, Unicode boundaries,
committed failures, nonbreaking-space/vertical-tab rejection, significant comment
rejection at the wrong parser position, cyclic/missing/hidden fragment controls,
call nesting and adversarial repeated delimiter prefixes.

Overlapping terminal starts (including the full ML_NOTE/SL_NOTE combination),
numeric conversion/prediction, inherited hidden policy, rule-local hidden policy,
wildcards, arbitrary negated expressions and the full KerML grammar remain open.
Legacy lexical examples retain their original contract and rejection assertions.
Optimized WASM/Vite, 18 development and 24 production browser tests pass.
See docs/xtext-day08.md for verification and docs/xtext-day08-design.md for next work.
