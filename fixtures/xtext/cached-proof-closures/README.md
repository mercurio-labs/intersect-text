# Cached syntax proof closures

Authored NativeV41 example using unchanged current-actions Ecore/Rcore.
One branch repeats a 30-fragment x/y recognizer before `left`; another consumes
exactly 100 x/y keywords before `right`. Repeated syntax closures retain exact
return stacks, and their construction and propagation share the original proof
budget. NativeV40 retains budget rejection. Short forms compare against the
uncached proof. Model, span, identity, replay, limit, malformed and guarded
ambiguity controls run in native Rust and real WASM. No full gate is closed.
