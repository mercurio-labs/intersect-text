# Pinned source-size policy prerequisite

The untouched pinned SysML Ecore is 629,610 bytes. The existing 256,000-byte
compiler/document policy rejects it before the actual KerML operand action can
be analyzed. `observations.json` retains its Pilot revision, path and SHA-256.
The source stays in its original package; it is neither split nor minified.

The additive `Document::parse_with_limits` and observed counterpart share the
existing parser. An explicit exact-size policy parses 182 classifier XML nodes
and retains original source bytes and identity. Default and one-byte-short
policies reject. This is imported document evidence only: it is not executable
schema, full grammar consumption, provider execution or semantic qualification.

Build the host inspector with `cargo build -p intersect-definitions --example
inspect_ecore_document --locked`, then run:

```powershell
python -B fixtures/xtext/pinned-source-policy/verify.py --upstream <pinned-pilot-directory> --inspector <built-inspect_ecore_document.exe>
```

Observed parsing retains default receipts and cancellation/storage/work checks.
XML nesting limits are unchanged. Resolver/compiler/grammar/archive propagation
of an explicit policy remains required before full pinned language compilation;
all existing public APIs and browser defaults retain their original boundaries.
No Java or network access is required.
