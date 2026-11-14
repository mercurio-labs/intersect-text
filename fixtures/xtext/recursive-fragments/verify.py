"""Verify exact argument slices and cached fragment-emitter provenance offline."""
import argparse
import hashlib
import json
from pathlib import Path

def digest(data):
    return hashlib.sha256(data).hexdigest()

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--upstream', type=Path)
a = p.parse_args()
root = Path(__file__).resolve().parent
manifest = json.loads((root/'provenance.json').read_text())
for name, expected in manifest['files'].items():
    assert digest((root/name).read_bytes()) == expected, name
oracle = json.loads((root/'oracle-provenance.json').read_text())
assert digest((root/'FragmentEmissionOracle.java').read_bytes()) == oracle['harness_sha256']
assert digest((root/'emission-observations.json').read_bytes()) == oracle['observations_sha256']
if a.upstream:
    pin = json.loads((root/'source-provenance.json').read_text())
    raw = (a.upstream/pin['source']).read_bytes()
    assert digest(raw) == pin['sha256']
    model = (root/'model.xtext').read_bytes()
    for row in pin['slices']:
        data = raw[row['start']:row['end']]
        assert digest(data) == row['sha256'], row['rule']
        assert model.count(data) == 1, row['rule']
print('Verified nine exact argument slices and fragment emitter identities; full KerML remains unqualified.')
