# Native terminal spelling example (X-D07)

This independently authored grammar uses a tagged name to isolate native terminal
recognition, not to claim that @Wheel is standard KerML syntax. TAG contains
literals, character ranges, alternatives and repetition. It serves an attribute
directly and a reference through the ordinary EString Name parser rule. There is
no privileged TAG or Name rule name and no language-specific lexer branch.

Expected: three objects in example.kerml, two in more.kerml, exact spelling and
reference spans, and no object allocation for Name. Frozen previous fixtures and
all budgets remain unchanged. Overlapping FIRST sets, hidden declarations,
terminal fragments/calls, other scalar types, negation and until constructs are
explicitly unsupported in this increment; later lexical increments retain them
as required work. See docs/xtext-day07-design.md for upstream evidence and caveats.

Four native groups and eleven native/WASM parity groups pass. See
`docs/xtext-day07.md` for precise qualification and browser verification status.
