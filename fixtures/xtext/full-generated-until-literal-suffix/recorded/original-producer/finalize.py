"""Finalize the separately frozen literal Until suffix record and fresh replay."""
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
def load(path): return json.loads(path.read_text(encoding='utf-8'))
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def pin(path): return {'path': path.relative_to(ROOT).as_posix(), 'bytes': path.stat().st_size, 'sha256': sha(path)}
def save(path, value):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        stream.write(json.dumps(value, ensure_ascii=True, indent=2) + '\n')

assert sha(ROOT / 'inputs/pins.json') == 'd0bbfbc1d97d7ad8f46789edef6d1bd5ff4db10387a2afd874bc75974ce4b680'
assert sha(ROOT / 'produce.py') == '5d6ab9ab0b5eca36443cdce3b7a003c3989a0b7e0053728f3eb7266376b26f79'
assert sha(ROOT / 'produce-location-adapter-v1.py') == '7abf6d96b35d2c85b684ea0056f8f57df58c5192cbf732c7282c009152c0135d'
for name, digest in load(ROOT / 'inputs/pins.json').items():
    assert sha(ROOT / 'inputs' / name) == digest
case = load(ROOT / 'inputs/cases.json')[0]
ecore_sha = sha(ROOT / 'inputs' / case['id'] / 'imported.ecore')
assert ecore_sha == '726b2116570fad065036c20eca40be7c4bda665317089fc2cd1f1cab5ea3f734'
observations, runs = [], {}
for name in ['record', 'replay']:
    output = ROOT / name
    pins = load(output / 'artifacts-sha256.json')
    actual = {path.relative_to(output).as_posix(): sha(path) for path in output.rglob('*') if path.is_file() and path.name != 'artifacts-sha256.json'}
    assert actual == pins
    rows = load(output / 'observations.json')
    assert len(rows) == 1 and rows[0]['id'] == case['id']
    row = rows[0]
    assert row['original_inputs'] == case['inputs']
    assert row['provider']['ecore_sha256'] == ecore_sha
    provider_path = Path(row['provider']['genmodel'])
    assert provider_path.is_absolute()
    assert provider_path == output / ('provider-' + ecore_sha[:12]) / 'provider.genmodel'
    assert provider_path.is_file()
    assert row['generation']['source_unchanged']
    for source in case['sources']:
        original = ROOT / 'inputs' / case['id'] / source['file']
        generated = output / case['id'] / 'src' / (source['grammar'].replace('.', '/') + '.xtext')
        assert original.read_bytes() == generated.read_bytes()
    commands = load(output / 'commands.json')
    runtime_commands = [command for command in commands if 'intersect.oracle.FullRuntime' in command['command']]
    assert len(runtime_commands) == len(row['runtime'])
    for run in row['runtime']:
        assert run['observation']['text'] == run['text']
        assert run['observation']['lexer']['through_eof']
        resource = run['observation']['resource']
        if resource['accepted']:
            assert not resource['errors'] and resource['model'] is not None and resource['failure'] is None
        elif resource['model'] is not None:
            assert resource['errors'] or resource['failure']
    normalized = copy.deepcopy(rows)
    normalized[0]['provider']['genmodel'] = '<owned-provider-output>/provider.genmodel'
    observations.append(normalized)
    runs[name] = {'observations': pin(output / 'observations.json'), 'artifact_manifest': pin(output / 'artifacts-sha256.json'), 'toolchain': pin(output / 'toolchain.json'), 'raw_artifact_pins_verified': len(pins), 'all_commands': len(commands), 'fresh_runtime_processes': len(runtime_commands)}
equal = observations[0] == observations[1]
row = load(ROOT / 'record/observations.json')[0]
runtime = [run['observation'] for run in row['runtime']]
raw_view = [{'text': run['text'], 'exit_code': run['exit_code'], 'observation': run['observation']} for run in row['runtime']]
save(ROOT / 'observed-results.json', {'version': 1, 'scope': 'Exact observed source/program/input results only, no general Until theorem or native award.', 'raw_runtime': raw_view})
receipt = {
    'version': 1, 'state': 'full_generated_record_and_fresh_replay_complete' if equal else 'replay_mismatch_preserved',
    'scope': 'Separate new literal Until plus local suffix companion; no edits to the frozen eighteen-graph corpus or any original receipt.',
    'preexecution': pin(ROOT / 'preexecution.json'),
    'initial_failure_and_location_only_adapter': pin(ROOT / 'producer-location-failure-and-adapter.json'),
    'inputs': pin(ROOT / 'inputs/pins.json'),
    'producers': [pin(ROOT / n) for n in ['produce.py', 'produce-location-adapter-v1.py', 'BuildProvider.java', 'FullRuntime.java', 'finalize.py']],
    'source': case['sources'][0],
    'original_ecore_sha256': ecore_sha,
    'syntax': row['metadata']['observation']['syntax'],
    'linking': row['metadata']['observation']['linking'],
    'validation': row['metadata']['observation']['validation'],
    'generation': row['generation'], 'compilation': row['compilation'],
    'planned_inputs': len(case['inputs']), 'actual_runtime_inputs_per_run': len(runtime),
    'actual_runtime_processes_both_runs': sum(run['fresh_runtime_processes'] for run in runs.values()),
    'accepted_per_run': sum(value['resource']['accepted'] for value in runtime),
    'rejected_per_run': sum(not value['resource']['accepted'] for value in runtime),
    'rejected_partial_models_per_run': sum(not value['resource']['accepted'] and value['resource']['model'] is not None for value in runtime),
    'complete_observation_replay_equal': equal,
    'comparison_exception': 'Only exact absolute owned provider-<original-Ecore-SHA12>/provider.genmodel field is replaced after its root/digest/path validation. No token/error/model/leaf/UTF16 span normalization.',
    'runs': runs,
    'raw_results': pin(ROOT / 'observed-results.json'),
    'native_support_award': False, 'gate_closures': [],
}
save(ROOT / 'receipt.json', receipt)
print(json.dumps({'receipt': pin(ROOT / 'receipt.json'), 'matched': equal, 'counts': {key: receipt[key] for key in ['planned_inputs', 'actual_runtime_processes_both_runs', 'accepted_per_run', 'rejected_per_run', 'rejected_partial_models_per_run']}}))
assert equal
