# Punctuation keyword fixture (X-D04)

Status: shared native lexer integration and four native execution groups pass.
Two lexer/admission controls, four matcher controls and all seven native/WASM
groups pass; optimized web verification passes. The recorded pre-D04 run rejected
these keywords.
No full Xtext or actual KerML qualification is claimed.

This Intersect-authored example retains a separate namespace. It demonstrates
punctuation token boundaries, Boolean keyword state and subtype dispatch.
Import all/recursive flags and specialization targetName strings are illustrative;
no KerML import resolution, namespace scope, specialization or redefinition
semantics are implemented by this fixture. The existing language package owns
interpretation and uses unchanged model/store services. No upstream bytes copied.

Expected objects: the first document constructs Model plus two Import and two
Specialization instances. The second constructs Model plus one Import. Exact
ordinals, spans, values, unsettable markers and native/WASM equality are required.
Reject split keywords (`: :`, `:/*gap*/:`, `* *`, `: >`, `:> >`), incomplete
suffixes, reserved keyword IDs, unsupported keyword/comment interactions and
budget overages. Do not repair token boundaries by joining across hidden trivia.

Existing Boolean, dispatch and fragment fixtures remain unchanged.

The normal optimized build, five development Xtext tests and all 20 production
tests pass. The rendered four-class diagram was inspected with zero displayed
problems. Full pinned KerML semantics remain unsupported.
