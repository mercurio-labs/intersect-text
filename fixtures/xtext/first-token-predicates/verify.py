"""Optional pinned Xtext/ANTLR predicate oracle. No downloads or production Java."""
import argparse
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import zipfile


def checked(path, digest):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError(f"SHA-256 mismatch: {path}")
    return data


def collect(args, fixture, provenance):
    artifacts = args.artifact_dir.resolve()
    artifacts.mkdir(parents=True, exist_ok=True)
    jars = []
    for item in provenance["jars"]:
        path = args.maven_cache / item["path"]
        checked(path, item["sha256"])
        jars.append(str(path.resolve()))
    for name, digest in provenance["harnesses"].items():
        checked(fixture / name, digest)
    lock = json.loads((fixture.parent.parent / "definition-conformance/upstream/artifacts.json").read_text())
    if provenance["jars"][0]["sha256"] not in {row.get("archive_sha256") for row in lock["files"]}:
        raise ValueError("Xtext binary differs from frozen upstream lock")
    generator = next(path for path in jars if "org.eclipse.xtext.xtext.generator-" in path)
    with zipfile.ZipFile(generator) as archive:
        for item in provenance["generator_classes"]:
            if hashlib.sha256(archive.read(item["path"])).hexdigest() != item["sha256"]:
                raise ValueError("Xtext generator class changed")
    suffix = ".exe" if os.name == "nt" else ""
    java = str(args.jdk / "bin" / ("java" + suffix))
    javac = str(args.jdk / "bin" / ("javac" + suffix))
    cp = os.pathsep.join(jars)
    def run(command):
        return subprocess.run(command, cwd=artifacts, check=True, capture_output=True, text=True, encoding="utf-8", timeout=60)
    run([javac, "-encoding", "UTF-8", "-cp", cp, "-d", str(artifacts), *[str(fixture / name) for name in provenance["harnesses"]]])
    launch = [java, "-cp", str(artifacts) + os.pathsep + cp]
    first = [json.loads(line) for line in run(launch + ["intersect.oracle.FirstSetOracle"]).stdout.splitlines()]
    if first != json.loads(checked(fixture / "observations.json", provenance["observations_sha256"])):
        raise ValueError("Independent first-set observations changed")
    emissions = dict(line.split("\t", 1) for line in run(launch + ["intersect.oracle.PredicateEmissionOracle"]).stdout.splitlines())
    names = []
    grammar_hashes = {}
    for kind, guard in emissions.items():
        bodies = {
            "Choice": f'({guard} \'a\' \'b\' {{branch+="g";}} | \'a\' \'c\' {{branch+="f";}})',
            "Optional": f'({guard} \'a\' \'b\' {{branch+="g";}})? \'a\' \'c\' {{branch+="f";}}',
            "Repeat": f'({guard} \'a\' \'b\' {{branch+="g";}})* \'a\' \'c\' {{branch+="f";}}',
        }
        # These recursive guards are independently authored ANTLR probes,
        # not claimed to be the complete output of Xtext language generation.
        recursive_guard = "('a')=>" if kind == "first" else "('a' inner 'b')=>"
        bodies["Recursive"] = f'({recursive_guard} \'a\' inner \'b\' {{branch+="g";}} | \'a\' inner \'c\' {{branch+="f";}})'
        for context, body in bodies.items():
            name = kind.title() + context
            names.append(name)
            grammar = f'grammar {name};\n@parser::members {{public String branch="";}}\nentry: {body} EOF;\n'
            if context == "Recursive":
                grammar += "inner: ('a' inner 'b')?;\n"
            grammar += r"WS: (' '|'\t'|'\r'|'\n')+ {$channel=HIDDEN;};" + "\n"
            path = artifacts / (name + ".g")
            path.write_text(grammar, encoding="utf-8", newline="\n")
            grammar_hashes[name] = hashlib.sha256(grammar.encode()).hexdigest()
            generated = run([java, "-cp", cp, "org.antlr.Tool", str(path)])
            (artifacts / (name + "-generation.log")).write_text(generated.stdout + generated.stderr, encoding="utf-8")
    run([javac, "-encoding", "UTF-8", "-cp", cp, "-d", str(artifacts), *[str(artifacts / (name + suffix + ".java")) for name in names for suffix in ["Lexer", "Parser"]]])
    result = run(launch + ["DecisionOracle", *names])
    decisions = []
    for line in result.stdout.splitlines():
        name, text, branch, errors = line.split("\t")
        decisions.append({"parser": name, "text": text, "branch": branch, "syntax_errors": int(errors), "accepted": int(errors) == 0})
    actual = {"emissions": emissions, "grammar_sha256": grammar_hashes, "decisions": decisions}
    (artifacts / "decision-observations.json").write_text(json.dumps(actual, indent=2) + "\n", encoding="utf-8")
    (artifacts / "decision-diagnostics.txt").write_text(result.stderr, encoding="utf-8")
    return actual



def collect_visibility(args, fixture, provenance):
    metadata = json.loads((fixture / "visibility-provenance.json").read_text())
    artifacts = args.artifact_dir.resolve()
    cp = os.pathsep.join(str((args.maven_cache / item["path"]).resolve()) for item in provenance["jars"])
    suffix = ".exe" if os.name == "nt" else ""
    java = str(args.jdk / "bin" / ("java" + suffix))
    javac = str(args.jdk / "bin" / ("javac" + suffix))
    generation = []
    for item in metadata["grammars"]:
        source = checked(fixture / item["file"], item["sha256"])
        path = artifacts / (item["name"] + ".g")
        path.write_bytes(source)
        result = subprocess.run([java, "-cp", cp, "org.antlr.Tool", str(path)], cwd=artifacts, capture_output=True, text=True, encoding="utf-8", timeout=60)
        (artifacts / (item["name"] + "-generation.log")).write_text(result.stdout + result.stderr, encoding="utf-8")
        generation.append({"name": item["name"], "exit": result.returncode,
                           "diagnostic_codes": re.findall(r"(warning|error)\((\d+)\)", result.stderr),
                           "hidden_by_actions": "hidden by actions" in result.stderr})
    subprocess.run([javac, "-encoding", "UTF-8", "-cp", cp, "-d", str(artifacts), *[str(artifacts / (name + suffix + ".java")) for name in metadata["runtime_parsers"] for suffix in ["Lexer", "Parser"]]], cwd=artifacts, check=True, capture_output=True, timeout=60)
    result = subprocess.run([java, "-cp", str(artifacts) + os.pathsep + cp, "DecisionOracle", *metadata["runtime_parsers"]], cwd=artifacts, check=True, capture_output=True, text=True, encoding="utf-8", timeout=60)
    decisions = []
    for line in result.stdout.splitlines():
        name, text, branch, errors = line.split("\t")
        decisions.append({"parser": name, "text": text, "branch": branch, "syntax_errors": int(errors), "accepted": int(errors) == 0})
    # Normalize tuples to JSON arrays for a platform-independent comparison.
    actual = json.loads(json.dumps({"generation": generation, "decisions": decisions}))
    (artifacts / "visibility-observations.json").write_text(json.dumps(actual, indent=2) + "\n", encoding="utf-8")
    (artifacts / "visibility-diagnostics.txt").write_text(result.stderr, encoding="utf-8")
    return actual


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--maven-cache", type=Path, default=Path.home() / ".m2/repository")
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    provenance = json.loads((fixture / "oracle-provenance.json").read_text())
    actual = collect(args, fixture, provenance)
    expected = json.loads(checked(fixture / "decision-observations.json", provenance["decisions_sha256"]))
    if actual != expected:
        raise ValueError("Independent predicate decision observations changed")
    print(f"Verified 9 first sets, 2 emitted guards and {len(actual['decisions'])} ANTLR decisions; native support not awarded.")
    visibility = collect_visibility(args, fixture, provenance)
    metadata = json.loads((fixture / "visibility-provenance.json").read_text())
    expected_visibility = json.loads(checked(fixture / "visibility-observations.json", metadata["observations_sha256"]))
    if visibility != expected_visibility:
        raise ValueError("Independent predicate visibility observations changed")
    print(f"Verified {len(visibility['generation'])} generator visibility controls and {len(visibility['decisions'])} guarded parser observations; native support not awarded.")


if __name__ == "__main__":
    main()
