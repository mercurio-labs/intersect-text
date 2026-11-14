# Pinned native Pilot corpus

These are the unmodified upstream definitions, all 94 standard-library files,
and six actual Pilot example files at commit
`692170b71867353b8f90341e61556f49a5beb0e5`.
`manifest.json` retains original source URIs, exact byte hashes and separate,
percent-encoded logical resource URIs. Storage locations do not change SourceIds.
The existing basic-systems authored corpus and frozen fixture contracts remain unchanged.

This commit materializes inputs only. It does not establish native construction,
linkage, validation, ResourceSet activation, or semantic conformance.
Whole-language expected diagnostics for the Pilot examples remain unclassified:
PartTest contains specialization cycles, and Expressions exercises broad syntax
whose complete diagnostic baseline has not yet been frozen. The native runner reports ingestion, linkage,
selected semantic facts and whole-language validation separately. No test text
may be repaired or trimmed to manufacture success.

Upstream license: EPL-2.0, reproduced byte-for-byte in `UPSTREAM-LICENSE`.
