"""Replay the complete original ordinary Result and both budget counters."""
from pathlib import Path
import gzip
import hashlib
import json
import subprocess
import sys

root = Path(__file__).resolve().parent
manifest = json.loads((root / "provenance.json").read_bytes())
def blob(key):
    entry = manifest["entries"][key]
    raw = gzip.decompress((root / "objects" / (entry["sha256"] + ".gz")).read_bytes())
    assert len(raw) == entry["bytes"] and hashlib.sha256(raw).hexdigest() == entry["sha256"]
    return raw

artifact, output = map(Path, sys.argv[1:])
assert not output.exists()
output.mkdir(parents=True)
inputs = blob("before-ordinary/requests.tsv") + blob("before-ordinary-unit/requests.tsv")
expected = blob("before-ordinary/run-0.stdout") + blob("before-ordinary-unit/run-0.stdout")
assert len(inputs.splitlines()) == len(expected.splitlines()) == 162
for replay in range(2):
    result = subprocess.run([str(artifact.resolve())], input=inputs, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    (output / ("run-" + str(replay) + ".stdout")).write_bytes(result.stdout)
    (output / ("run-" + str(replay) + ".stderr")).write_bytes(result.stderr)
    assert result.returncode == 0, result.stderr
    assert result.stdout == expected, "Entire original Result and both counters"
(output / "receipt.json").write_text(json.dumps({"state":"pass","rows_per_run":162,"runs":2,"artifact_sha256":hashlib.sha256(artifact.read_bytes()).hexdigest(),"output_sha256":hashlib.sha256(expected).hexdigest(),"excluded_result_fields":[]},indent=2)+"\n",encoding="utf-8")
print("162whole ordinary Results and both numeric counters x2 PASS")
