"""Frozen guard evidence; Java is an independent, optional verification oracle."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
sys.dont_write_bytecode = True

HERE = Path(__file__).resolve().parent
COMPONENTS = {"cases.json", "observations.json", "native-before.json", "assemble.py", "verify.py", "README.md"}
COUNTS = {"graphs": 19, "ordinary_metadata_accepted": 13, "ordinary_metadata_rejected": 2,
          "ordinary_compiled": 12, "ordinary_generator_failed": 1, "ordinary_requests": 103,
          "ordinary_rooted": 14, "ordinary_rejected": 89, "unexecuted": 25,
          "inferred_compiled": 4, "inferred_requests": 32, "inferred_rooted": 6, "inferred_rejected": 26}


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def checked(path, digest):
    data = Path(path).read_bytes()
    assert hashlib.sha256(data).hexdigest() == digest, "SHA-256 mismatch: " + str(path)


def validate(cases, observed, before):
    assert cases["version"] == observed["version"] == 1
    ordinary, inferred = cases["ordinary"], cases["inferred"]
    assert len(ordinary) == 15 and len(inferred) == 4
    assert len({c["id"] for c in ordinary + inferred}) == 19
    obs = observed["ordinary"]
    assert [c["id"] for c in ordinary] == [r["id"] for r in obs["metadata"]] == [r["id"] for r in obs["generation"]]
    for case, meta, generated in zip(ordinary, obs["metadata"], obs["generation"]):
        assert meta["imported_structure_unchanged"] and meta["imported_metadata_unchanged"]
        assert meta["imported_before"] == meta["imported_after"]
        for source in case["sources"]:
            assert hashlib.sha256(source["source"].encode("utf-8")).hexdigest() == source["sha256"]
            assert source["uri"] == f'memory:/ordinary-parameters/{case["id"]}/{source["id"]}.xtext'
        for model in case["metamodels"]:
            assert hashlib.sha256(model["source"].encode("utf-8")).hexdigest() == model["sha256"]
        if not meta["all_sources_accepted"]:
            assert generated == {"id": case["id"], "stage": "metadata_rejected"}
        else:
            assert generated["provider"]["ecore_hashes_match_exact_imported_inputs"]
            provider = generated["provider"]["generation"]
            assert provider["stage"] == "compiled" and provider["generation_exit"] == provider["compilation_exit"] == 0
            assert provider["source_unchanged"] and not provider["error_keys"]
            scaffold = case["oracle_model_provider"]["scaffold_case"]
            assert provider["source_sha256"] == {s["id"]: hashlib.sha256(s["source"].encode("utf-8")).hexdigest() for s in scaffold["sources"]}
            assert provider["ecore_sha256"] == case["oracle_model_provider"]["scaffold_ecore_hashes"]
            assert generated["generated_ecore_files"] == []
            if generated["stage"] == "generator_failed":
                assert case["id"] == "root_false" and generated["exit_code"] != 0
                assert any('because "ele" is null' in d for d in generated["diagnostics"])
                assert "java.lang.IllegalArgumentException: Unhandled parameter types: [null]" in generated["diagnostics"]
            else:
                assert generated["stage"] == "compiled"
                assert generated["generation"] == {"exit_code": 0, "compilation_exit": 0, "source_unchanged": True, "generated_ecore_files": []}
                assert set(generated["parser_entries"]) == {s["grammar"] for s in case["sources"]}
                assert all(v and v == sorted(set(v)) for v in generated["parser_entries"].values())
        expected_runs = case["runs"] if generated["stage"] == "compiled" else []
        actual = [r for r in obs["runs"] if r["case"] == case["id"]]
        assert [(r["entry"], r["text"]) for r in actual] == [(r["entry"], r["text"]) for r in expected_runs]
    assert {r["id"] for r in obs["generation"] if r["stage"] == "metadata_rejected"} == {"dead_arity_false", "empty_sequence"}
    unexecuted = [{"case": c["id"], "reason": g["stage"], "requests": c["runs"]}
                  for c, g in zip(ordinary, obs["generation"]) if g["stage"] != "compiled"]
    assert obs["unexecuted_requests"] == unexecuted
    assert [r["case"] for r in obs["unregistered_context"]] == [r["id"] for r in obs["generation"] if r["stage"] == "compiled"]
    assert all(r.get("exception") or r["errors"] for r in obs["unregistered_context"])
    for row in obs["runs"] + observed["inferred"]["runs"]:
        assert row["entry"] == "resource"
        if row["accepted"]:
            assert row["exception"] is None and row["errors"] == [] and row["resource_contents"] == 1 and row["model"]
        else:
            assert row.get("exception") or row["errors"]
        for model in row.get("model") or []:
            assert model["class"] and isinstance(model["values"], dict)
            for value in model["values"].values():
                if isinstance(value, dict) and "object" in value:
                    assert 0 <= value["object"] < len(row["model"])
                    assert row["model"][value["object"]]["class"] == value["class"]
    shared = load(HERE.parent / "inference-fragment-runtime/verify.py", "guard_inferred_validation")
    shared.validate(HERE.parent / "inference-fragment-runtime", inferred, observed["inferred"])
    counts = {"graphs": len(ordinary) + len(inferred),
              "ordinary_metadata_accepted": sum(m["all_sources_accepted"] for m in obs["metadata"]),
              "ordinary_metadata_rejected": sum(not m["all_sources_accepted"] for m in obs["metadata"]),
              "ordinary_compiled": sum(g["stage"] == "compiled" for g in obs["generation"]),
              "ordinary_generator_failed": sum(g["stage"] == "generator_failed" for g in obs["generation"]),
              "ordinary_requests": len(obs["runs"]), "ordinary_rooted": sum(r["accepted"] for r in obs["runs"]),
              "ordinary_rejected": sum(not r["accepted"] for r in obs["runs"]),
              "unexecuted": sum(len(r["requests"]) for r in unexecuted),
              "inferred_compiled": sum(g["stage"] == "compiled" for g in observed["inferred"]["generation"]),
              "inferred_requests": len(observed["inferred"]["runs"]),
              "inferred_rooted": sum(r["accepted"] for r in observed["inferred"]["runs"]),
              "inferred_rejected": sum(not r["accepted"] for r in observed["inferred"]["runs"])}
    assert counts == COUNTS, counts
    # Do not confuse removing an inactive alternative with admitting epsilon.
    by_request = {(r["case"], r["text"]): r for r in obs["runs"]}
    for name in ["alternatives_false", "nested_choice_false"]:
        assert not by_request[name, "pre post"]["accepted"]
        assert by_request[name, "pre skip post"]["accepted"]
    assert by_request["all_false_nested", "pre post"]["accepted"]
    # The exploratory author's assumption is retained, but the independent
    # metadata rejection is authoritative and must not be relabeled to agree.
    empty = next(c for c in ordinary if c["id"] == "empty_sequence")
    assert empty["author_expected_metadata_acceptance"] is True
    # Known defect evidence is not an acceptance contract or an upstream result.
    assert [(r["case"], r["text"]) for r in before] == [(c["id"], r["text"]) for c in ordinary[:2] for r in c["runs"]]
    defect = next(r for r in before if r["case"] == "mandatory_false" and r["text"] == "pre post")
    upstream = next(r for r in obs["runs"] if r["case"] == defect["case"] and r["text"] == defect["text"])
    assert upstream["accepted"] and defect["model"]["elements"] == []
    assert any(d["message"] == "Boolean guard is false" for d in defect["diagnostics"])
    control = next(r for r in before if r["case"] == "mandatory_true" and r["text"] == "pre alpha post")
    assert not control["diagnostics"] and len(control["model"]["elements"]) == 2
    return counts


def frozen(repo):
    provenance = read(HERE / "provenance.json")
    assert set(provenance["components"]) == COMPONENTS
    assert {p.name for p in HERE.iterdir() if p.is_file()} == COMPONENTS | {"provenance.json"}
    for name, digest in provenance["components"].items():
        checked(HERE / name, digest)
    for name, digest in provenance["shared_components"].items():
        checked(repo / name, digest)
    cases, observed, before = [read(HERE / name) for name in ("cases.json", "observations.json", "native-before.json")]
    counts = validate(cases, observed, before)
    assert provenance["counts"] == counts
    assert provenance["native_support"] is False
    shared = load(HERE.parent / "inference-fragment-runtime/verify.py", "guard_pinned_dependencies")
    original, _, _ = shared.frozen(HERE.parent / "inference-fragment-runtime")
    assert provenance["jars"] == original["jars"]
    return provenance, cases, observed, before, counts


def replay(repo, output, jdk, cache, cases, expected):
    output = output.resolve()
    assert output != repo / "fixtures" and repo / "fixtures" not in output.parents
    output.mkdir(parents=True, exist_ok=False)
    plan = output / "ordinary-plan.json"
    plan.write_text(json.dumps({"version": 1, "cases": cases["ordinary"]}, indent=2) + "\n", encoding="utf-8", newline="\n")
    def run(command, logfile, allowed):
        result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8")
        logfile.write_text(result.stdout + result.stderr, encoding="utf-8", newline="\n")
        assert result.returncode in allowed, (result.returncode, str(logfile))
    old = repo / "fixtures/xtext/imported-parameters"
    metadata = output / "metadata/observations.json"
    run([sys.executable, "-B", str(old / "replay_metadata.py"), str(plan), str(output / "metadata"), str(jdk), str(cache)], output / "metadata.log", (0,))
    # Exit 1 alone is never accepted as the expected generator obstruction:
    # ordinary() checks the exact failed case, phase, diagnostics and no execution.
    run([sys.executable, "-B", str(old / "replay_runtime.py"), str(plan), str(metadata), str(output / "runtime"), str(jdk), str(cache)], output / "runtime.log", (0, 1))
    assembly = load(HERE / "assemble.py", "guard_assembly")
    ordinary = assembly.ordinary(cases["ordinary"], read(metadata), [output / "runtime"])
    fixture = repo / "fixtures/xtext/inference-fragment-runtime"
    oracle = load(fixture / "verify.py", "guard_inferred_replay")
    provenance, _, _ = oracle.frozen(fixture)
    inferred = oracle.replay(fixture, provenance, cases["inferred"], jdk, cache, output / "inferred")
    actual = {"version": 1, "ordinary": ordinary, "inferred": inferred}
    (output / "actual.json").write_text(json.dumps(actual, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    assert actual == expected, "Fresh independent observations differ; frozen expectations were not changed"
    (output / "receipt.json").write_text(json.dumps({"state": "read_only_replay_passed", "native_support": False,
        "observations_sha256": hashlib.sha256((HERE / "observations.json").read_bytes()).hexdigest()}, indent=2) + "\n", encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=HERE.parents[2])
    parser.add_argument("--replay", type=Path)
    parser.add_argument("--java-home", type=Path)
    parser.add_argument("--maven-cache", type=Path)
    args = parser.parse_args()
    repo = args.root.resolve()
    _, cases, expected, _, counts = frozen(repo)
    if args.replay:
        if not args.java_home or not args.maven_cache:
            parser.error("Replay requires --java-home and --maven-cache")
        replay(repo, args.replay, args.java_home.resolve(), args.maven_cache.resolve(), cases, expected)
    print(json.dumps({**counts, "native_support": False, "read_only_replay": bool(args.replay)}))


if __name__ == "__main__":
    main()
