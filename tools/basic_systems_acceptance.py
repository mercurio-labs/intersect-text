"""Execute the draft contract; missing admission or adapters are BLOCKED, never PASS.

This host-only runner does not build Rust, install packages, change the repo,
implement language semantics, or rewrite the pinned grammars. JSON stdio adapter
commands are explicit host composition, not production APIs already implemented.
"""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess
import sys
import traceback

REPO = Path(__file__).resolve().parents[1]
HERE = REPO / "fixtures/basic-systems/v1"


def read(path):
    return json.loads(Path(path).read_bytes())


def pin(path):
    path = Path(path)
    raw = path.read_bytes()
    return {"path": str(path.resolve()), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def checked(expected):
    actual = pin(expected["path"])
    assert all(actual[key] == expected[key] for key in ("bytes", "sha256")), expected["path"]
    return actual


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8", newline="\n")


def walk_spans(value):
    if isinstance(value, dict):
        if value.get("unit") == "UTF-8 bytes" and "source" in value:
            yield value
        for child in value.values():
            yield from walk_spans(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk_spans(child)


def integrity(contract, pilot=None):
    assert contract["state"] == "DRAFT_UNQUALIFIED"
    assert contract["hard_prerequisites"]["reduced_grammar_or_projection_forbidden"]
    assert set(contract["required_lanes"]) == {"native", "wasm", "browser-dev", "browser-production"}
    observed = {}
    for key, row in contract["sources"].items():
        actual = pin(HERE / row["path"])
        assert all(actual[x] == row[x] for x in ("bytes", "sha256")), key
        observed[row["uri"]] = row
    ids = set()
    grounding = read(HERE / contract["grounding"])["grammars"]
    for case in contract["cases"]:
        assert case["id"] not in ids
        ids.add(case["id"])
        assert case["mandatory"] and case["steps"]
        assert case["language"] in ("kerml", "sysml")
        assert all(x in contract["sources"] for x in case["sources"])
        assert all(x in grounding for x in case["grounding"])
        assert all(case["status"][x] == "unverified" for x in ("imported", "consumed", "semantically_verified"))
        for location in walk_spans(case):
            row = observed[location["source"]["uri"]]
            assert location["source"]["sha256"] == row["sha256"]
            assert 0 <= location["start"] < location["end"] <= row["bytes"]
        for step in case["steps"]:
            assert isinstance(step["expected"], dict) and step["operation"]
            for key in step["arguments"].get("sources", []):
                assert key in contract["sources"]
    if pilot is None:
        return {"state": "contract_integrity_pass_only", "cases": len(ids),
                "corpus_sources": len(observed), "upstream_verified": False,
                "semantic_qualification": False}
    for row in contract["pilot"]["sources"].values():
        actual = pin(pilot / row["relative_path"])
        assert all(actual[x] == row[x] for x in ("bytes", "sha256")), row["relative_path"]
    libraries = read(HERE / contract["stdlib_manifest"])["files"]
    for relative, expected in libraries.items():
        actual = pin(pilot / relative)
        assert all(actual[x] == expected[x] for x in ("bytes", "sha256")), relative
    return {"state": "contract_integrity_pass_only", "cases": len(ids),
            "corpus_sources": len(observed), "upstream_library_sources": len(libraries),
            "grammar_sources": len(contract["pilot"]["sources"]), "upstream_verified": True,
            "semantic_qualification": False}


def admission_for(contract, qualifications, lane, language):
    rows = qualifications.get("admission", {})
    row = rows.get(lane, {}).get(language)
    if row is None:
        return "No exact full original grammar admission evidence for this language/lane"
    checked(row["receipt"])
    observation = read(row["receipt"]["path"])
    assert observation["state"] == "pass"
    assert observation["ok"] is True and observation["observation_complete"] is True
    assert observation["projection_or_reduced_grammar_used"] is False
    assert observation["source_postguard"]["state"] == "pass"
    closure = contract["hard_prerequisites"]["grammar_closures"][language]
    assert observation["authored_rules"] == closure["authored_rules"]
    assert observation["language"] == language and observation["lane"] == lane
    expected = {contract["pilot"]["sources"][key]["relative_path"]:
                {x: contract["pilot"]["sources"][key][x] for x in ("bytes", "sha256")}
                for key in closure["sources"]}
    assert observation["full_original_sources"] == expected
    assert observation["dependency_edges"] == {language: ["ecore", "expressions"], "expressions": ["ecore"]}
    checked(observation["artifact"])
    checked(observation["raw_admission_result"])
    if lane.startswith("browser"):
        assert observation["static_postguard"]["state"] == "pass"
        checked(observation["static_manifest"])
    return None


def execute_case(contract, case, lane, command, directory, pilot_root=None):
    directory.mkdir()
    request = {"schema": contract["protocol"]["request_schema"], "lane": lane,
               "contract": pin(HERE / "contract.json"), "case": case,
               "source_files": [{**row, "text": (HERE / row["path"]).read_text(encoding="utf-8")}
                                for row in contract["sources"].values()],
               "stdlib_manifest": pin(HERE / contract["stdlib_manifest"]),
               "full_original_grammars_required": True,
               "upstream_input_root": str(Path(pilot_root).resolve()) if pilot_root else None}
    save(directory / "request.json", request)
    with (directory / "stderr.log").open("xb") as errors:
        result = subprocess.run(command, input=json.dumps(request).encode(), stdout=subprocess.PIPE,
                                stderr=errors, cwd=HERE, check=False)
    (directory / "response.json").write_bytes(result.stdout)
    assert result.returncode == 0, f"Adapter exit {result.returncode}; complete stdout/stderr retained"
    response = json.loads(result.stdout)
    assert response["schema"] == contract["protocol"]["response_schema"]
    assert response["case_id"] == case["id"] and response["lane"] == lane
    assert response["state"] == "pass"
    assert response["observations"] == [step["expected"] for step in case["steps"]]
    assert response["imported"] is True and response["consumed"] is True
    assert response["source_postguard"]["state"] == "pass"
    assert response["excluded_result_fields"] == []
    roles = {x["role"] for x in response["evidence"]}
    assert {"complete-raw-model", "complete-raw-result", "complete-raw-diagnostics",
            "language-context", "native-or-WASM-artifact", "independent-semantic-truth"} <= roles
    for row in response["evidence"]:
        checked(row["file"])
    # File pins alone cannot establish independent semantic truth. Keep all
    # support states unverified until actual raw/truth receipt review.
    return {"state": "candidate_pass_pending_review", "response": pin(directory / "response.json"),
            "imported": False, "consumed": False, "semantically_verified": False,
            "candidate_imported": True, "candidate_consumed": True,
            "evidence": response["evidence"]}


def fixture_snapshot():
    return {path.relative_to(HERE).as_posix(): {key: value for key, value in pin(path).items() if key != "path"}
            for path in sorted(HERE.rglob("*")) if path.is_file()}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-draft", action="store_true",
                        help="Verify local corpus; include --pilot-root to also verify upstream byte pins")
    parser.add_argument("--pilot-root", type=Path,
                        help="Explicit unmodified pinned Pilot checkout; no machine default or implicit I/O")
    parser.add_argument("--qualifications", type=Path,
                        help="Existing admission receipts and per-lane JSON stdio adapter argv")
    parser.add_argument("--output", type=Path,
                        help="New ignored result directory under target/basic-systems-acceptance/")
    args = parser.parse_args(argv)
    contract = read(HERE / "contract.json")
    if args.verify_draft:
        print(json.dumps(integrity(contract, args.pilot_root)))
        return 0
    if args.output is None:
        parser.error("--output is required unless --verify-draft is supplied")
    output = args.output.resolve()
    allowed = (REPO / "target/basic-systems-acceptance").resolve()
    assert output.is_relative_to(allowed) and output != allowed, "Output must be a child of target/basic-systems-acceptance"
    output.mkdir(parents=True, exist_ok=False)
    report = {"schema": "intersect.basic-systems.run.v1", "state": "running",
              "contract": pin(HERE / "contract.json"), "runner": pin(__file__),
              "results": [], "gate_closures": []}
    save(output / "preexecution.json", report)
    success = False
    before = None
    try:
        before = fixture_snapshot()
        save(output / "fixture-before.json", before)
        report["integrity"] = integrity(contract, args.pilot_root)
        qualifications = read(args.qualifications) if args.qualifications else {}
        for lane in contract["required_lanes"]:
            for case in contract["cases"]:
                row = {"case_id": case["id"], "lane": lane, "state": "blocked",
                       "imported": False, "consumed": False, "semantically_verified": False}
                try:
                    blocker = ("Original grammar and library bytes unverified; supply explicit --pilot-root"
                               if args.pilot_root is None
                               else admission_for(contract, qualifications, lane, case["language"]))
                    command = qualifications.get("adapters", {}).get(lane)
                    if blocker:
                        row["blocker"] = blocker
                    elif not isinstance(command, list) or not command or not all(isinstance(x, str) for x in command):
                        row["blocker"] = "Real language-owned workflow adapter for this lane is missing"
                    else:
                        row.update(execute_case(contract, case, lane, command,
                                                output / (lane + "-" + case["id"]), args.pilot_root))
                except BaseException:
                    row["state"] = "fail"
                    row["failure"] = traceback.format_exc()
                report["results"].append(row)
                save(output / "progress.json", report)
        # Candidate observations do not establish independently reviewed semantics.
        success = bool(report["results"]) and all(row["state"] == "pass" for row in report["results"])
    except BaseException:
        report["failure"] = traceback.format_exc()
    finally:
        try:
            after = fixture_snapshot()
            save(output / "fixture-after.json", after)
            assert before is not None and before == after, "Fixture source additions/deletions/changes detected"
            assert pin(__file__) == report["runner"], "Runner changed during execution"
            report["input_postguard"] = {"state": "pass", "fixture_files": len(after),
                                         "integrity": integrity(contract, args.pilot_root)}
        except BaseException:
            success = False
            report["input_postguard"] = {"state": "fail", "failure": traceback.format_exc()}
        report["state"] = "pass" if success else "incomplete_or_failed"
        report["counts"] = {state: sum(row["state"] == state for row in report["results"])
                            for state in ("pass", "candidate_pass_pending_review", "blocked", "fail")}
        save(output / "terminal.json", report)
        print(json.dumps({"state": report["state"], "counts": report["counts"],
                          "terminal": pin(output / "terminal.json"), "gate_closures": []}))
    return 0 if success else 2


if __name__ == "__main__":
    sys.exit(main())
