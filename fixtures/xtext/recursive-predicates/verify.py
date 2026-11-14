"""Verify authored source identities; optionally execute the shared native host."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--native", type=Path)
args = parser.parse_args()
f = Path(__file__).resolve().parent
metadata = json.loads((f/"runtime-provenance.json").read_bytes())
for name, digest in metadata["files"].items():
    assert hashlib.sha256((f/name).read_bytes()).hexdigest() == digest, name
assert hashlib.sha256((f/"provenance.json").read_bytes()).hexdigest() == metadata["oracle_provenance_sha256"]
oracle = json.loads((f/"provenance.json").read_bytes())
raw = (f/"observations.json").read_bytes()
assert hashlib.sha256(raw).hexdigest() == oracle["observations_sha256"]
assert len(json.loads(raw)) == 22
print("Verified authored recursive source identities and unchanged 22 independent observations.")
if args.native:
    def invoke(request):
        return json.loads(subprocess.check_output([str(args.native)], input=json.dumps(request).encode("utf-8")))
    request = dict(method="compile", xtext_profile=metadata["profile"], **{key:(f/("model."+key)).read_bytes().decode("utf-8") for key in ["ecore","xtext","rcore_behavior"]})
    compiled = invoke(request)
    assert compiled["ok"], compiled
    assert not invoke({**request,"xtext_profile":"intersect.xtext.native.v8"})["ok"]
    rows = [row for row in json.loads(raw) if row["parser"] == "RecursiveEntryFirst"]
    for row in rows:
        result = invoke(dict(method="evaluate", package=compiled["package"], sources=[dict(uri="oracle.model",text=row["text"])]))
        assert (not result["diagnostics"]) == row["accepted"], row
        if not row["accepted"]:
            assert not result["model"]["elements"], row
    result = invoke(dict(method="evaluate", package=compiled["package"], sources=[dict(uri="example.model",text=(f/"example.model").read_text(encoding="utf-8"))]))
    assert not result["diagnostics"], result
    elements = result["model"]["elements"]
    assert len(elements) == metadata["expected"]["objects"]
    assert [item["class"] for item in elements] == metadata["expected"]["classes"]
    assert elements[0]["values"]["number"] == metadata["expected"]["number"]
    for parent, child in metadata["expected"]["containment_edges"]:
        assert elements[parent]["values"]["child"] == elements[child]["id"]
    print("Verified NativeV9 model execution, all 11 first-token acceptance cases and NativeV8 rejection. Full-syntax predicates and actual KerML semantics remain unqualified.")
