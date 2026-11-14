# Grouped object assignment alternatives

Authored NativeV36 example reusing the unchanged current-actions Ecore/Rcore.
Both scalar = and collection += containment assignments select an object parser
rule from a grouped alternative. Each branch must produce one conformant object.
Single-child grouping preserves that value through the existing text runtime.
Mixed scalar/reference/fragment RHS values, incompatible classes and optional or
multiple values in one branch remain rejected. Native/WASM controls verify exact
containment, identities, spans, replay, limits and malformed-input recovery.
This is not full assignment syntax or pinned KerML qualification.
