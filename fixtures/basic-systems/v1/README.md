# Basic systems acceptance corpus v1

Twenty authored workflows and 88 ordered observations cover a bounded basic
KerML/SysML system. All imported, consumed and semantically verified states are
unverified. Syntax grounding is not independent runtime truth.

The complete original pinned Pilot grammars and combined metamodel are a hard
prerequisite. These small multi-file models never replace them with a reduced
grammar or structural projection. `contract.json` preserves exact source pins,
expected values/failures/UTF-8 spans, cases and four required runtime lanes.
`grounding.json` records exact grammar ranges and inspected upstream examples;
`upstream-standard-library-manifest.json` pins 94 original library inputs.

Use `tools/basic_systems_acceptance.py` from the repository root. See
[the milestone and commands](../../../docs/basic-systems-completion.md) and
[the receipt/adapter protocol](protocol.md). The runner has no default Pilot
checkout path and performs no compiler build, browser run or semantic evaluation
unless a real caller-selected adapter is explicitly supplied. Matching adapter
results remain candidates pending independent proof review.

Historical draft runs, synthetic control artifacts and source snapshots belong
under ignored `target/`; they are not product examples or semantic receipts.
Corpus examples were originally authored; upstream example bodies were not
copied. Upstream source/license hashes are retained in grounding.json.

## Draft revision 2 syntax erratum

The original authored S06 negative source used `part def ;`. The exact pinned
Pilot parser accepts that as an unnamed part definition, so the original S06
syntax-rejection expectation was incorrect and has never been awarded a pass.
Draft revision 2 uses `part def Broken : ;` instead. The independent parser
rejects its colon at UTF-8 bytes 42..43 in the unchanged canonical source URI
`memory://intersect/basic-v1/sysml/syntax-error.sysml`. Both original and revised
observations are retained; the other 19 cases and all 88 workflow steps remain.

The original 39-byte source SHA-256 is
`58ce5dca64755ce6412f9299fb8e51bf852f0896e90958c5fe3cdd5b2dfd1fcb`;
the revised 48-byte source is
`efc9189bfeb4577fc4725418558098ad6cab94d1f9e970c6fa4e9598bfc70c30`.
Ignored evidence under `target/basic-systems-s06-erratum-proposal-v1/observation.json`
has SHA-256 `6dd564066a135f1c7fb28dfa5c1dba3c47260c2ca500bb7c93b820ad44e34755`.
The exact revised-identity observation under
`target/basic-systems-s06-draft-revision2-proposal/observation.json` has SHA-256
`333c2ba17cf5f8a22b83c7784c0572c3f6caaf64c83b362f6678588ed341254b`;
its guarded terminal is
`fadb80ba967c4627d0b3e0255c94df7aee6c58c1f5e9e8e110a337d8327c9b09`.
This proves a syntax-control correction only. Native publication rejection,
rollback, recovery and the complete S06 workflow remain unverified, as do all
case support states in the contract. Java is an independent verification oracle
and is not a production dependency.
