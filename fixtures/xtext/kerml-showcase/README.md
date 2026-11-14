# Cumulative KerML syntax showcase

This independently authored, bounded language combines the verified D01-D11 Xtext
features. It is not the complete pinned KerML grammar or KerML domain semantics.
The parser/metamodel is repository MIT source. The nine terminal declarations in
base.xtext retain their pinned upstream text, EPL-2.0 header and provenance.json.

The child inherits the model entry, pure parser dispatch, typed Identification
fragment, qualified-name and numeric EString rules, terminals and hidden policy.
It overrides Type and Feature with leading simple actions, Boolean keyword
assignment, integer/exponent/quoted-string values and the multi-character :>
keyword. Two resources demonstrate exact-name linking, including a qualified
quoted name and a cross-resource reference. Significant documentation and hidden
notes are distinct. Save/reopen and language archive replay retain both grammars.

Expected: six objects, Wheel assembly abstract=true/count=12/magnitude=1e3,
Spare abstract=false/count=3/magnitude=12E-3, and two resolved feature targets.
Names and custom string/numeric spellings remain raw; semantic decoding and full
KerML scoping are not claimed. Existing Library and individual Xtext fixtures stay
unchanged. See docs/xtext-wrap-up.md for final verification and remaining work.
