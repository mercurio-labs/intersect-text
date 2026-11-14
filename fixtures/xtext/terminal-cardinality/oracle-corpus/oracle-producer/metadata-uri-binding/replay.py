"""Frozen explicit logical-provider URI binding; exact complete original metadata only."""
import hashlib
import json
import os
import subprocess
from pathlib import Path
import argparse
ADD = Path(__file__).resolve().parent
CORPUS = ADD.parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--jdk', type=Path, required=True)
parser.add_argument('--maven-cache', type=Path, required=True)
args = parser.parse_args()
OUT = args.output.resolve()
JDK = args.jdk.resolve()
CACHE = args.maven_cache.resolve()
assert OUT != CORPUS and CORPUS not in OUT.parents, 'Output must be NEW and outside immutable corpus'
OUT.mkdir(exist_ok=False)

def disk(p):
    p = Path(p).resolve()
    return p if str(p).startswith('\\\\?\\') else Path('\\\\?\\' + str(p))

def pin(p):
    b = disk(p).read_bytes()
    return {'bytes': len(b), 'sha256': hashlib.sha256(b).hexdigest()}

def read(p):
    return json.loads(disk(p).read_text(encoding='utf-8'))

def save(p, value):
    disk(p).parent.mkdir(parents=True, exist_ok=True)
    with disk(p).open('x', encoding='utf-8', newline='\n') as f:
        f.write(json.dumps(value, indent=2) + '\n')

freeze = read(ADD / 'frozen-addendum.json')
for item in [freeze['helper'], freeze['original_helper'], freeze['original_metadata']]:
    assert pin(CORPUS / item['path']) == {k: item[k] for k in ['bytes', 'sha256']}
for item in freeze['bindings']:
    assert pin(CORPUS / item['physical_provider']) == item['physical_provider_pin']
    assert pin(CORPUS / item['mapping']) == item['mapping_pin']
stack = read(CORPUS / 'inputs/return12/inputs/stack.json')
for item in [*stack['jars'], stack['source_archive']]:
    assert pin(CACHE / item['path'])['sha256'] == item['sha256'], item['path']
assert len(stack['jars']) == 60
java, javac = JDK / 'bin/java.exe', JDK / 'bin/javac.exe'
assert pin(java)['sha256'] == 'abd12f952caba9b7404f15ca3939100f5ad5db5553187a3449814c57c2bf4b82'
assert pin(javac)['sha256'] == 'c81b3bece8daf28136b387841f924c66a9da88746246211cf769bfa6ac3a6f25'
cp = os.pathsep.join(str(CACHE / item['path']) for item in stack['jars'])
original = read(CORPUS / freeze['original_metadata']['path'])
save(OUT / 'preexecution.json', {'producer': pin(Path(__file__)), 'addendum_freeze': pin(ADD / 'frozen-addendum.json'),
    'original_corpus_provenance': pin(CORPUS / 'oracle-provenance.json'), 'java': pin(java), 'javac': pin(javac),
    'original_metadata': freeze['original_metadata'], 'comparison': 'Exact complete JSON; no normalization',
    'helper_and_mapping_frozen_before_execution': True, 'gate_closures': []})
commands = []
def run(command, prefix):
    try:
        p = subprocess.run(command, capture_output=True, timeout=120)
        code, stdout, stderr = p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired as error:
        code, stdout, stderr = 124, error.stdout or b'', error.stderr or b''
    disk(OUT / (prefix + '.stdout.log')).parent.mkdir(parents=True, exist_ok=True)
    disk(OUT / (prefix + '.stdout.log')).write_bytes(stdout)
    disk(OUT / (prefix + '.stderr.log')).write_bytes(stderr)
    commands.append({'command': list(map(str, command)), 'exit_code': code,
        'stdout': {'path': prefix + '.stdout.log', **pin(OUT / (prefix + '.stdout.log'))},
        'stderr': {'path': prefix + '.stderr.log', **pin(OUT / (prefix + '.stderr.log'))}})
    # This progress manifest is new attempt-local output, never an original receipt.
    disk(OUT / 'commands.json').write_text(json.dumps(commands, indent=2) + '\n', encoding='utf-8', newline='\n')
    return code, stdout

try:
    complete = []
    for key in ['record', 'replay']:
        dest = OUT / key
        disk(dest / 'classes').mkdir(parents=True)
        adapter = CORPUS / 'oracle/return12' / key / 'adapter'
        classpath = str(adapter) + os.pathsep + cp
        args = ['-encoding', 'UTF-8', '--release', '17', '-cp', classpath, '-d', str(dest / 'classes'), str(CORPUS / freeze['helper']['path'])]
        argfile = dest / 'javac.args'
        disk(argfile).write_text('\n'.join('"' + x.replace('\\', '/').replace('"', '\\"') + '"' for x in args) + '\n', encoding='utf-8', newline='\n')
        code, _ = run([str(javac), '@' + str(argfile)], key + '/javac')
        assert code == 0, (key, 'javac', code)
        rows = []
        for item in freeze['bindings']:
            physical = CORPUS / item['physical_provider']
            command = [str(java), '-cp', str(dest / 'classes') + os.pathsep + classpath,
                'intersect.oracle.TerminalReturnIdentityOracleMapped', str(physical.parent), str(CORPUS / item['mapping'])]
            code, stdout = run(command, key + '/' + item['id'])
            values = [json.loads(line) for line in stdout.decode('utf-8').splitlines() if line.startswith('{')]
            rows.append({'id': item['id'], 'exit_code': code, 'observation': values[0] if len(values) == 1 else None})
            assert code == 0 and len(values) == 1, (key, item['id'], code)
        save(dest / 'observations.json', rows)
        assert rows == original, (key, 'Complete metadata changed under explicit logical URI binding')
        complete.append(rows)
        print('COMPLETE_MAPPED_METADATA_' + key.upper(), flush=True)
    assert complete[0] == complete[1]
    save(OUT / 'receipt.json', {'state': 'relocated_bytes_explicit_logical_uri_binding_complete_original_metadata_equal',
        'graphs_per_run': len(original), 'fresh_metadata_processes': len(original) * 2, 'independent_javac_runs': 2,
        'record': pin(OUT / 'record/observations.json'), 'replay': pin(OUT / 'replay/observations.json'),
        'commands': pin(OUT / 'commands.json'), 'preexecution': pin(OUT / 'preexecution.json'),
        'comparison_normalization': None, 'original_helper_or_source_mutations': False, 'gate_closures': []})
except BaseException as error:
    save(OUT / 'failure.json', {'state': 'original_addendum_attempt_preserved', 'class': type(error).__name__, 'message': str(error),
         'commands': pin(OUT / 'commands.json') if (OUT / 'commands.json').exists() else None})
    raise
print(json.dumps({'path': str(OUT / 'receipt.json'), **pin(OUT / 'receipt.json')}))
