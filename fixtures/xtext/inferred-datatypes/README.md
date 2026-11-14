# Inferred EString rules

Independent bounded KerML syntax example using NativeV4. This combines the
cumulative showcase with omitted-return QualifiedName, Name, Spelling, Number and
ConditionalOperator rules. Name calls the later Spelling rule; the child model
rules consume inherited scalar rules. ConditionalOperator accepts the literal if.
The Type operator attribute contains the raw spelling if. Other raw values,
references and the six-object model follow the cumulative showcase contract.

Nine pinned upstream terminal declarations remain byte-identical with their
original EPL-2.0 license and provenance. Inference is implemented in the shared
native grammar analyzer, using the independent inference-contract oracle as
reference evidence. This fixture is not the full pinned KerML grammar.

Recursive/nullable datatype execution, scalar entries, datatype overrides,
additional datatype converters and full KerML semantics remain unsupported.
NativeV1-V3 do not acquire the new inference behavior. Imported, consumed and
verified qualification remains separate; see docs/xtext-day12-design.md.
