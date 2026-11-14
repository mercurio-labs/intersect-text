"""Pure immutable oracle integrity; no Java or production execution by default."""
import argparse
import hashlib
import json
import os
from pathlib import Path
ROOT = Path(__file__).resolve().parent.parent

def disk(p):
    p = Path(p).resolve()
    return p if os.name != 'nt' or str(p).startswith('\\\\?\\') else Path('\\\\?\\' + str(p))

def pin(p):
    b = disk(p).read_bytes()
    return {'bytes': len(b), 'sha256': hashlib.sha256(b).hexdigest()}

def read(p):
    return json.loads(disk(p).read_text(encoding='utf-8'))

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.parse_args()
    manifest_path = ROOT / 'portable-manifest.json'
    if disk(manifest_path).exists():
        manifest = read(manifest_path)
        actual = set()
        for folder, _, names in os.walk(disk(ROOT)):
            for name in names:
                p = Path(folder) / name
                relative = p.relative_to(disk(ROOT)).as_posix()
                if relative != 'portable-manifest.json':
                    actual.add(relative)
        assert actual == set(manifest['files']), 'Unexpected or missing portable corpus files'
        for relative, expected in manifest['files'].items():
            assert pin(ROOT / relative) == expected, relative
    provenance = read(ROOT / 'oracle-provenance.json')
    for relative, expected in provenance['files'].items():
        assert pin(ROOT / relative) == {k: expected[k] for k in ['bytes', 'sha256']}, relative
    frozen = read(ROOT / 'oracle-producer/metadata-uri-binding/frozen-addendum.json')
    for value in [frozen['helper'], frozen['original_helper'], frozen['original_metadata']]:
        assert pin(ROOT / value['path']) == {k: value[k] for k in ['bytes', 'sha256']}, value['path']
    seen = set()
    for value in frozen['bindings']:
        assert value['id'] not in seen, value['id']
        seen.add(value['id'])
        assert pin(ROOT / value['physical_provider']) == value['physical_provider_pin']
        assert pin(ROOT / value['mapping']) == value['mapping_pin']
        lines = disk(ROOT / value['mapping']).read_text(encoding='utf-8').splitlines()
        assert lines == [value['original_logical_provider_uri'] + '\t' + value['physical_provider_pin']['sha256'] + '\timported.ecore']
    assert len(seen) == 12
    for label in ['original8', 'typed8', 'fragment2', 'return12']:
        inputs = ROOT / 'inputs' / label
        pins = read(inputs / 'inputs/pins.json')
        for name, expected in pins.items():
            assert pin(inputs / 'inputs' / name)['sha256'] == expected, (label, name)
    print(json.dumps({'state': 'all_byteexact_raw_oracle_and_binding_pins_valid',
        'original_files': len(provenance['files']), 'metadata_bindings': len(seen),
        'normalization': None, 'java_executed': False, 'native_support_award': False}))

if __name__ == '__main__':
    main()
