# Inherited grammar example

Status: native, real WASM and optimized browser acceptance pass. This is a bounded
inheritance example; see docs/xtext-day10.md for evidence and remaining limits.

`model.xtext` inherits `base.xtext`, including its hidden policy. The child entry
must produce a Model named p containing a Type named Wheel. Calls to Name and Type
resolve through the effective child grammar while their declaration ownership
remains in base.xtext. Bundle inputs must retain exact per-file identities and
explicit dependency edges. This independent example is MIT-licensed repository
source, not copied upstream KerML source. It models only the inheritance mechanism
identified in the pinned grammar; it does not implement KerML semantics.

See docs/xtext-day10-design.md for rejection controls, replay obligations and the
pinned whole-grammar requirements that remain open.
