"""Optional hash-pinned Xtext inference oracle; no network or production Java."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile


def checked(path, expected):
    value = path.read_bytes()
    if hashlib.sha256(value).hexdigest() != expected:
        raise ValueError(f"SHA-256 mismatch: {path}")
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--maven-cache", type=Path, default=Path.home() / ".m2" / "repository")
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    provenance = json.loads((fixture / "provenance.json").read_text())
    source = fixture / "InferenceOracle.java"
    checked(source, provenance["oracle_sha256"])
    expected = json.loads(checked(fixture / "observations.json", provenance["observations_sha256"]))
    jars = []
    for item in provenance["jars"]:
        jar = args.maven_cache / item["path"]
        checked(jar, item["sha256"])
        jars.append(str(jar))
    archive = provenance["source_archive"]
    source_jar = args.maven_cache / archive["path"]
    checked(source_jar, archive["sha256"])
    lock = json.loads((fixture.parent.parent / "definition-conformance/upstream/artifacts.json").read_text())
    locked_hashes = {item.get("archive_sha256") for item in lock["files"]}
    for digest in (archive["sha256"], provenance["jars"][0]["sha256"]):
        if digest not in locked_hashes:
            raise ValueError("Xtext archive differs from frozen upstream lock")
    with zipfile.ZipFile(source_jar) as zipped:
        for item in provenance["source_members"]:
            if hashlib.sha256(zipped.read(item["path"])).hexdigest() != item["sha256"]:
                raise ValueError("Upstream inference source member differs")
    args.artifact_dir.mkdir(parents=True, exist_ok=True)
    suffix = ".exe" if os.name == "nt" else ""
    classpath = os.pathsep.join(jars)
    subprocess.run([str(args.jdk / "bin" / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", classpath, "-d", str(args.artifact_dir), str(source)], check=True, timeout=60)
    result = subprocess.run([str(args.jdk / "bin" / ("java" + suffix)), "-cp", str(args.artifact_dir) + os.pathsep + classpath, "intersect.oracle.InferenceOracle"], check=True, capture_output=True, timeout=60)
    actual = [json.loads(line) for line in result.stdout.decode("ascii").splitlines()]
    if actual != expected:
        raise ValueError(f"Inference observations differ: {actual!r}")
    print(f"Verified {len(actual)} independent Xtext inference observations; native support not awarded.")


if __name__ == "__main__":
    main()
