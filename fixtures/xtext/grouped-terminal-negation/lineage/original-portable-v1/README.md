# Grouped terminal negation: full-source oracle and bounded native slice

These five exact source graphs were frozen before NativeV62 implementation. The
separate original seven-graph corpus remains unchanged. The original sequence
source is copied byte for byte from the older terminal-negation corpus; the four
other graphs are separately authored companions, using the unchanged imported
Ecore provider and its explicit canonical Rcore native structure adapter.

The independent oracle loads actual Xtext 2.38 sources, generates the provider
from the original Ecore/GenModel, runs StandardLanguage/MWE2, compiles the actual
generated Java, and executes its lexer, IParser and XtextResource routes. Java is
verification only. Production consumes no Java, generated tables or observations.

Both original record and independent fresh replay retain all 40 executions:
15 accepted, 25 rejected, including 18 rejected partial models. Their complete
raw UTF16 tokens, lexer/parser/resource errors, named models and leaves remain
under oracle/. Do not decode isolated UTF16 units as scalar-safe model strings.
The positive resource view retains all 15 successful upstream models, including
those from recovered generation; native consumers qualify only the three clean
cases, 27 inputs and 11 successful models.

A zero MWE2/javac exit does not imply clean generation. original_sequence emits
ANTLR buildnfa error(100), expecting EOA after its first set member, then executes
recovered output. fragment_mixed emits error(100), invalid char literal. Both
original failures and actual runtime/partial outputs are preserved; NativeV62
keeps their source programs explicitly unsupported. A later recovery-policy
increment must prove the actual traversal/error recovery; FIRST-set substitution
or silently repairing those original graphs is insufficient.

The three clean graphs prove grouped multi-unit keywords, a local literal-bodied
fragment call and mixed inline literal alternatives. A multi-unit keyword
contributes no singleton to ANTLR's lexer exclusion set. Genuine singleton/range
members contribute their actual excluded intervals; a multi-unit keyword is not
replaced with its first character. NativeV62 retains the entire authored/resolved
operand and callee identity, reuses the shared one-unit runtime operation, and
validates fresh source on archive admission. Nested new domains, mixed multi-unit
calls, inherited/explicit-parent new calls, multi-element sequences, nullable,
repeated, EOF and Until operands require further evidence or remain unsupported.
Old unit_domain behavior and NativeV1-V61/default inference remain unchanged.

Run `python -B verify.py` for byte/manifest/complete-observation integrity. Optional
fresh generation requires the pinned 60 jars from inputs/stack.json and JDK
21.0.6+7: `python -B verify.py --replay NEW_OUTPUT --jdk JDK --maven-cache CACHE`.
Outputs must be new and outside this fixture. The only observation comparison
exception is the exact absolute, owned provider-<EcoreSHA12>/provider.genmodel
output location; no source, error, model or runtime normalization is allowed.
The original frozen producer/helper bytes and input/receipt lineage are retained.

Native successful model values, identities and UTF8 source spans are checked
separately from full upstream rejected-partial/error streams. Astral-token UTF16
to UTF8 boundaries and broader invalid-input recovery remain explicit limits.
This evidence closes no full X05/X06 or parent support gate.
