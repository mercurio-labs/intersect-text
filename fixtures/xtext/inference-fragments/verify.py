"""Read-only replay of frozen full-source parser-fragment inference evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile


def checked(path, digest):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError(f"SHA-256 mismatch: {path}")
    return data


def structural(value):
    if isinstance(value, dict):
        return {key: structural(item) for key, item in value.items() if key != "origins"}
    if isinstance(value, list):
        return [structural(item) for item in value]
    return value


def validate_rows(cases, rows, imported):
    ids = [case["id"] for case in cases]
    if len(ids) != len(set(ids)) or any(not re.fullmatch(r"[a-z][a-z0-9_]*", item) for item in ids):
        raise ValueError("Invalid composition case identities")
    if [row["id"] for row in rows] != ids:
        raise ValueError("Composition case order/identity differs")
    for case, row in zip(cases, rows):
        source_ids = [source["id"] for source in case["sources"]]
        if len(source_ids) != len(set(source_ids)) or any(not re.fullmatch(r"[a-z][a-z0-9_]*", item) for item in source_ids):
            raise ValueError("Invalid source identities")
        if case["entry"] != source_ids[-1] or row["entry"] != case["entry"]:
            raise ValueError("Entry must be the last authored source")
        grammars = {source["grammar"]: source["source"] for source in case["sources"]}
        if len(grammars) != len(source_ids):
            raise ValueError("Duplicate authored grammar identity")
        seen = set()
        for source in case["sources"]:
            if not source["source"].isascii() or not source["source"].startswith("grammar " + source["grammar"]):
                raise ValueError("Authored grammar/source identity differs")
            if any(dependency not in seen for dependency in source["dependencies"]):
                raise ValueError("Missing exact earlier parent dependency")
            seen.add(source["id"])
        for stage in ("initial", "sources"):
            observed = row[stage]
            if [source["id"] for source in observed] != source_ids:
                raise ValueError("Source snapshot order/identity differs")
            for authored, source in zip(case["sources"], observed):
                if source["grammar"] != authored["grammar"]:
                    raise ValueError("Grammar snapshot identity differs")
                parents = [case["sources"][source_ids.index(item)]["grammar"] for item in authored["dependencies"]]
                if [parent["name"] for parent in source["parents"]] != parents:
                    raise ValueError("Resolved parent identity differs")
                errors = sum(item["severity"] == "ERROR" for item in source["diagnostics"])
                if source["error_count"] != errors or source["accepted"] != (errors == 0):
                    raise ValueError("Incomplete source diagnostic outcome")
                if source["accepted"] and any(parent["proxy"] for parent in source["parents"]):
                    raise ValueError("Accepted parent binding is unresolved")
                def span(item):
                    if not 0 <= item["start"] <= item["end"] <= len(authored["source"]):
                        raise ValueError("Rule/call source span exceeds exact source")
                for rule in source["rules"]:
                    span(rule)
                    if not isinstance(rule["fragment"], bool) or not isinstance(rule["wildcard"], bool) or (rule["wildcard"] and not rule["fragment"]):
                        raise ValueError("Invalid parser fragment flags")
                    if rule["wildcard"] and rule["type"] is not None:
                        raise ValueError("Wildcard fragment acquired a return classifier")
                    for call in rule["calls"]:
                        if call["synthetic"]:
                            if call["start"] is not None or call["end"] is not None:
                                raise ValueError("Synthetic call acquired an authored span")
                        else:
                            span(call)
                        target = call["target"]
                        if source["accepted"] and (call["proxy"] or target["grammar"] not in grammars):
                            raise ValueError("Accepted call has no exact declaring grammar")
                for declaration in source["metamodels"]:
                    package = declaration["package"]
                    if not declaration["generated"] or package is None:
                        continue
                    names = set()
                    for classifier in package["classifiers"]:
                        if classifier["name"] in names or classifier["identity"] != package["uri"] + "#//" + classifier["name"]:
                            raise ValueError("Generated classifier identity differs")
                        names.add(classifier["name"])
                        if source["accepted"]:
                            for feature in classifier.get("features", []):
                                if not feature["type"] or feature["type"].startswith("proxy:"):
                                    raise ValueError("Accepted generated feature has unresolved type")
                        for item in [classifier, *classifier.get("features", []), *classifier.get("literals", [])]:
                            for origin in item["origins"]:
                                text = grammars.get(origin["grammar"])
                                if text is None or not 0 <= origin["start"] <= origin["end"] <= len(text):
                                    raise ValueError("Generated origin exceeds declaring source")
        if row["accepted"] != row["sources"][-1]["accepted"] or row["all_sources_accepted"] != all(item["accepted"] for item in row["sources"]):
            raise ValueError("Entry/package acceptance differs from source outcomes")
        if row["imported_unchanged"] != (row["imported_package"] == imported):
            raise ValueError("Original imported metadata classification differs")
        if structural(row["imported_package"]) != structural(imported):
            raise ValueError("Original imported structure mutated")
        def origins(value):
            if isinstance(value, dict):
                for origin in value.get("origins", []):
                    text = grammars.get(origin["grammar"])
                    if text is None or not 0 <= origin["start"] <= origin["end"] <= len(text):
                        raise ValueError("Imported origin exceeds declaring source")
                for item in value.values():
                    origins(item)
            elif isinstance(value, list):
                for item in value:
                    origins(item)
        origins(row["imported_package"])
        if [item["id"] for item in row["parent_snapshots"]] != source_ids[:-1]:
            raise ValueError("Parent snapshot denominator differs")
        for before, after, observation in zip(row["initial"], row["sources"], row["parent_snapshots"]):
            metadata = before["metamodels"] == after["metamodels"]
            unchanged = structural(before["metamodels"]) == structural(after["metamodels"])
            if observation["metadata_unchanged"] != metadata or observation["structure_unchanged"] != unchanged or not unchanged:
                raise ValueError("Parent mutation classification differs")
        # Calls retain declaring-source identities, even for same-name overrides.
        rules = {(source["grammar"], rule["name"]): rule for source in row["sources"] for rule in source["rules"]}
        for source in row["sources"]:
            if not source["accepted"]:
                continue
            for rule in source["rules"]:
                for call in rule["calls"]:
                    target = call["target"]
                    declared = rules.get((target["grammar"], target["name"]))
                    if declared is None or any(target[key] != declared[key] for key in ("name", "grammar", "kind", "type", "fragment", "wildcard")):
                        raise ValueError("Resolved rule target differs from exact declaration")


def frozen(fixture):
    provenance = json.loads((fixture / "provenance.json").read_text(encoding="utf-8"))
    if set(provenance["files"]) != {"cases.json", "observations.json", "FragmentOracle.java", "verify.py", "README.md"}:
        raise ValueError("Fragment provenance omits a component")
    for name, digest in provenance["files"].items():
        checked(fixture / name, digest)
    for name, digest in provenance["dependencies"].items():
        checked(fixture.parents[2] / name, digest)
    cases = json.loads((fixture / "cases.json").read_text(encoding="utf-8"))
    rows = json.loads((fixture / "observations.json").read_text(encoding="utf-8"))
    imported = json.loads((fixture.parent / "metamodel-inference/observations.json").read_text(encoding="utf-8"))[0]["imported_package"]
    validate_rows(cases, rows, imported)
    actual = (len(rows), sum(r["all_sources_accepted"] for r in rows), sum(r["accepted"] for r in rows), sum(len(r["sources"]) for r in rows))
    expected = (provenance["case_count"], provenance["complete_graph_accepted"], provenance["entry_only_accepted"], provenance["source_count"])
    if actual != expected or provenance["complete_graph_rejected"] != len(rows) - actual[1]:
        raise ValueError("Fragment evidence denominator differs")
    parents = [parent for row in rows for parent in row["parent_snapshots"]]
    changes = (len(parents), sum(not p["metadata_unchanged"] for p in parents), sum(not r["imported_unchanged"] for r in rows))
    if changes != (provenance["parent_snapshots"], provenance["parent_metadata_changes"], provenance["imported_metadata_changes"]):
        raise ValueError("Parent/imported metadata denominator differs")
    synthetic = sum(c["synthetic"] for r in rows for source in r["sources"] for rule in source["rules"] for c in rule["calls"])
    if synthetic != provenance["synthetic_calls"]:
        raise ValueError("Synthetic call denominator differs")
    return provenance, cases, rows


def replay(fixture, provenance, cases, jdk, cache, output):
    output = output.resolve()
    protected = fixture.parents[1]
    if output == protected or protected in output.parents:
        raise ValueError("Artifacts must stay outside immutable fixtures")
    jars = []
    for item in provenance["jars"]:
        path = cache / item["path"]
        checked(path, item["sha256"])
        jars.append(str(path))
    archive = cache / provenance["source_archive"]["path"]
    checked(archive, provenance["source_archive"]["sha256"])
    lock_path = fixture.parents[2] / "fixtures/definition-conformance/upstream/artifacts.json"
    lock = json.loads(checked(lock_path, provenance["upstream_lock_sha256"]))
    locked = {item.get("archive_sha256") for item in lock["files"]}
    if provenance["source_archive"]["sha256"] not in locked or provenance["jars"][0]["sha256"] not in locked:
        raise ValueError("Xtext differs from frozen upstream lock")
    with zipfile.ZipFile(archive) as source:
        for item in provenance["source_members"]:
            if hashlib.sha256(source.read(item["path"])).hexdigest() != item["sha256"]:
                raise ValueError("Upstream source member differs")
    output.mkdir(parents=True, exist_ok=True)
    for case in cases:
        directory = output / case["id"]
        directory.mkdir(exist_ok=True)
        for source in case["sources"]:
            (directory / (source["id"] + ".xtext")).write_text(source["source"], encoding="utf-8", newline="\n")
        (directory / "order.txt").write_text("\n".join(s["id"] for s in case["sources"]) + "\n", encoding="utf-8", newline="\n")
        (directory / "entry.txt").write_text(case["entry"], encoding="utf-8")
    base = fixture.parent / "metamodel-inference"
    (output / "imported.ecore").write_bytes((base / "imported.ecore").read_bytes())
    suffix = ".exe" if os.name == "nt" else ""
    classpath = os.pathsep.join(jars)
    subprocess.run([str(jdk / "bin" / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", classpath,
                    "-d", str(output), str(base / "MetamodelOracle.java"), str(fixture.parent / "inference-composition/CompositionOracle.java"), str(fixture / "FragmentOracle.java")], check=True, timeout=60)
    result = subprocess.run([str(jdk / "bin" / ("java" + suffix)), "-cp", str(output) + os.pathsep + classpath,
                             "intersect.oracle.FragmentOracle", str(output), *[case["id"] for case in cases]],
                            check=True, capture_output=True, text=True, encoding="utf-8", timeout=120)
    (output / "oracle-stderr.log").write_text(result.stderr, encoding="utf-8", newline="\n")
    rows = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    imported = json.loads((base / "observations.json").read_text(encoding="utf-8"))[0]["imported_package"]
    validate_rows(cases, rows, imported)
    (output / "observations-replay.json").write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8", newline="\n")
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--maven-cache", type=Path, default=Path.home() / ".m2/repository")
    parser.add_argument("--artifact-dir", type=Path, required=True)
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    provenance, cases, expected = frozen(fixture)
    actual = replay(fixture, provenance, cases, args.jdk, args.maven_cache, args.artifact_dir)
    if actual != expected:
        changed = [case["id"] for case, left, right in zip(cases, actual, expected) if left != right]
        raise ValueError(f"Complete fragment observations differ: {changed}")
    print(f"Verified {len(actual)} complete source-loaded Xtext fragment observations; native fragment inference support not awarded.")


if __name__ == "__main__":
    main()
