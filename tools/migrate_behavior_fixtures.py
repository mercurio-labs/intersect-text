"""Convert repository model-rule fixture data to canonical Rcore source syntax.

Original rule DTOs remain separate .rules.json files for qualification mutations;
they are not executable definition inputs. This script is an offline fixture tool.
Build intersect-web's rcore_behavior_parse example first; bundle fixtures use its
native migration and compilation APIs. --check verifies deterministic outputs.
"""
import json
import copy
import hashlib
import argparse
import subprocess
import sys
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent.parent


from behavior_fixture_migration import canonicalize_bundle_value, render


def migrate_compilation_fixture(compilation):
    """Use the native migration report to remap runtime selections exactly."""
    binary = ROOT / 'target/debug/examples' / ('rcore_behavior_parse.exe' if sys.platform == 'win32' else 'rcore_behavior_parse')
    def invoke(request):
        process = subprocess.run([str(binary)], input=json.dumps(request), text=True,
                                 encoding='utf-8', capture_output=True, check=True, cwd=ROOT)
        result = json.loads(process.stdout)
        if not result.get('ok'):
            raise ValueError(result)
        return result
    targets = [{'source': source['id'], 'uri': source['id']['uri'] + '.rcore'}
               for source in compilation['bundle']['sources'] if source['kind'] == 'ecore']
    migration = invoke({'method': 'ecore/migrateCompilation', 'compilation': compilation, 'targets': targets})
    report, converted = migration['report'], migration['compilation']
    invoke({'method': 'compile_bundle', 'compilation': converted})
    return converted, [issue for item in report['reports'] for issue in item['issues']]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    def save(path, text):
        data = text.encode('utf-8')
        if args.check:
            if path.read_bytes() != data:
                raise ValueError(f'Stale behavior fixture: {path}')
        else:
            path.write_bytes(data)
        return hashlib.sha256(data).hexdigest()

    converted = []
    for base in [ROOT / 'fixtures', ROOT / 'crates/intersect-model/assets/fixtures', ROOT / 'crates/intersect-text/assets/fixtures']:
        for snapshot in sorted(base.rglob('*.rules.json')):
            path = snapshot.with_name(snapshot.name.removesuffix('.rules.json') + '.rcore')
            text = snapshot.read_text(encoding='utf-8')
            rules = json.loads(text)
            namespace = None
            if any(rules.get(key) for key in ['derived', 'validations', 'initializers']):
                model = path.with_name(path.stem.removesuffix('-behavior') + '.ecore')
                namespace = ET.fromstring(model.read_text(encoding='utf-8')).attrib['nsURI']
            converted.append({'original': snapshot.relative_to(ROOT).as_posix(),
                              'original_sha256': hashlib.sha256(snapshot.read_bytes()).hexdigest(),
                              'converted': path.relative_to(ROOT).as_posix(),
                              'converted_sha256': save(path, render(rules, namespace))})
    for name in ['library/multi-resource', 'ecore-generics/language-bundle']:
        original = ROOT / 'fixtures' / (name + '.json')
        target = original.with_name(original.stem + '-canonical.json')
        migrated, issues = migrate_compilation_fixture(canonicalize_bundle_value(json.loads(original.read_text(encoding='utf-8'))))
        converted.append({'original': original.relative_to(ROOT).as_posix(),
                          'original_sha256': hashlib.sha256(original.read_bytes()).hexdigest(),
                          'converted': target.relative_to(ROOT).as_posix(),
                          'converted_sha256': save(target, json.dumps(migrated, indent=2, ensure_ascii=False) + '\n'),
                          'issues': issues})
    save(ROOT / 'fixtures/rcore/behavior-migration.json', json.dumps(converted, indent=2) + '\n')
    print(f'{"Verified" if args.check else "Converted"} {len(converted)} behavior fixtures.')


if __name__ == '__main__':
    main()
