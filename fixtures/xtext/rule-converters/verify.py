"""Verify pinned rule-converter evidence; no native converter support is awarded."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import xml.etree.ElementTree as ET

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--upstream", type=Path, required=True)
p.add_argument("--mercurio", type=Path, required=True)
p.add_argument("--native", type=Path, required=True)
p.add_argument("--native-v8", action="store_true", help="Also verify the candidate NativeV8 execution")
a = p.parse_args()
f = Path(__file__).parent
e = json.loads((f / "provenance.json").read_bytes())
sources = {}
for name, row in e["sources"].items():
    raw = (a.upstream / row["path"]).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == row["sha256"], name
    sources[name] = raw
for row in e["rules"]:
    raw = sources[row["source"]][row["start"]:row["end"]]
    assert raw.decode("utf-8") == row["text"]
    assert hashlib.sha256(raw).hexdigest() == row["sha256"]
for row in e["mercurio"]["sources"]:
    assert hashlib.sha256((a.mercurio / row["path"]).read_bytes()).hexdigest() == row["sha256"]
cached = json.loads((a.mercurio / e["cached_oracle"]["source_path"]).read_bytes())
assert cached["provenance"] == e["cached_oracle"]["provenance"]
for key in ["bindings", "controls"]:
    assert [r for r in cached[key] if "Nonunique" in r["rule"] or "BooleanValue" in r["rule"]] == e["cached_oracle"][key]
root = ET.fromstring(sources["ecore"])
row = e["feature"]
assert root.get("nsURI") == row["namespace"]
cls = root.findall("eClassifiers")[row["classifier_index"]]
assert cls.get("name") == "Feature"
assert cls.findall("eStructuralFeatures")[row["feature_index"]].attrib == row["attributes"]
assert row["attributes"]["defaultValueLiteral"] == "true"
assert (f / "LICENSE").read_bytes() == (f.parent / "enum-showcase/LICENSE").read_bytes()
assert (f / "UPSTREAM-NOTICE.txt").read_bytes() == sources["converter"].split(b"package ", 1)[0]
request = {"method": "compile", "xtext_profile": "intersect.xtext.native.v7",
           **{n: (f / ("model." + n)).read_bytes().decode("utf-8") for n in ["ecore", "xtext", "rcore_behavior"]}}
expected = json.loads((f / "baseline.json").read_bytes())
def invoke(value):
    return json.loads(subprocess.check_output([str(a.native)], input=json.dumps(value).encode("utf-8")))
rejected = invoke(request)
assert rejected["ok"] is False
assert rejected["diagnostics"] == expected["rejection"]
fragment = e["rules"][0]["text"]
assert request["xtext"].count(fragment) == 1
without = request["xtext"].replace(fragment, "").replace("(isUnique=Nonunique)?", "")
for name, grammar, model in [
    ("absent_default", without, "feature F;"),
    ("boolean_assignment_control", without.replace("name=ID  ';'", "name=ID (isUnique?='nonunique')? ';'"), "feature F nonunique;")
]:
    compiled = invoke({**request, "xtext": grammar})
    assert compiled["ok"] is True, compiled
    result = invoke({"method": "evaluate", "package": compiled["package"], "sources": [{"uri": "nonunique.model", "text": model}]})
    assert result["diagnostics"] == []
    assert len(result["model"]["elements"]) == 1
    assert result["model"]["elements"][0]["values"]["isUnique"] is expected[name]
print("Verified pinned Nonunique/BooleanValue, converter/service/module, Feature.isUnique default and Mercurio dispatch identities; NativeV7 rejection and true-valued controls replay. NativeV7 converter execution remains unsupported.")

if a.native_v8:
    compiled = invoke({**request, "xtext_profile": "intersect.xtext.native.v8", "rcore_behavior": (f / "native-v8-behavior.rcore").read_bytes().decode("utf-8")})
    assert compiled["ok"] is True, compiled
    expected_false = next(r["value"] for r in e["cached_oracle"]["controls"] if r["context"] == "org.omg.kerml.xtext.KerML" and r["rule"].endswith("::Nonunique"))
    for text, value in [("feature F;", True), ((f / "example.kerml").read_bytes().decode("utf-8"), expected_false)]:
        result = invoke({"method": "evaluate", "package": compiled["package"], "sources": [{"uri": "example.kerml", "text": text}]})
        assert result["diagnostics"] == []
        assert len(result["model"]["elements"]) == 1
        assert result["model"]["elements"][0]["values"]["isUnique"] is value
    print("NativeV8 authored binding executes the pinned Nonunique fragment as false and preserves the true default; actual pinned metamodel semantics remain unqualified.")
