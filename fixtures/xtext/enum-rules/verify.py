"""Replay exact pinned enum inventory and the NativeV6 rejection baseline."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET


def main():
    args_parser = argparse.ArgumentParser(description=__doc__)
    args_parser.add_argument("--upstream", type=Path, required=True)
    args_parser.add_argument("--native", type=Path, required=True)
    args_parser.add_argument("--oracle-jdk", type=Path)
    args_parser.add_argument("--oracle-artifact-dir", type=Path)
    args_parser.add_argument("--maven-cache", type=Path, default=Path.home()/".m2/repository")
    args = args_parser.parse_args()
    fixture = Path(__file__).parent
    evidence = json.loads((fixture / "provenance.json").read_text(encoding="utf-8"))
    sources = {}
    for kind in ["grammar", "ecore"]:
        entry = evidence[kind]
        raw = (args.upstream / entry["path"]).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == entry["sha256"]
        sources[kind] = raw
    for entry in evidence["rules"]:
        raw = sources["grammar"][entry["start"]:entry["end"]]
        assert raw.decode("utf-8") == entry["text"]
        assert hashlib.sha256(raw).hexdigest() == entry["sha256"]
    root = ET.fromstring(sources["ecore"])
    assert root.get("nsURI") == evidence["ecore"]["namespace"]
    for entry in evidence["enums"]:
        node = root.findall("eClassifiers")[entry["classifier_index"]]
        assert node.attrib == entry["attributes"]
        assert [x.attrib for x in node.findall("eLiterals")] == entry["literals"]
    def invoke(request):
        result = subprocess.run([str(args.native)], input=json.dumps(request).encode(), capture_output=True, check=True, timeout=60)
        return json.loads(result.stdout)
    request = {"method": "compile", "xtext_profile": "intersect.xtext.native.v6"}
    for name in ["ecore", "xtext", "rcore_behavior"]:
        request[name] = (fixture / ("model." + name)).read_bytes().decode("utf-8")
    expected = json.loads((fixture / "baseline.json").read_text(encoding="utf-8"))
    rejected = invoke(request)
    assert rejected["ok"] is False
    assert rejected["diagnostics"] == expected["rejected"]
    without = "\n".join(line for line in request["xtext"].splitlines() if not line.startswith("enum ")) + "\n"
    terminal = invoke({**request, "xtext": without.replace("visibility=VisibilityIndicator", "visibility=ID")})
    assert terminal["ok"] is False
    assert terminal["diagnostics"] == expected["terminal_assignment_rejected"]
    compiled = invoke({**request, "xtext": without.replace(" visibility=VisibilityIndicator", "")})
    assert compiled["ok"] is True, compiled
    model = invoke({"method": "evaluate", "package": compiled["package"], "sources": [{"uri": "enum.model", "text": "visibility;"}]})
    assert model["diagnostics"] == []
    assert len(model["model"]["elements"]) == 1
    assert model["model"]["elements"][0]["values"]["visibility"] == expected["control_default_value"]
    if args.oracle_jdk:
        if args.oracle_artifact_dir is None:
            args_parser.error("--oracle-artifact-dir is required with --oracle-jdk")
        verify_emission(args, fixture)
    print("Verified three pinned enum rules, two exact enum inventories, NativeV6 rejection and identity-preserving default control. NativeV6 enum-rule execution remains unsupported.")


def verify_emission(args, fixture):
    evidence=json.loads((fixture/"oracle-provenance.json").read_text(encoding="utf-8"))
    def checked(path, digest):
        raw=path.read_bytes()
        assert hashlib.sha256(raw).hexdigest()==digest, path
        return raw
    jars=[]
    for entry in evidence["jars"]:
        path=args.maven_cache/entry["path"]
        checked(path,entry["sha256"])
        jars.append(str(path.resolve()))
    harness=fixture/"EnumEmissionOracle.java"
    checked(harness,evidence["harness_sha256"])
    expected=json.loads(checked(fixture/"emission-observations.json",evidence["observations_sha256"]))
    out=args.oracle_artifact_dir.resolve();out.mkdir(parents=True,exist_ok=True)
    suffix=".exe" if os.name=="nt" else ""
    cp=os.pathsep.join(jars)
    subprocess.run([str(args.oracle_jdk/"bin"/("javac"+suffix)),"-encoding","UTF-8","-cp",cp,"-d",str(out),str(harness.resolve())],check=True,capture_output=True,timeout=60)
    result=subprocess.run([str(args.oracle_jdk/"bin"/("java"+suffix)),"-cp",str(out)+os.pathsep+cp,"intersect.oracle.EnumEmissionOracle"],check=True,capture_output=True,text=True,encoding="utf-8",timeout=60)
    observed=[]
    for line in result.stdout.splitlines():
        spelling,name,index,emission=line.split("\t",3)
        observed.append(dict(spelling=spelling,literal=name,literal_index=int(index),emission=emission))
    assert observed==expected
    print("Verified five independent actual Xtext enum emission observations. No production Java dependency.")


if __name__ == "__main__":
    main()
