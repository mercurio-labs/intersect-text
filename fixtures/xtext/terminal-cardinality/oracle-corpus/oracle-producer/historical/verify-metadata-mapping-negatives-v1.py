"""Explicit binding protocol failure controls; no authored source edits."""
import hashlib
import json
import subprocess
from pathlib import Path
HERE = Path(__file__).resolve().parent
CORPUS = HERE / 'portable-terminal-cardinality-v1'
BASE = HERE / 'metadata-binding-attempt1'
OUT = HERE / 'metadata-binding-negative-attempt1'
OUT.mkdir(exist_ok=False)
def read(p):
    return json.loads(Path(p).read_text(encoding='utf-8'))
def pin(p):
    b = Path(p).read_bytes()
    return {'bytes': len(b), 'sha256': hashlib.sha256(b).hexdigest()}
def save(p, v):
    with Path(p).open('x', encoding='utf-8', newline='\n') as f:
        f.write(json.dumps(v, indent=2) + '\n')
frozen = read(CORPUS / 'oracle-producer/metadata-uri-binding/frozen-addendum.json')
binding = next(v for v in frozen['bindings'] if v['id'] == 'ecore_aliased_implicit')
original = (CORPUS / binding['mapping']).read_text(encoding='utf-8')
assert pin(CORPUS / binding['mapping']) == binding['mapping_pin']
command = next(v['command'] for v in read(BASE / 'commands.json') if v['command'][-2].endswith('/ecore_aliased_implicit') or v['command'][-2].endswith('\\ecore_aliased_implicit'))
assert command[-3] == 'intersect.oracle.TerminalReturnIdentityOracleMapped'
protocols = {
    'ambiguous_duplicate_rows': (original + original, 'Expected exactly one provider binding'),
    'mismatched_bytes': (original.replace(binding['physical_provider_pin']['sha256'], '0' * 64), 'Provider binding bytes differ'),
    'unsupported_escaping_path': (original.replace('\timported.ecore', '\t../imported.ecore'), 'Invalid provider binding'),
}
inputs = []
for name, (text, message) in protocols.items():
    mapping = OUT / (name + '.tsv')
    with mapping.open('x', encoding='utf-8', newline='\n') as f:
        f.write(text)
    inputs.append({'id': name, 'mapping': str(mapping), 'pin': pin(mapping), 'expected_error': message})
save(OUT / 'preexecution.json', {'producer': pin(Path(__file__)), 'original_binding_receipt': pin(BASE / 'receipt.json'),
    'original_mapping': binding, 'compiled_helper': pin(BASE / 'record/classes/intersect/oracle/TerminalReturnIdentityOracleMapped.class'),
    'inputs': inputs, 'source_changes': False})
all_rows = []
for key in ['record', 'replay']:
    rows = []
    for item in inputs:
        cmd = [*command[:-1], item['mapping']]
        p = subprocess.run(cmd, capture_output=True, timeout=120)
        assert p.returncode != 0 and item['expected_error'].encode() in p.stderr
        prefix = OUT / (key + '-' + item['id'])
        prefix.with_suffix('.stdout.log').write_bytes(p.stdout)
        prefix.with_suffix('.stderr.log').write_bytes(p.stderr)
        rows.append({'id': item['id'], 'exit_code': p.returncode, 'stdout': pin(prefix.with_suffix('.stdout.log')), 'stderr': pin(prefix.with_suffix('.stderr.log'))})
    save(OUT / (key + '.json'), rows)
    all_rows.append(rows)
assert all_rows[0] == all_rows[1]
save(OUT / 'receipt.json', {'state': 'all_explicit_binding_protocol_failures_reject_and_fresh_replay_exact',
    'cases': len(inputs), 'fresh_java_processes': len(inputs) * 2, 'record': pin(OUT / 'record.json'),
    'replay': pin(OUT / 'replay.json'), 'preexecution': pin(OUT / 'preexecution.json'), 'gate_closures': []})
print(json.dumps({'path': str(OUT / 'receipt.json'), **pin(OUT / 'receipt.json')}))
