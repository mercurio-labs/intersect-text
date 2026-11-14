"""Replay immutable full-source Xtext metamodel observations; Java is oracle-only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile


def checked(path, digest):
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError(f"SHA-256 mismatch: {path}")
    return raw


def validate_rows(cases, rows):
    ids = [case["id"] for case in cases]
    if len(ids) != len(set(ids)) or any(not re.fullmatch(r"[a-z][a-z0-9_]*", item) for item in ids):
        raise ValueError("Invalid or duplicate inference case identities")
    if [row["id"] for row in rows] != ids:
        raise ValueError("Inference case order/identity differs")
    for case, row in zip(cases, rows):
        errors = sum(item["severity"] == "ERROR" for item in row["diagnostics"])
        if row["error_count"] != errors or row["accepted"] != (errors == 0):
            raise ValueError(f"Incomplete diagnostic outcome: {case['id']}")
        if not row["imported_unchanged"]:
            raise ValueError(f"Imported package mutated: {case['id']}")
        for entry in row["generated_packages"]:
            package = entry["package"]
            if package is None:
                if row["accepted"]:
                    raise ValueError("Accepted inference has an unresolved generated package")
                continue
            names = set()
            for classifier in package["classifiers"]:
                name = classifier["name"]
                if name in names or classifier["identity"] != package["uri"] + "#//" + name:
                    raise ValueError("Generated classifier identity mismatch")
                names.add(name)
                if row["accepted"]:
                    for feature in classifier.get("features", []):
                        if feature["type"] is None or feature["type"].startswith("proxy:"):
                            raise ValueError("Accepted feature type is unresolved")
                observed = [classifier, *classifier.get("features", []), *classifier.get("literals", [])]
                for item in observed:
                    for origin in item["origins"]:
                        if not 0 <= origin["start"] <= origin["end"] <= len(case["source"]):
                            raise ValueError("Generated source origin exceeds authored source")


def frozen(fixture):
    provenance = json.loads((fixture / "provenance.json").read_text(encoding="utf-8"))
    for name, digest in provenance["files"].items():
        checked(fixture / name, digest)
    required = {"cases.json", "observations.json", "imported.ecore", "MetamodelOracle.java", "verify.py", "README.md"}
    if set(provenance["files"]) != required:
        raise ValueError("Inference provenance omits or adds a fixture component")
    cases = json.loads((fixture / "cases.json").read_text(encoding="utf-8"))
    rows = json.loads((fixture / "observations.json").read_text(encoding="utf-8"))
    validate_rows(cases, rows)
    if len(rows) != provenance["case_count"]:
        raise ValueError("Inference case denominator differs")
    if sum(row["accepted"] for row in rows) != provenance["accepted_cases"]:
        raise ValueError("Inference acceptance denominator differs")
    if sum(not row["accepted"] for row in rows) != provenance["rejected_cases"]:
        raise ValueError("Inference rejection denominator differs")
    return provenance, cases, rows


def replay(fixture, provenance, cases, jdk, maven_cache, output):
    output = output.resolve()
    if output == fixture or fixture in output.parents:
        raise ValueError("Oracle artifacts must stay outside the immutable fixture")
    jars = []
    for item in provenance["jars"]:
        path = maven_cache / item["path"]
        checked(path, item["sha256"])
        jars.append(str(path))
    archive = provenance["source_archive"]
    path = maven_cache / archive["path"]
    checked(path, archive["sha256"])
    lock = json.loads((fixture.parents[2] / "fixtures/definition-conformance/upstream/artifacts.json").read_text(encoding="utf-8"))
    locked = {item.get("archive_sha256") for item in lock["files"]}
    if archive["sha256"] not in locked or provenance["jars"][0]["sha256"] not in locked:
        raise ValueError("Xtext differs from the frozen upstream archive lock")
    with zipfile.ZipFile(path) as source:
        for item in provenance["source_members"]:
            if hashlib.sha256(source.read(item["path"])).hexdigest() != item["sha256"]:
                raise ValueError("Upstream inference source member differs")
    output.mkdir(parents=True, exist_ok=True)
    for case in cases:
        (output / (case["id"] + ".xtext")).write_text(case["source"], encoding="utf-8", newline="\n")
    (output / "imported.ecore").write_bytes((fixture / "imported.ecore").read_bytes())
    suffix = ".exe" if os.name == "nt" else ""
    classpath = os.pathsep.join(jars)
    subprocess.run([str(jdk / "bin" / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", classpath,
                    "-d", str(output), str(fixture / "MetamodelOracle.java")], check=True, timeout=60)
    result = subprocess.run([str(jdk / "bin" / ("java" + suffix)), "-cp", str(output) + os.pathsep + classpath,
                             "intersect.oracle.MetamodelOracle", str(output), *[case["id"] for case in cases]],
                            check=True, capture_output=True, text=True, encoding="utf-8", timeout=60)
    (output / "oracle-stderr.log").write_text(result.stderr, encoding="utf-8", newline="\n")
    rows = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    validate_rows(cases, rows)
    (output / "observations-replay.json").write_text(json.dumps(rows, ensure_ascii=True, indent=2) + "\n",
                                                   encoding="utf-8", newline="\n")
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--maven-cache", type=Path, default=Path.home() / ".m2/repository")
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    provenance, cases, expected = frozen(fixture)
    actual = replay(fixture, provenance, cases, args.jdk, args.maven_cache, args.artifact_dir)
    if actual != expected:
        differing = [case["id"] for case, left, right in zip(cases, actual, expected) if left != right]
        raise ValueError(f"Full Xtext inference observations differ: {differing}")
    print(f"Verified {len(actual)} full-source Xtext inference observations; native X03 support not awarded.")


if __name__ == "__main__":
    main()
