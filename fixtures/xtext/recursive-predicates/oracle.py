"""Independent recursive predicate entry observations; optional Java oracle only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def checked(path, digest):
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == digest, str(path)
    return raw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--maven-cache", type=Path, default=Path.home()/".m2/repository")
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    parent = fixture.parent/"first-token-predicates"
    metadata = json.loads((fixture/"provenance.json").read_bytes())
    provenance = json.loads(checked(parent/"oracle-provenance.json", metadata["parent_provenance_sha256"]))
    artifacts = args.artifact_dir.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    jars = []
    for item in provenance["jars"]:
        path = args.maven_cache/item["path"]
        checked(path, item["sha256"])
        jars.append(str(path.resolve()))
    cp = os.pathsep.join(jars)
    suffix = ".exe" if os.name == "nt" else ""
    java, javac = [str(args.jdk/"bin"/(name+suffix)) for name in ["java", "javac"]]
    def run(command):
        return subprocess.run(command, cwd=artifacts, capture_output=True, text=True, encoding="utf-8", check=True, timeout=60)
    names = []
    for name, digest in metadata["grammars"].items():
        target = artifacts/(name+".g")
        target.write_bytes(checked(fixture/(name+".g"), digest))
        result = run([java, "-cp", cp, "org.antlr.Tool", str(target)])
        assert not result.stderr.strip(), result.stderr
        names.append(name)
    harness = parent/"DecisionOracle.java"
    checked(harness, provenance["harnesses"][harness.name])
    generated = [str(artifacts/(name+kind+".java")) for name in names for kind in ["Lexer", "Parser"]]
    run([javac, "-encoding", "UTF-8", "-cp", cp, "-d", str(artifacts), str(harness), *generated])
    result = run([java, "-cp", str(artifacts)+os.pathsep+cp, "DecisionOracle", *names])
    (artifacts/"diagnostics.txt").write_text(result.stderr, encoding="utf-8")
    rows = []
    for line in result.stdout.splitlines():
        name, text, branch, errors = line.split("\t")
        rows.append(dict(parser=name, text=text, branch=branch, syntax_errors=int(errors), accepted=int(errors)==0))
    if args.verify:
        expected = json.loads(checked(fixture/"observations.json", metadata["observations_sha256"]))
        assert rows == expected
        print(f"Verified {len(rows)} recursive branch-entry observations; native execution remains unsupported.")
    else:
        print(json.dumps(rows, indent=2))


if __name__ == "__main__":
    main()
