"""Read-only replay of complete pinned Xtext source-encoding observations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

def checked(path, digest):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError("SHA-256 mismatch: " + str(path))
    return data

def frozen(folder):
    p = json.loads((folder / "source-encoding-provenance.json").read_text(encoding="utf-8"))
    for name, digest in p["artifacts"].items():
        checked(folder / name, digest)
    cases = json.loads((folder / "source-encoding-cases.json").read_text(encoding="utf-8"))
    rows = json.loads((folder / "source-encoding-observations.json").read_text(encoding="utf-8"))
    if [r["id"] for r in rows] != [c["id"] for c in cases] or [r["all_sources_accepted"] for r in rows] != [True, False, True]:
        raise ValueError("Encoding observation identity/outcome differs")
    return p, cases, rows

def replay(folder, jdk, cache, output):
    p, cases, expected = frozen(folder)
    root = folder.parents[2]
    legacy = json.loads(checked(root / p["oracle_provenance"], p["oracle_provenance_sha256"]))
    jars = []
    for item in legacy["jars"]:
        path = cache / item["path"]
        checked(path, item["sha256"])
        jars.append(str(path))
    for path, digest in p["source_files"].items():
        checked(root / path, digest)
    output = output.resolve()
    protected = root / "fixtures"
    if output == protected or protected in output.parents or output.exists():
        raise ValueError("Oracle output must be a fresh directory outside frozen fixtures")
    output.mkdir(parents=True)
    for case in cases:
        directory = output / case["id"]
        directory.mkdir()
        for source in case["sources"]:
            path = directory / (source["id"] + ".xtext")
            path.write_text(source["source"], encoding="utf-8", newline="\n")
            if path.read_bytes() != source["source"].encode("utf-8"):
                raise ValueError("Exact UTF-8 source changed")
        (directory / "order.txt").write_text("\n".join(s["id"] for s in case["sources"]) + "\n", encoding="utf-8", newline="\n")
        (directory / "entry.txt").write_text(case["entry"], encoding="utf-8", newline="\n")
    imported = root / "fixtures/xtext/metamodel-inference/imported.ecore"
    (output / "imported.ecore").write_bytes(imported.read_bytes())
    sources = [root / path for path in p["source_files"] if path.endswith(".java")]
    suffix = ".exe" if os.name == "nt" else ""
    classpath = os.pathsep.join(jars)
    subprocess.run([str(jdk / "bin" / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", classpath, "-d", str(output), *map(str, sources)], check=True, timeout=60)
    result = subprocess.run([str(jdk / "bin" / ("java" + suffix)), *p["java_options"], "-cp", str(output) + os.pathsep + classpath, "intersect.oracle.FragmentOracle", str(output), *[c["id"] for c in cases]], check=True, capture_output=True, encoding="utf-8", timeout=120)
    actual = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    (output / "observations-replay.json").write_text(json.dumps(actual, indent=2) + "\n", encoding="utf-8", newline="\n")
    (output / "oracle-stderr.log").write_text(result.stderr, encoding="utf-8", newline="\n")
    if actual != expected:
        raise ValueError("Complete source-encoding observations differ")
    for case in cases:
        for source in case["sources"]:
            if (output / case["id"] / (source["id"] + ".xtext")).read_bytes() != source["source"].encode("utf-8"):
                raise ValueError("Oracle changed authored bytes")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--maven-cache", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    args = parser.parse_args()
    replay(Path(__file__).resolve().parent, args.jdk, args.maven_cache, args.artifact_dir)
    print("Three complete pinned source-encoding observations match; plain/Unicode accept and leading BOM rejects. No native diagnostic-span qualification awarded.")
