# E05 bounded Mercurio controls

`mercurio-enum-rules.json` is the unchanged `enum-rules.extract.json` from the
Mercurio LATEST SPEC baseline `32750d7c7799970d7d494e0e8e6e0bf1daae4689`, under
`crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/`.
SHA-256: `0a76cbca06b55dbe9943911ac8dd497b86a83cab9c527d0488d07e90c5ce09db`.
The file retains its grammar and Ecore input hashes. Applicable notices are
[Mercurio](../definition-conformance/mercurio/LICENSE-Mercurio) and
[Pilot](../definition-conformance/mercurio/LICENSE-Pilot).

The E05 test projects each rule's literal/value pairs into a small Ecore enum and
exercises the native codec. Projection order is grammar rule order, **not** the
original Ecore declaration order; this fixture cannot establish the upstream
enum's default. It checks preserved literal names, numeric assignments and the
distinction between grammar tokens and Ecore lexical spellings. In particular,
`accept` is a grammar token for the `trigger` literal, not an Ecore literal alias.
The test does not execute Xtext enum rules, Java, or whole SysML models.
