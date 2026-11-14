# Authored qualified-name datatype slice (X-D05)

This independent simplified KerML syntax fixture targets explicit EString-returning
parser rules and scalar cross-reference spelling. It is Intersect-authored, not an
import of the actual KerML grammar. Names are matched by the existing unique_names
scope using their exact resulting strings. No namespace/import resolution, quoted
KerML name interpretation, feature-chain semantics or domain algorithms are added.

The native implementation produces four objects in example.kerml and two in more.kerml.
The front Feature links to Parts::Wheel in the first resource; distance links to
Units::Length in the second. QualifiedName calls allocate no objects. Type names
and Feature targets preserve explicit assignment markers and exact source identity.

Before implementation the current native executable rejects both the full fixture
at non-ID reference spelling and an ID-reference variant at the EString rule
return. Exact requests/results and executable SHA-256 are retained in the isolated
xtext-day01/datatype-fixture-red.json audit artifact. These are rejection baselines,
not native support. Conversion expectations come from ../datatype-contract.

Five native execution groups, five accumulator controls and eight parser controls
pass. All eight actual native/WASM groups pass, including source/package replay,
Unicode and nested raw spelling. Original Boolean/dispatch/fragment/punctuation
controls and frozen XMI compatibility pass. The normal optimized build, six development Xtext tests and all 21 production
tests pass. The rendered four-class diagram was inspected with zero problems. The initial rejection evidence above is historical and unchanged.
