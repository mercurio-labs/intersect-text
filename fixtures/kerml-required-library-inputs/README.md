# Required KerML library source inputs

These are unchanged source inputs from SysML-v2-Pilot-Implementation commit
`692170b71867353b8f90341e61556f49a5beb0e5`. The manifest lists the 22 full files
identified by the existing explicit-qualified-package inventory for `Base`,
`ScalarValues`, and `ScalarFunctions`. Twenty files are copied here. The exact
checked-in `fixtures/kerml-base-scalar-input/{Base,ScalarValues}.kerml` inputs are
reused rather than duplicated. `UPSTREAM-LICENSE` is the unchanged root EPL-2.0
license from that commit. All source bytes and original notices remain intact.
These byte hashes follow the existing pinned checkout manifest. Git blob hashes
are also recorded: checkout CRLF versus blob LF is verified separately; the
payload is never normalized, so existing SourceId hashes remain unchanged.

## Portable input contract

Resolve each `fixture_path` relative to the repository or a packaged asset root,
verify its byte count and SHA-256, then retain its separate `source_id`. The source
URI is the original upstream path, including spaces. The `sources/` asset name is
not a new source URI. This same manifest can drive native embedded inputs or a
browser asset bundle; neither needs an upstream checkout, Java, or host absolute
paths. A browser bundle must include the two referenced existing fixtures too.
No loader, URL decoder, logical-resource remap, constructor, or evaluator is
provided by these files. Explicit provenance-aware resource associations remain
a later checked runtime step.

## Qualification boundary

This is source import preparation only. The 22-file set is a textual full-file
dependency inventory, not proof of implicit/dynamic native closure. It does not
install a semantic library context, raise any resource/work limit, admit the full
original grammar, or prove parsing, linking, scoping, arithmetic delegates, or a
Basic systems workflow. Unsupported relationships and data remain preserved in
the original sources. `ScalarFunctions` declares abstract functions; vendoring it
does not implement operator evaluation. Imported, consumed, and semantically
verified states are deliberately separate.

`fixtures/basic-systems/v1/upstream-standard-library-manifest.json` retains the
larger pinned 94-file inventory. This smaller input package does not replace it.
