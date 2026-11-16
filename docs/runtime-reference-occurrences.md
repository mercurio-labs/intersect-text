# Construction-time reference occurrences

`DraftLanguage::parse_runtime_into` and `parse_runtime_into_with_settings`
retain active parser references in `StoredEvaluation::unresolved_references`.
Each occurrence includes the exact source URI and UTF-8 byte range, owner,
feature, spelling, expected target class, and optional collection index.
Speculative parser branches and overwritten scalar references are excluded.

This metadata does not claim that a reference resolved. Construction-only
parsing leaves `StoredEvaluation::references` empty. A language's separate
linker can associate the retained occurrences with its verified target identities
for navigation, or identify an editing probe's actual grammar reference feature.
No global-name lookup or language visibility policy is introduced in Text.

The frozen Catalog integration test checks retained counts and exact spans
through the ordinary parser, then links using its source-authored Rcore scopes.
KerML/SysML consumes this same API for reference navigation and contextual editing.
