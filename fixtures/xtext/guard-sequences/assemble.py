"""Canonicalize independent captures without replacing source or runtime evidence."""
import hashlib
import json
from pathlib import Path
import re


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8"))


def ordinary(cases, metadata, captures):
    rows = {}
    for capture in map(Path, captures):
        for row in read(capture / "capture-ledger.json")["results"]:
            assert row["id"] not in rows, "Duplicate captured case"
            rows[row["id"]] = (capture, row)
    assert set(rows) == {case["id"] for case in cases}
    assert [row["id"] for row in metadata] == [case["id"] for case in cases]
    generation, runs, probes, unexecuted = [], [], [], []
    for case, meta in zip(cases, metadata):
        capture, row = rows[case["id"]]
        assert row["planned_runs"] == case["runs"]
        if not meta["all_sources_accepted"]:
            assert row["state"] == "metadata_rejected" and not row["runs"]
            generation.append({"id": case["id"], "stage": "metadata_rejected"})
            unexecuted.append({"case": case["id"], "reason": "metadata_rejected", "requests": case["runs"]})
            continue
        project = capture / "languages" / case["id"]
        for source in case["sources"]:
            path = project / "src" / (source["grammar"].replace(".", "/") + ".xtext")
            assert hashlib.sha256(path.read_bytes()).hexdigest() == source["sha256"]
        assert row["provider"]["ecore_hashes_match_exact_imported_inputs"]
        assert not list((project / "model/generated").glob("*.ecore"))
        item = {"id": case["id"], "provider": row["provider"], "generated_ecore_files": []}
        if row["state"] == "capture_failed_unqualified":
            assert case["id"] == "root_false" and row["generation"]["exit_code"] != 0 and not row["runs"]
            log = (project / "generation.log").read_text(encoding="utf-8")
            diagnostics = sorted({line.strip() for line in log.splitlines()
                                  if line.strip().startswith("java.lang.NullPointerException:")
                                  or line.strip() == "java.lang.IllegalArgumentException: Unhandled parameter types: [null]"})
            assert any('because "ele" is null' in line for line in diagnostics)
            assert "java.lang.IllegalArgumentException: Unhandled parameter types: [null]" in diagnostics
            item.update(stage="generator_failed", exit_code=row["generation"]["exit_code"], diagnostics=diagnostics)
            unexecuted.append({"case": case["id"], "reason": "generator_failed", "requests": case["runs"]})
        else:
            assert row["state"] == "captured"
            assert row["generation"] == {"exit_code": 0, "compilation_exit": 0, "source_unchanged": True, "generated_ecore_files": []}
            entries = {}
            for source in case["sources"]:
                package, language = source["grammar"].rsplit(".", 1)
                parser = project / "src-gen" / package.replace(".", "/") / "parser/antlr/internal" / ("Internal" + language + "Parser.java")
                entries[source["grammar"]] = sorted(set(re.findall(r"public final [^\n]+ entryRule([A-Za-z_0-9]+)\(", parser.read_text(encoding="utf-8"))))
            item.update(stage="compiled", generation=row["generation"], parser_entries=entries)
            assert [(r["entry"], r["text"]) for r in row["runs"]] == [(r["entry"], r["text"]) for r in case["runs"]]
            runs.extend({"case": case["id"], **r} for r in row["runs"])
            probes.append({"case": case["id"], **row["unregistered_context_probe"]})
        generation.append(item)
    return {"metadata": metadata, "generation": generation, "runs": runs,
            "unregistered_context": probes, "unexecuted_requests": unexecuted}
