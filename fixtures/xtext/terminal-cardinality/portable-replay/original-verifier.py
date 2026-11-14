"""Independent full-source oracle record/replay; preserve all failed stages."""
import base64
import copy
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path, PurePosixPath, PureWindowsPath

HERE = Path(__file__).resolve().parent
INPUT = HERE / 'frozen-inputs-v1'
OUTPUT = HERE.parent / 'trm-oracle-v1'


def disk(p):
    p = Path(p).resolve()
    return p if os.name != 'nt' or str(p).startswith('\\\\?\\') else Path('\\\\?\\' + str(p))


def raw(p):
    return disk(p).read_bytes()


def pin(p):
    value = raw(p)
    return {'bytes': len(value), 'sha256': hashlib.sha256(value).hexdigest()}


def load(p):
    return json.loads(raw(p).decode('utf-8'))


def save(p, value):
    disk(p).write_text(json.dumps(value, indent=2, ensure_ascii=True) + '\n', encoding='utf-8', newline='\n')


def comparable(rows, root):
    rows = copy.deepcopy(rows)
    for row in rows:
        provider = row['provider']
        if provider is None:
            continue
        value = provider['genmodel']
        kind = PureWindowsPath if PureWindowsPath(value).drive else PurePosixPath
        path, parent = kind(value), kind(str(root))
        digest = provider['ecore_sha256']
        assert len(digest) == 64 and all(x in '0123456789abcdef' for x in digest)
        assert path.is_absolute() and parent.is_absolute() and '..' not in path.parts and '..' not in parent.parts
        assert path == parent / ('provider-' + digest[:12]) / 'provider.genmodel'
        provider['genmodel'] = '<exact-owned-provider-output>/provider.genmodel'
    return rows


def check(root, cases):
    manifest = load(root / 'artifacts-sha256.json')
    actual = set()
    for folder, _, names in os.walk(disk(root)):
        for name in names:
            relative = (Path(folder) / name).relative_to(disk(root)).as_posix()
            if relative != 'artifacts-sha256.json':
                actual.add(relative)
    assert set(manifest) == actual, 'Complete raw output manifest differs'
    for relative, digest in manifest.items():
        assert pin(root / relative)['sha256'] == digest, relative
    rows = load(root / 'observations.json')
    assert [r['id'] for r in rows] == [c['id'] for c in cases]
    runtime = []
    for case, row in zip(cases, rows):
        assert row['entry'] == case['entry'] and row['original_inputs'] == case['inputs']
        source = INPUT / 'inputs' / case['id']
        assert row['source_pins'] == {p.name: pin(p)['sha256'] for p in source.iterdir() if p.is_file()}
        for authored in case['sources']:
            assert pin(source / authored['file'])['sha256'] == authored['source_id']['sha256']
        if row['generation']:
            assert row['generation']['source_unchanged']
        if row['compilation'] and row['compilation']['exit_code'] == 0:
            assert [r['text'] for r in row['runtime']] == case['inputs']
        else:
            assert row['runtime'] == []
        for run in row['runtime']:
            runtime.append((case['entry'], base64.b64encode(run['text'].encode('utf-8')).decode('ascii'), run['exit_code']))
    commands = [c for c in load(root / 'commands.json') if 'intersect.oracle.FullRuntime' in c['command']]
    assert len(commands) == len(runtime)
    for command, (entry, text, code) in zip(commands, runtime):
        assert command['command'][-2:] == [entry, text] and command['exit_code'] == code
    assert '21.0.6+7' in load(root / 'toolchain.json')['jdk']
    return rows, comparable(rows, root), {'artifact_pins': len(manifest), 'manifest': pin(root / 'artifacts-sha256.json'), 'observations': pin(root / 'observations.json'), 'toolchain': pin(root / 'toolchain.json'), 'commands': pin(root / 'commands.json')}


def classify(root, rows):
    commands = load(root / 'commands.json')
    generation = [c for c in commands if 'org.eclipse.emf.mwe2.launch.runtime.Mwe2Launcher' in c['command']]
    result = []
    for row, command in zip(rows, generation):
        metadata = row['metadata']['observation'] if row['metadata'] else None
        output = raw(root / command['stdout']) + raw(root / command['stderr'])
        runtime = row['runtime']
        observed = [r for r in runtime if r['exit_code'] == 0 and r['timeout_seconds'] is None and r['observation'] is not None]
        accepted = sum(bool(r['observation']['resource']['accepted']) for r in observed)
        partial = sum(not r['observation']['resource']['accepted'] and r['observation']['resource']['model'] is not None for r in observed)
        result.append({'id': row['id'], 'metadata_exit': row['metadata']['exit_code'] if row['metadata'] else None, 'syntax_errors': sum(len(x['errors']) for x in metadata['syntax']) if metadata else None, 'link_errors': sum(len(x['errors']) for x in metadata['linking']) if metadata else None, 'validation_ERRORs': sum(sum(i['severity'] == 'ERROR' for i in x['issues']) for x in metadata['validation']) if metadata else None, 'generation': row['generation'], 'antlr_error_lines': [line for line in output.decode('utf-8').splitlines() if 'error(' in line], 'antlr_warning_lines': [line for line in output.decode('utf-8').splitlines() if 'warning(' in line], 'javac': row['compilation'], 'planned_inputs': len(row['original_inputs']), 'runtime_attempts': len(runtime), 'complete_runtime_observations': len(observed), 'accepted': accepted, 'rejected': len(observed) - accepted, 'rejected_partials': partial, 'unexecuted_inputs': row['original_inputs'] if not runtime else [], 'incomplete_runtime_inputs': [r['text'] for r in runtime if r not in observed]})
    return result


def main():
    OUTPUT.mkdir(exist_ok=False)
    freeze = load(INPUT / 'preexecution.json')
    for relative, expected in freeze['files'].items():
        assert pin(INPUT / relative) == expected, relative
    cases = load(INPUT / 'inputs/cases.json')
    native = load(HERE / 'native-before-v62-v1/receipt.json')
    assert native['counts']['graphs'] == len(cases)
    receipt = {'state': 'frozen_full_source_record_and_fresh_replay_started', 'input_freeze': pin(INPUT / 'preexecution.json'), 'producer': pin(INPUT / 'produce.py'), 'verifier': pin(Path(__file__)), 'native_before': pin(HERE / 'native-before-v62-v1/receipt.json'), 'commands': [], 'native_award': False, 'gate_closures': []}
    save(OUTPUT / 'preexecution.json', receipt)
    try:
        compared = []
        runs = {}
        for key in ['record', 'replay']:
            command = [sys.executable, '-B', str(INPUT / 'produce.py'), '--output', str(OUTPUT / key), '--jdk', 'D:/dev/jdks/jdk21', '--maven-cache', 'C:/Users/MarkPetrotta/.m2/repository']
            print('START_' + key.upper(), flush=True)
            with (OUTPUT / (key + '.stdout.log')).open('wb') as stdout, (OUTPUT / (key + '.stderr.log')).open('wb') as stderr:
                finished = subprocess.run(command, stdout=stdout, stderr=stderr)
            receipt['commands'].append({'command': command, 'exit_code': finished.returncode})
            save(OUTPUT / 'receipt.json', receipt)
            assert finished.returncode == 0, (key, finished.returncode)
            rows, normalized, info = check(OUTPUT / key, cases)
            compared.append(normalized)
            runs[key] = info
            print('COMPLETE_' + key.upper(), flush=True)
        assert compared[0] == compared[1], 'Complete record/replay observations differ; no unapproved normalization'
        for relative, expected in freeze['files'].items():
            assert pin(INPUT / relative) == expected, relative
        receipt.update(state='full_source_record_and_independent_fresh_replay_complete_observations_equal', runs=runs, graphs=len(cases), planned_inputs=sum(len(c['inputs']) for c in cases), classification=classify(OUTPUT / 'record', rows), comparison_exception='Only exact absolute owned provider-<EcoreSHA12>/provider.genmodel output path')
    except BaseException as failure:
        receipt.update(state='failed_or_incomplete_original_stage_preserved', failure={'class': type(failure).__name__, 'message': str(failure)})
        save(OUTPUT / 'receipt.json', receipt)
        raise
    save(OUTPUT / 'receipt.json', receipt)
    print(json.dumps({'receipt': str(OUTPUT / 'receipt.json'), **pin(OUTPUT / 'receipt.json')}), flush=True)


if __name__ == '__main__':
    main()
