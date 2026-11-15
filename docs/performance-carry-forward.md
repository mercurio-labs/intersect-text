# Performance changes carried forward on November 15, 2026

The net committed runtime changes after the pre-dev baseline were ported onto
the independent active `develop` baseline. All affected pre-change runtime files
matched that baseline, and the runtime patch applied without conflict. The active
repository retains its independent history and canonical Model dependency.

The port includes scan-local lexer frontiers, terminal and transition reuse,
bounded owner-set reuse, shared parser return prefixes, linking conformance reuse,
scan-local reference counts, borrowed ASCII terminal input, and bounded parser and
lexer decision memoization. It also carries the ordinary parser's shared identity
and pending-reference spellings and propagating checkpoints, plus the benchmark
host's pooled allocator and in-place large-buffer growth. The custom allocator is
host-only; it does not replace a production application's allocator.

`performance-carry-forward.json` inventories the forty source commits and imported
runtime/benchmark files. Many source commits only contain historical evidence or
rejected experiments. The port takes their net committed runtime implementation;
saved prototype/rejected patch files are not applied. Historical raw measurements
remain attached to the archived revisions. Full EMF/Xtext performance parity is
still open, and this migration does not requalify existing browser artifacts.

Follow the README quick checks for native, strict lint, formatting, and WASM.
Exercise budgeted and ordinary parsing, rollback, Unicode input, resource lifecycle,
and diagnostic behavior. All maintained Model dependency declarations and locks
must select the same canonical full revision. Explicit local coordinated-pair
checks are useful before pushing, but do not prove GitHub fetchability.

`benchmarks/java-parity/README.md` describes the retained benchmark hosts. Java
remains an external reference, never a production dependency. Broad performance
comparisons and million-object runs remain milestone-only work.

The local validation record is `performance-validation.json`. It binds the tested
runtime files to their hashes and records native, WASM, lint, API, dependency,
and allocator checks. The paired Model revision is recorded in
`text-compatibility.json`. No canonical GitHub-fetch qualification or new browser
artifact is claimed before the migration branches are published and checked.

KerML and the focused Examples Toolkit still select their existing baseline
pins. Their contributor guidance requires landing Model, then Text, before
updating those consumers. Mercurio's existing application branches and pins were
left intact; this source port does not merge unrelated application work.
