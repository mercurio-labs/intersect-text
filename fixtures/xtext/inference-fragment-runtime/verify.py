"""Read-only generated Xtext fragment parser/model construction replay."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess


def checked(path, digest):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError(f"SHA-256 mismatch: {path}")
    return data


def inference_oracle(fixture):
    path = fixture.parent / "inference-fragments/verify.py"
    spec = importlib.util.spec_from_file_location("source_fragment_oracle", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def validate(fixture, cases, observations):
    metadata = observations["metadata"]
    imported = json.loads((fixture.parent / "metamodel-inference/observations.json").read_text(encoding="utf-8"))[0]["imported_package"]
    inference_oracle(fixture).validate_rows(cases, metadata, imported)
    original = {c["id"]: c for c in json.loads((fixture.parent / "inference-fragments/cases.json").read_text(encoding="utf-8"))}
    for case in cases:
        if "origin_case" in case and case["sources"] != original[case["origin_case"]]["sources"]:
            raise ValueError("Original source graph was changed")
    if observations["version"] != 1 or [r["id"] for r in observations["generation"]] != [c["id"] for c in cases]:
        raise ValueError("Construction observation identity/version differs")
    requests = [(c["id"], r["entry"], r["text"]) for c in cases for r in c["runs"]]
    if [(r["case"], r["entry"], r["text"]) for r in observations["runs"]] != requests:
        raise ValueError("Runtime request identity/order differs")
    for case, meta, generated in zip(cases, metadata, observations["generation"]):
        expected_sources = {s["id"]: hashlib.sha256(s["source"].encode()).hexdigest() for s in case["sources"]}
        if generated["source_sha256"] != expected_sources or not generated["source_unchanged"]:
            raise ValueError("Exact generation source identity differs")
        if not meta["all_sources_accepted"]:
            if generated["stage"] != "inference_rejected" or case["runs"]:
                raise ValueError("Invalid complete graph advanced to generation")
        elif generated["stage"] == "generation_failed":
            if not generated["error_keys"] or case["runs"] or generated["generation_exit"] == 0:
                raise ValueError("Generation failure has no independent evidence")
        elif generated["stage"] == "compiled":
            if generated["generation_exit"] != 0 or generated["compilation_exit"] != 0 or generated["error_keys"]:
                raise ValueError("Compiled graph has an unsuccessful prerequisite")
            if set(generated["parser_entries"]) != {s["grammar"] for s in case["sources"]}:
                raise ValueError("Generated parser inventory omits a source language")
            for entries in generated["parser_entries"].values():
                if not entries or entries != sorted(set(entries)):
                    raise ValueError("Generated parser entry inventory differs")
            if not generated["ecore_sha256"]:
                raise ValueError("Generated model artifacts are missing")
        else:
            raise ValueError("Unknown generation stage")
    for row in observations["runs"]:
        if row["accepted"]:
            if row["exception"] is not None or row["errors"] or not row["model"]:
                raise ValueError("Accepted construction lacks a complete model")
            if row["entry"] == "resource" and row["resource_contents"] != 1:
                raise ValueError("Accepted resource root count differs")
        elif not row.get("exception") and not row.get("errors"):
            raise ValueError("Rejected construction has no failure evidence")
        for model in row.get("model") or []:
            if not model["class"] or not isinstance(model["values"], dict):
                raise ValueError("Model classifier/feature data missing")
            for value in model["values"].values():
                if isinstance(value, dict) and "object" in value:
                    index = value["object"]
                    if not 0 <= index < len(row["model"]) or row["model"][index]["class"] != value["class"]:
                        raise ValueError("Model reference ordinal/class differs")


def frozen(fixture):
    p = json.loads((fixture / "provenance.json").read_text(encoding="utf-8"))
    required = {"cases.json", "observations.json", "RuntimeOracle.java", "Generate.mwe2.template", "verify.py", "README.md"}
    if set(p["files"]) != required:
        raise ValueError("Construction fixture component denominator differs")
    for name, digest in p["files"].items():
        checked(fixture / name, digest)
    for name, digest in p["dependencies"].items():
        checked(fixture.parents[2] / name, digest)
    cases = json.loads((fixture / "cases.json").read_text(encoding="utf-8"))
    rows = json.loads((fixture / "observations.json").read_text(encoding="utf-8"))
    validate(fixture, cases, rows)
    actual = {
        "graphs": len(cases), "sources": sum(len(c["sources"]) for c in cases),
        "inference_accepted": sum(r["all_sources_accepted"] for r in rows["metadata"]),
        "compiled_graphs": sum(r["stage"] == "compiled" for r in rows["generation"]),
        "generation_failures": sum(r["stage"] == "generation_failed" for r in rows["generation"]),
        "runtime_requests": len(rows["runs"]), "runtime_accepted": sum(r["accepted"] for r in rows["runs"]),
    }
    if p["counts"] != actual:
        raise ValueError("Construction evidence denominator differs")
    return p, cases, rows


def run(command, output, cwd=None):
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="strict", timeout=180)
    output.write_text(result.stdout + result.stderr, encoding="utf-8", newline="\n")
    return result


def replay(fixture, p, cases, jdk, cache, output):
    output = output.resolve()
    protected = fixture.parents[1]
    if output == protected or protected in output.parents:
        raise ValueError("Generated artifacts must stay outside fixtures")
    output.mkdir(parents=True, exist_ok=False)
    jars = []
    for item in p["jars"]:
        path = cache / item["path"]
        checked(path, item["sha256"])
        jars.append(str(path))
    cp = os.pathsep.join(jars)
    suffix = ".exe" if os.name == "nt" else ""
    java, javac = [str(jdk / "bin" / (name + suffix)) for name in ("java", "javac")]
    old = inference_oracle(fixture)
    metadata = old.replay(fixture.parent / "inference-fragments", p, cases, jdk, cache, output / "metadata")
    template = (fixture / "Generate.mwe2.template").read_text(encoding="utf-8")
    observations = {"version": 1, "metadata": metadata, "generation": [], "runs": []}
    for case, meta in zip(cases, metadata):
        project = output / case["id"]
        project.mkdir()
        source_hashes = {s["id"]: hashlib.sha256(s["source"].encode()).hexdigest() for s in case["sources"]}
        generated = {"id": case["id"], "source_sha256": source_hashes, "source_unchanged": True}
        observations["generation"].append(generated)
        if not meta["all_sources_accepted"]:
            generated.update(stage="inference_rejected", generation_exit=None, compilation_exit=None, error_keys=[])
            continue
        for source in case["sources"]:
            if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*(\.[A-Za-z_][A-Za-z_0-9]*)+", source["grammar"]):
                raise ValueError("Invalid Java/grammar source identity")
            path = project / "src" / (source["grammar"].replace(".", "/") + ".xtext")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source["source"], encoding="utf-8", newline="\n")
        languages = "".join('    language = StandardLanguage {\n      name = "' + s["grammar"] + '"\n      fileExtensions = "fragmenttest"\n      serializer = { generateStub = false }\n      validator = { generateDeprecationValidation = false }\n    }\n' for s in case["sources"])
        workflow = project / "src/Generate.mwe2"
        workflow.write_text(template.replace("@LANGUAGES@", languages), encoding="utf-8", newline="\n")
        result = run([java, "-cp", str(project / "src") + os.pathsep + cp, "org.eclipse.emf.mwe2.launch.runtime.Mwe2Launcher", workflow.as_uri(), "-p", "rootPath=" + project.as_posix()], project / "generation.log", project)
        for source in case["sources"]:
            path = project / "src" / (source["grammar"].replace(".", "/") + ".xtext")
            checked(path, source_hashes[source["id"]])
        generated["generation_exit"] = result.returncode
        if result.returncode:
            names = sorted(set(re.findall(r"Cannot determine the package name of '([^']+)'", result.stdout + result.stderr)))
            if not names:
                raise ValueError(f"Unexpected generator failure: {case['id']}; see generation.log")
            generated.update(stage="generation_failed", compilation_exit=None, error_keys=["TypeReference.package_name:" + n for n in names])
            continue
        classes = project / "classes"
        classes.mkdir()
        files = sorted(project.glob("src/**/*.java")) + sorted(project.glob("src-gen/**/*.java"))
        files += [fixture.parent / "metamodel-inference/MetamodelOracle.java", fixture / "RuntimeOracle.java"]
        args = ["-encoding", "UTF-8", "--release", "17", "-cp", cp, "-d", str(classes), *map(str, files)]
        argfile = project / "javac.args"
        argfile.write_text("\n".join('"' + a.replace("\\", "/").replace('"', '\\"') + '"' for a in args) + "\n", encoding="utf-8", newline="\n")
        result = run([javac, "@" + str(argfile)], project / "compilation.log")
        result.check_returncode()
        entries = {}
        for source in case["sources"]:
            package, language = source["grammar"].rsplit(".", 1)
            parser = project / "src-gen" / package.replace(".", "/") / "parser/antlr/internal" / ("Internal" + language + "Parser.java")
            entries[source["grammar"]] = sorted(set(re.findall(r"public final [^\n]+ entryRule([A-Za-z_0-9]+)\(", parser.read_text(encoding="utf-8"))))
        generated.update(stage="compiled", compilation_exit=0, error_keys=[], parser_entries=entries,
                         ecore_sha256={path.relative_to(project).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(project.glob("model/generated/*.ecore"))})
        for index, request in enumerate(case["runs"]):
            result = run([java, "-cp", os.pathsep.join([str(classes), str(project / "src-gen"), str(project / "src"), cp]), "intersect.oracle.RuntimeOracle", case["sources"][-1]["grammar"], request["entry"], request["text"]], project / f"runtime-{index}.log")
            result.check_returncode()
            rows = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
            if len(rows) != 1:
                raise ValueError("Runtime probe did not return one complete observation")
            observations["runs"].append({"case": case["id"], **rows[0]})
    validate(fixture, cases, observations)
    (output / "observations-replay.json").write_text(json.dumps(observations, indent=2) + "\n", encoding="utf-8", newline="\n")
    return observations


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--artifact-dir", type=Path, required=True)
    parser.add_argument("--maven-cache", type=Path, default=Path.home() / ".m2/repository")
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    p, cases, expected = frozen(fixture)
    if replay(fixture, p, cases, args.jdk, args.maven_cache, args.artifact_dir) != expected:
        raise ValueError("Complete generated parser/model observations differ")
    print(f"Verified {len(cases)} generated-parser graphs and {len(expected['runs'])} runtime requests; native support is separate.")


if __name__ == "__main__":
    main()
