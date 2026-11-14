# Terminal decisions: authored X04/X05 probes

Use model.xtext and example.model with ../terminal-spellings/model.ecore and
../terminal-spellings/model-behavior.rcore, explicitly selecting NativeV13. Existing
metamodel source identities are retained; this is not a pinned KerML grammar.

The native and WASM controls live in xtext_terminal_decisions.rs and
terminal_decisions_probe.rs. See docs/xtext-gate-audit-2026-10-13.md for the
acceptance boundary and two explicit independent-oracle limitations.

Optional regeneration: run oracle.py with --jdk and --artifact-dir. --verify
compares both observations.json and generation-rejections.json. The Java harness
and grammar bodies are authored verification probes; no Java runs in production.
Provenance records input/harness/output and upstream tool artifact hashes.
