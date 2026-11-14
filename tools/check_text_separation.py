"""Permanent model/Xtext source and dependency ownership guard.

Model permits no typed text/compiler coupling. Historical separation budgets
are evidence only and cannot authorize an exception.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parent.parent
FORBIDDEN = {"intersect-text", "intersect-xtext", "intersect-language", "intersect-library",
             "intersect-web", "intersect-text-acceptance"}
XTEXT_FORBIDDEN = {
    "intersect-model-test-support", "intersect-model-acceptance",
    "intersect-language", "intersect-library", "intersect-web", "intersect-text-acceptance",
    "intersect-codegen", "intersect-rust-bindings-tests", "intersect-semantic-bindings-tests",
    "intersect-resource-lifecycle-tests", "intersect-model-text-tests",
}
PATTERNS = {
    "grammar_reference": re.compile(r"\bgrammar\s*::"),
    "text_feature": re.compile(r"\btext-compiler\b"),
    "text_recipe": re.compile(r"\b(?:XtextInferenceInputs|CompilationSourcePolicy|BundleCompilation)\b"),
    "foreign_text_path": re.compile(r"\bintersect[-_]xtext\b"),
    "typed_grammar": re.compile(
        r"\b(?:Grammar|LexerPrediction|ParserPrediction|TokenCursor|RuntimeGuardedAdmission|"
        r"CompiledRuntimeGrammar|PreparedLanguageDefinition|DraftRuntimeCompilation|"
        r"SharedDraftRuntimeLanguageDefinition|SourceBoundDraftRuntimeDefinition)\b"),
    "grammar_policy": re.compile(r"\b(?:language_reference_key|language_rule_class_key)\b"),
}


def source_counts(root: Path) -> dict[str, dict[str, int]]:
    result = {}
    source = root / "crates/intersect-model/src"
    for path in sorted(source.rglob("*.rs")):
        relative = path.relative_to(root).as_posix()
        text = path.read_text(encoding="utf-8")
        counts = {name: len(pattern.findall(text)) for name, pattern in PATTERNS.items()}
        if path == source / "definitions/grammar.rs" or source / "definitions/grammar" in path.parents:
            counts["grammar_backend"] = 1
        counts = {name: count for name, count in counts.items() if count}
        if counts:
            result[relative] = counts
    return result


def source_errors(actual: dict) -> list[str]:
    return [f"{path}: forbidden {name} ({count} occurrences)"
            for path, counts in actual.items() for name, count in counts.items() if count]


def dependency_errors(root: Path, manifest: dict, *, owner: str = "model",
                      forbidden: set[str] = FORBIDDEN) -> list[str]:
    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    workspace_dependencies = workspace.get("workspace", {}).get("dependencies", {})
    errors = []
    if owner == "model" and "text-compiler" in manifest.get("features", {}):
        errors.append("model declares the removed text-compiler feature")
    contexts = [("package", manifest)] + list(manifest.get("target", {}).items())
    for context, tables in contexts:
        for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
            for alias, spec in tables.get(kind, {}).items():
                inherited = isinstance(spec, dict) and spec.get("workspace", False)
                if inherited:
                    spec = workspace_dependencies.get(alias, spec)
                name = spec.get("package", alias) if isinstance(spec, dict) else alias
                if name in forbidden:
                    errors.append(f"{owner} {context}/{kind}: forbidden dependency {name}")
                if isinstance(spec, dict) and "path" in spec:
                    # Check aliases and path dependencies even when their table
                    # key hides the actual package name.
                    base = root if inherited else root / "crates" / f"intersect-{owner}"
                    dependency = (base / spec["path"] / "Cargo.toml").resolve()
                    if dependency.is_file():
                        actual = tomllib.loads(dependency.read_text(encoding="utf-8"))["package"]["name"]
                        if actual in forbidden and name not in forbidden:
                            errors.append(f"{owner} {context}/{kind}: alias {alias} resolves to {actual}")
    return errors


def model_inventory(root: Path) -> dict:
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, check=True,
                              capture_output=True, text=True).stdout.strip()
    return {"version": 2, "revision": revision, "entries": source_counts(root)}


def write_inventory(root: Path, destination: Path) -> None:
    """Record consumers and exact representative inputs without running an oracle."""
    tracked = subprocess.run(["git", "ls-files", "-z"], cwd=root, check=True,
                             capture_output=True, text=True).stdout.split("\0")
    pattern = re.compile(r"\bintersect[-_]xtext\b|\btext-compiler\b|\bgrammar\s*::")
    consumers = []
    for name in tracked:
        if not name or name.startswith("crates/intersect-model/src/"):
            continue
        path = root / name
        if path.suffix not in {".rs", ".toml", ".py", ".mjs", ".js", ".yml", ".md"}:
            continue
        # Upstream evidence may use legacy encodings. Scan ASCII identifiers
        # without converting or rewriting any imported bytes.
        matches = [number for number, line in enumerate(path.read_text(encoding="utf-8", errors="surrogateescape").splitlines(), 1)
                   if pattern.search(line)]
        if matches:
            if name.startswith("crates/intersect-text/"):
                owner, batch = "Xtext", "X3-X6"
            elif name.startswith(("docs/", "benchmarks/")):
                owner, batch = "Documentation or evidence; preserve historical inputs", "X7-X9"
            else:
                owner, batch = "Integration consumer or verification tool", "X7-X8"
            consumers.append({"path": name, "lines": matches, "owner": owner, "batch": batch})
    fixtures = []
    for directory in ("crates/intersect-text/assets/fixtures/library", "fixtures/xtext/model-to-text"):
        for path in sorted((root / directory).rglob("*")):
            if path.is_file():
                fixtures.append({"path": path.relative_to(root).as_posix(),
                                 "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    payload = {"version": 1, "model": model_inventory(root), "consumers": consumers,
               "fixture_inputs": fixtures}
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"Inventory recorded {len(consumers)} consumers and {len(fixtures)} exact fixture inputs.")


def check_ownership(root: Path) -> list[str]:
    errors = []
    model = root / "crates/intersect-model/Cargo.toml"
    text = root / "crates/intersect-text/Cargo.toml"
    if model.is_file():
        manifest = tomllib.loads(model.read_text(encoding="utf-8"))
        errors += dependency_errors(root, manifest) + source_errors(source_counts(root))
    if text.is_file():
        manifest = tomllib.loads(text.read_text(encoding="utf-8"))
        errors += dependency_errors(root, manifest, owner="text", forbidden=XTEXT_FORBIDDEN)
    if not model.is_file() and not text.is_file():
        errors.append("No owned Model or Text package was found")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, help="Write the complete consumer and fixture inventory")
    args = parser.parse_args()
    if args.inventory:
        write_inventory(ROOT, args.inventory)
    actual = source_counts(ROOT)
    errors = check_ownership(ROOT)
    if errors:
        print("\n".join(errors))
        return 1
    print(f"Xtext ownership guard passed: {len(actual)} model source files remain inventoried.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
