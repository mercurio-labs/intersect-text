# Branching recursive predicate analysis

Authored NativeV39 grammar with separate left/right recursive call trees.
Ten branching levels preserve distinct call-return contexts. Depth-first
compile-time analysis finds a conflicting recursion witness within the same
10,000-work and 128-call limits; NativeV38 retains its budget rejection.
Runtime prediction and model construction are unchanged. Controls cover x/y
paths, nested recursion, exact identities, containment, spans, package replay,
limits, recovery and first-predicate rejection. This is not full X11 closure.
