"""Verify full-source grouped-negation evidence; optional fresh oracle replay."""
import argparse
import base64
import copy
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path, PurePosixPath, PureWindowsPath

HERE = Path(__file__).resolve().parent


def disk(path):
    path = Path(path).resolve()
    return Path('\\\\?\\' + str(path)) if os.name == 'nt' and not str(path).startswith('\\\\?\\') else path


def raw(path):
    return disk(path).read_bytes()


def load(path):
    return json.loads(raw(path).decode('utf-8'))


def pin(path):
    value = raw(path)
    return {'bytes': len(value), 'sha256': hashlib.sha256(value).hexdigest()}


def comparable(rows, output):
    rows = copy.deepcopy(rows)
    for row in rows:
        provider = row['provider']
        value = provider['genmodel']
        kind = PureWindowsPath if PureWindowsPath(value).drive else PurePosixPath
        path, root = kind(value), kind(str(output))
        digest = provider['ecore_sha256']
        if not path.is_absolute() or not root.is_absolute() or '..' in path.parts or '..' in root.parts:
            raise ValueError('GenModel is not an absolute owned output path')
        if len(digest) != 64 or any(c not in '0123456789abcdef' for c in digest):
            raise ValueError('Provider Ecore SHA differs')
        if path != root / ('provider-' + digest[:12]) / 'provider.genmodel':
            raise ValueError('GenModel is outside the exact owned provider output')
        provider['genmodel'] = '<owned-provider-output>/provider.genmodel'
    return rows


def validate(root, cases):
    manifest = load(root / 'artifacts-sha256.json')
    actual = set()
    for folder, _, names in os.walk(disk(root)):
        for name in names:
            path = Path(folder) / name
            relative = path.relative_to(disk(root)).as_posix()
            if relative != 'artifacts-sha256.json':
                actual.add(relative)
    if actual != set(manifest):
        raise ValueError('Raw artifact manifest is incomplete')
    for name, digest in manifest.items():
        if pin(root / name)['sha256'] != digest:
            raise ValueError('Raw artifact differs: ' + name)
    rows = load(root / 'observations.json')
    if [r['id'] for r in rows] != [c['id'] for c in cases]:
        raise ValueError('Source graph order differs')
    expected_commands = []
    positives = []
    counts = {'graphs': len(rows), 'planned': 0, 'runtime': 0, 'accepted': 0, 'rejected': 0, 'rejected_partial': 0}
    for row, case in zip(rows, cases):
        if row['entry'] != case['entry'] or row['original_inputs'] != case['inputs']:
            raise ValueError('Original runtime request differs')
        source = HERE / 'inputs' / case['id']
        if row['source_pins'] != {p.name: pin(p)['sha256'] for p in source.iterdir() if p.is_file()}:
            raise ValueError('Original source bytes differ')
        if row['generation'] != {'exit_code': 0, 'source_unchanged': True} or row['compilation'] != {'exit_code': 0}:
            raise ValueError('Recorded generation/javac stage differs')
        if row['provider']['generation_exit'] != 0 or row['provider']['compile_exit'] != 0:
            raise ValueError('Original Ecore-generated provider missing')
        if [r['text'] for r in row['runtime']] != case['inputs']:
            raise ValueError('Actual runtime inputs incomplete')
        counts['planned'] += len(case['inputs'])
        for run in row['runtime']:
            observation = run['observation']
            resource, parser = observation['resource'], observation['parser']
            if run['exit_code'] != 0 or run['timeout_seconds'] is not None:
                raise ValueError('Runtime process incomplete')
            if observation['text'] != run['text'] or not observation['lexer']['through_eof']:
                raise ValueError('Raw UTF16 token stream incomplete')
            accepted = resource['model'] is not None and not resource['errors'] and resource['failure'] is None
            if accepted != resource['accepted'] or parser['accepted'] != accepted:
                raise ValueError('Rejected partial mislabeled as accepted')
            counts['runtime'] += 1
            counts['accepted' if accepted else 'rejected'] += 1
            counts['rejected_partial'] += int(not accepted and resource['model'] is not None)
            if accepted:
                positives.append({'id': case['id'], 'text': run['text'], 'resource': resource})
            expected_commands.append((case['entry'], base64.b64encode(run['text'].encode('utf-8')).decode('ascii'), run['exit_code']))
    commands = [c for c in load(root / 'commands.json') if 'intersect.oracle.FullRuntime' in c['command']]
    if len(commands) != len(expected_commands):
        raise ValueError('Full source-generated runtime execution count differs')
    for command, (entry, text, code) in zip(commands, expected_commands):
        if command['command'][-2:] != [entry, text] or command['exit_code'] != code:
            raise ValueError('Actual runtime command differs')
    if counts != {'graphs': 5, 'planned': 40, 'runtime': 40, 'accepted': 15, 'rejected': 25, 'rejected_partial': 18}:
        raise ValueError('Full evidence denominator differs: ' + str(counts))
    if positives != load(HERE / 'positive-resources.json'):
        raise ValueError('Complete successful model projection differs')
    if '21.0.6+7' not in load(root / 'toolchain.json')['jdk']:
        raise ValueError('Pinned JDK differs')
    return rows, counts, len(manifest)


def frozen():
    provenance = load(HERE / 'provenance.json')
    for name, expected in provenance['files'].items():
        if pin(HERE / name) != expected:
            raise ValueError('Frozen file differs: ' + name)
    for name, digest in load(HERE / 'inputs/pins.json').items():
        if pin(HERE / 'inputs' / name)['sha256'] != digest:
            raise ValueError('Frozen source graph differs: ' + name)
    cases = load(HERE / 'inputs/cases.json')
    compared = []
    for key in ['record', 'replay']:
        root = HERE / 'oracle' / key
        rows, counts, artifacts = validate(root, cases)
        compared.append(comparable(rows, provenance['original_output_roots'][key]))
    if compared[0] != compared[1]:
        raise ValueError('Complete recorded and fresh replay observations differ')
    if rows != load(HERE / 'replay-observations.json') or load(HERE / 'oracle/record/observations.json') != load(HERE / 'observations.json'):
        raise ValueError('Consumer raw observations differ from original corpus')
    return cases, compared[0], counts, artifacts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--replay', type=Path)
    parser.add_argument('--jdk', type=Path)
    parser.add_argument('--maven-cache', type=Path)
    args = parser.parse_args()
    cases, expected, counts, artifacts = frozen()
    if args.replay:
        if args.jdk is None or args.maven_cache is None:
            parser.error('Fresh replay needs explicit --jdk and --maven-cache')
        output = args.replay.resolve()
        subprocess.run([sys.executable, '-B', str(HERE / 'produce.py'), '--output', str(output), '--jdk', str(args.jdk.resolve()), '--maven-cache', str(args.maven_cache.resolve())], check=True)
        rows, _, _ = validate(output, cases)
        if comparable(rows, output) != expected:
            raise ValueError('Fresh complete full-source observations differ')
        frozen()
    print(json.dumps({'state': 'verified', 'counts': counts, 'raw_artifact_pins_per_run': artifacts, 'native_award': False, 'gate_closures': []}))


if __name__ == '__main__':
    main()
