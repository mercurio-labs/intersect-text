# Library demonstration language

An unpublished, bounded language package over `intersect-model` and
`intersect-text`. Handwritten Rust owns Library terminal spellings, reference
names, declaration placement and allowed actions. It consumes the existing
Library Rcore/Xtext fixtures and pinned common.Terminals source; shared services
own parsing, linking, model mutation, validation, serialization and source edits.
There is no production Java dependency.

`language`, `sources` and `plan` provide the interactive host path. The actions
cover field/reference changes, target renames, Loan append, leaf deletion and
same-resource leaf reordering. Unrelated bytes and stable object identities are
preserved. Deletion keeps all comments; movement carries internal comments and
leaves surrounding comments in place. Invalid or ambiguous spellings reject
without changing the accepted session.

Run `cargo test -p intersect-library --locked`. The native example is
`cargo run -p intersect-library --example library_text --locked`; the browser host
is Toolkit's `/library-text.html`. See the [contract](../../docs/text-editing-guide.md)
for verification commands and explicit general-language limitations.
