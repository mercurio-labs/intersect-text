# Contextual override acceptance inputs

Status: native, real-WASM and optimized-browser acceptance pass under NativeV3.
See docs/xtext-day11.md for evidence, limits and pending full-workspace integration.

The inherited Model rule calls Member. Compiling base.xtext alone must accept
base-example.kerml and reject example.kerml. Selecting model.xtext must produce a
Model containing Type Wheel from example.kerml and reject base-example.kerml.
Both contexts retain their exact source identities. The child Entry deliberately
calls an inherited parser rule, so replacing only child-authored calls is
insufficient. @Override must retain its replaced declaration's identity.

These inputs are independently authored MIT-licensed repository source, not
copied KerML definitions. They isolate the contextual replacement used by the
pinned KerML ExpressionBody and ResultExpressionMember rules. They do not claim
KerML domain semantics or full grammar qualification. See docs/xtext-day11-design.md.
