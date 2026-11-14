"""Optional independent Xtext token oracle; no downloads or production dependency."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def verify_hash(path, expected):
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual != expected:
        raise ValueError(f"SHA-256 mismatch for {path}: {actual}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--maven-cache", type=Path, default=Path.home() / ".m2" / "repository")
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    provenance = json.loads((fixture / "provenance.json").read_text(encoding="utf-8"))
    lock = json.loads((fixture.parent.parent / "definition-conformance" / "upstream" / "artifacts.json").read_text(encoding="utf-8"))
    for kind in ("binary", "source"):
        matches = [item for item in lock["files"] if item.get("archive_url") == provenance[f"{kind}_archive_url"]]
        if not matches or any(item["archive_sha256"] != provenance[f"{kind}_archive_sha256"] for item in matches):
            raise ValueError(f"{kind} archive identity differs from frozen upstream lock")
    jar = args.maven_cache / "org/eclipse/xtext/org.eclipse.xtext/2.38.0/org.eclipse.xtext-2.38.0.jar"
    source = fixture / "DatatypeTokenOracle.java"
    observations = fixture / "observations.json"
    verify_hash(jar, provenance["binary_archive_sha256"])
    verify_hash(source, provenance["oracle_sha256"])
    verify_hash(observations, provenance["observations_sha256"])
    args.artifact_dir.mkdir(parents=True, exist_ok=True)
    suffix = ".exe" if os.name == "nt" else ""
    subprocess.run([str(args.jdk / "bin" / ("javac" + suffix)), "-encoding", "UTF-8", "-cp", str(jar), "-d", str(args.artifact_dir), str(source)], check=True, timeout=60)
    result = subprocess.run([str(args.jdk / "bin" / ("java" + suffix)), "-cp", os.pathsep.join([str(args.artifact_dir), str(jar)]), "DatatypeTokenOracle"], check=True, capture_output=True, timeout=60)
    actual = [json.loads(line) for line in result.stdout.decode("ascii").splitlines()]
    expected = json.loads(observations.read_text(encoding="utf-8"))
    if actual != expected:
        raise ValueError(f"Oracle observations differ: {actual!r}")
    print(f"Verified {len(actual)} pinned Xtext datatype-token observations; native support not awarded.")


if __name__ == "__main__":
    main()
