"""Pure byte integrity; optional fresh generated-Xtext record and independent replay.

No Java is run unless --replay, --jdk and --maven-cache are supplied together.
The only comparison exception is the exact owned provider.genmodel output path.
"""
import argparse
import base64
import copy
import hashlib
import json
import ntpath
import os
import posixpath
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def disk(path):
    path = Path(path).resolve()
    if os.name == 'nt' and not str(path).startswith('\\\\?\\'):
        return Path('\\\\?\\' + str(path))
    return path


def raw(path):
    return disk(path).read_bytes()


def read(path):
    return json.loads(raw(path).decode('utf-8'))


def sha(path):
    return hashlib.sha256(raw(path)).hexdigest()


def files(root):
    base = disk(root)
    return {p.relative_to(base).as_posix(): p for p in base.rglob('*') if p.is_file()}


def save(path, value):
    disk(path).write_text(json.dumps(value, ensure_ascii=True, indent=2) + '\n', encoding='utf-8', newline='\n')


def pin(path):
    return {'bytes': len(raw(path)), 'sha256': sha(path)}


def local(root, name):
    # Every manifest path must be an owned portable relative path, including on Windows.
    assert isinstance(name, str) and name and '\\' not in name
    assert not posixpath.isabs(name) and not ntpath.isabs(name)
    assert all(p not in ('', '.', '..') for p in name.split('/'))
    return Path(root).joinpath(*name.split('/'))


def artifacts(root, expected=None):
    manifest = read(root / 'artifacts-sha256.json')
    actual = files(root)
    assert set(manifest) == set(actual) - {'artifacts-sha256.json'}, str(root)
    if expected is not None:
        assert len(manifest) == expected, (root, len(manifest), expected)
    for name, digest in manifest.items():
        assert sha(local(root, name)) == digest, (root, name)
    return len(manifest)


def normalized(rows, recorded_root, physical_root=None):
    result = copy.deepcopy(rows)
    # Historical observations retain their original Windows absolute paths. Validate
    # the exact recorded root lexically; never consult the historical machine path.
    pathmod = ntpath if ntpath.isabs(recorded_root) and not posixpath.isabs(recorded_root) else posixpath
    assert pathmod.isabs(recorded_root) and pathmod.normpath(recorded_root) == recorded_root
    for row in result:
        provider = row['provider']
        digest = provider['ecore_sha256']
        assert re.fullmatch('[0-9a-f]{64}', digest)
        relative = 'provider-' + digest[:12] + '/provider.genmodel'
        expected = pathmod.join(recorded_root, *relative.split('/'))
        assert provider['genmodel'] == expected, (row['id'], provider['genmodel'], expected)
        if physical_root is not None:
            assert disk(local(physical_root, relative)).is_file(), (row['id'], relative)
        provider['genmodel'] = 'OWNED_OUTPUT_ROOT/' + relative
    return result


def check_full(root, cases, recorded_root, expected_artifacts=None):
    count = artifacts(root, expected_artifacts)
    rows = read(root / 'observations.json')
    assert [r['id'] for r in rows] == [c['id'] for c in cases]
    commands = read(root / 'commands.json')
    runtime_commands = [c for c in commands if 'intersect.oracle.FullRuntime' in c['command']]
    expected_commands = []
    for case, row in zip(cases, rows):
        assert row['entry'] == case['entry'] and row['original_inputs'] == case['inputs']
        assert row['source_pins'] == {n: sha(p) for n, p in files(ROOT / 'inputs' / case['id']).items()}
        assert row['metadata']['exit_code'] == 0 and row['metadata']['observation'] is not None
        assert row['provider']['ecore_sha256'] == sha(ROOT / 'inputs' / case['id'] / 'imported.ecore')
        assert row['provider']['generation_exit'] == 0 and row['provider']['compile_exit'] == 0
        if row['generation'] is not None:
            assert row['generation']['source_unchanged'] is True
        if row['compilation'] and row['compilation']['exit_code'] == 0:
            assert [r['text'] for r in row['runtime']] == case['inputs']
        else:
            assert row['runtime'] == [], (case['id'], 'failed stage must remain unexecuted')
        for value in row['runtime']:
            expected_commands.append((row['provider']['package_class'], case['entry'], base64.b64encode(value['text'].encode('utf-8')).decode('ascii'), value))
    assert len(runtime_commands) == len(expected_commands)
    for command, (provider, entry, text, value) in zip(runtime_commands, expected_commands):
        assert command['command'][-3:] == [provider, entry, text]
        assert command['exit_code'] == value['exit_code']
        data = [json.loads(line) for line in raw(local(root, command['stdout'])).decode('utf-8').splitlines() if line.startswith('{')]
        assert value['observation'] == (data[0] if len(data) == 1 else None)
    toolchain = read(root / 'toolchain.json')
    assert '21.0.6+7' in toolchain['jdk']
    assert toolchain['jars'] == read(ROOT / 'inputs/stack.json')['jars']
    assert toolchain['input_pins_sha256'] == sha(ROOT / 'inputs/pins.json')
    return rows, normalized(rows, recorded_root, root), count


def check_metadata(root, cases, expected_artifacts=None):
    count = artifacts(root, expected_artifacts)
    rows = read(root / 'observations.json')
    assert [r['id'] for r in rows] == [c['id'] for c in cases]
    commands = [c for c in read(root / 'commands.json') if 'intersect.oracle.FirstRuleMetadataV2' in c['command']]
    assert len(commands) == len(cases)
    for row, command in zip(rows, commands):
        assert command['exit_code'] == 0
        data = [json.loads(line) for line in raw(local(root, command['stdout'])).decode('utf-8').splitlines() if line.startswith('{')]
        assert len(data) == 1 and row['observation'] == data[0]
    return rows, count


def totals(cases, rows):
    values = [v for r in rows for v in r['runtime']]
    accepted = sum(bool(v['observation'] and v['observation'].get('resource', {}).get('accepted')) for v in values)
    partials = sum(bool(v['observation'] and not v['observation'].get('resource', {}).get('accepted') and v['observation'].get('resource', {}).get('model') is not None) for v in values)
    return {'graphs': len(cases), 'planned_inputs': sum(len(c['inputs']) for c in cases),
            'generation_and_javac_successes': sum(bool(r['compilation'] and r['compilation']['exit_code'] == 0) for r in rows),
            'actual_runtime_inputs': len(values), 'accepted': accepted, 'rejected': len(values) - accepted,
            'rejected_partial_models': partials, 'unexecuted': sum(len(c['inputs']) for c in cases) - len(values)}


def integrity():
    provenance = read(ROOT / 'oracle-provenance.json')
    actual = {}
    for directory in ('inputs', 'oracle', 'oracle-producer'):
        actual.update({directory + '/' + n: p for n, p in files(ROOT / directory).items()})
    assert set(actual) == set(provenance['files']), 'Oracle owned-file coverage differs'
    for name, expected in provenance['files'].items():
        assert pin(local(ROOT, name)) == expected, name
    input_pins = read(ROOT / 'inputs/pins.json')
    assert set(input_pins) == set(files(ROOT / 'inputs')) - {'pins.json'}
    for name, digest in input_pins.items():
        assert sha(local(ROOT / 'inputs', name)) == digest, name
    cases = read(ROOT / 'inputs/cases.json')
    assert len(cases) == 20 and len({c['id'] for c in cases}) == 20
    for case in cases:
        for source in case['sources']:
            assert sha(local(ROOT / 'inputs' / case['id'], source['file'])) == source['source_id']['sha256']
    stack = read(ROOT / 'inputs/stack.json')
    assert len(stack['jars']) == 60
    # These are completely enumerated text transformations, not a second producer.
    for adapter in read(ROOT / 'oracle-producer/path-discovery-adaptations.json')['producers']:
        before = raw(local(ROOT, adapter['original'])).decode('utf-8')
        # Path.read_text used for the frozen derivation applies universal-newline translation.
        before = before.replace('\r\n', '\n')
        for replacement in adapter['replacements']:
            assert before.count(replacement['old']) == replacement['count'], adapter['portable']
            before = before.replace(replacement['old'], replacement['new'])
        assert before.encode('utf-8') == raw(local(ROOT, adapter['portable'])), adapter['portable']
    for name in ('BuildProvider.java', 'FullRuntime.java', 'FirstRuleMetadata.java', 'FirstRuleMetadataV2.java'):
        assert raw(ROOT / 'oracle-producer' / name) == raw(ROOT / 'oracle-producer/original' / name)
    full = {}
    normalized_full = {}
    counts = {}
    for key in ('record', 'replay'):
        full[key], normalized_full[key], counts[key] = check_full(ROOT / 'oracle' / key, cases, provenance['recorded_roots'][key], 775)
    assert normalized_full['record'] == normalized_full['replay']
    assert totals(cases, full['record']) == provenance['expected_totals_per_run']
    metadata = {}
    for key in ('record', 'replay'):
        metadata[key], counts['metadata-v2-' + key] = check_metadata(ROOT / 'oracle' / ('metadata-v2-' + key), cases, 121)
    assert metadata['record'] == metadata['replay']
    recovered = []
    for corrected, original in zip(metadata['record'], full['record']):
        if original['first_rule_metadata']['exit_code'] == 0:
            assert corrected['observation'] == original['first_rule_metadata']['observation']
        else:
            assert original['first_rule_metadata'] == {'exit_code': 1, 'observation': None}
            recovered.append(original['id'])
    assert recovered == ['enum_before_local_model', 'enum_library_model_child']
    for name, key, ids in provenance['old_four_lineage']:
        old = read(ROOT / 'oracle/lineage' / name)
        old = [r for r in old if r['id'] in ids]
        current = [copy.deepcopy(r) for r in full['record'] if r['id'] in ids]
        for row in current:
            row.pop('first_rule_metadata')
        assert normalized(old, provenance['recorded_roots'][key]) == normalized(current, provenance['recorded_roots']['record'])
    return provenance, cases, full, normalized_full, metadata, {'state': 'portable_oracle_integrity_pass', 'owned_files': len(actual), 'raw_artifact_pins': sum(counts.values()), 'artifact_counts': counts, 'totals_per_run': totals(cases, full['record']), 'original_metadata_failures_preserved': recovered, 'native_support_award': False, 'gate_closures': []}


def replay(args, cases, frozen_full, frozen_metadata):
    out = args.replay.resolve()
    assert out != ROOT and ROOT not in out.parents and not out.exists(), 'Replay output must be NEW and outside the corpus'
    out.mkdir(parents=True, exist_ok=False)
    commands = []
    receipt = {'state': 'fresh_portable_oracle_replay_started', 'commands': commands, 'native_support_award': False, 'gate_closures': []}
    save(out / 'receipt.json', receipt)
    def run(command, label):
        # Stream logs to files and preserve failed/partial outputs instead of repairing.
        with disk(out / (label + '.stdout.log')).open('wb') as stdout, disk(out / (label + '.stderr.log')).open('wb') as stderr:
            result = subprocess.run(list(map(str, command)), stdout=stdout, stderr=stderr)
        commands.append({'command': list(map(str, command)), 'exit_code': result.returncode, 'stdout': label + '.stdout.log', 'stderr': label + '.stderr.log'})
        save(out / 'receipt.json', receipt)
        assert result.returncode == 0, (label, result.returncode)
    try:
        observed = {}
        for key in ('record', 'replay'):
            run([sys.executable, ROOT / 'oracle-producer/produce.py', '--output', out / key, '--jdk', args.jdk, '--maven-cache', args.maven_cache], key)
            rows, observed[key], _ = check_full(out / key, cases, str(out / key))
            assert totals(cases, rows) == read(ROOT / 'oracle-provenance.json')['expected_totals_per_run']
            assert observed[key] == frozen_full['record'] == frozen_full['replay'], key
        assert observed['record'] == observed['replay']
        run([sys.executable, ROOT / 'oracle-producer/metadata_v2_pair.py', '--output', out / 'metadata-addendum', '--jdk', args.jdk, '--maven-cache', args.maven_cache], 'metadata-v2-pair')
        for key in ('record', 'replay'):
            rows, _ = check_metadata(out / 'metadata-addendum' / ('metadata-v2-' + key), cases)
            assert rows == frozen_metadata['record'] == frozen_metadata['replay']
        receipt['state'] = 'fresh_portable_oracle_record_and_replay_equal_to_both_frozen_runs'
        receipt['comparison_exception'] = 'Only row.provider.genmodel after exact absolute owned root + provider-<EcoreSHA12>/provider.genmodel validation; no other normalization.'
    except BaseException as failure:
        receipt['state'] = 'fresh_portable_oracle_replay_failed_preserved'
        receipt['failure'] = {'type': type(failure).__name__, 'message': str(failure)}
        save(out / 'receipt.json', receipt)
        raise
    receipt['new_artifact_pins'] = {n: pin(p) for n, p in files(out).items() if n != 'receipt.json'}
    save(out / 'receipt.json', receipt)
    return {'path': str(out / 'receipt.json'), **pin(out / 'receipt.json'), 'state': receipt['state']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--replay', type=Path)
    parser.add_argument('--jdk', type=Path)
    parser.add_argument('--maven-cache', type=Path)
    args = parser.parse_args()
    assert bool(args.replay) == bool(args.jdk) == bool(args.maven_cache), 'Supply --replay, --jdk and --maven-cache together'
    _, cases, _, frozen_full, metadata, result = integrity()
    if args.replay:
        result['fresh_replay'] = replay(args, cases, frozen_full, metadata)
    print(json.dumps(result, ensure_ascii=True, indent=2))


if __name__ == '__main__':
    main()
