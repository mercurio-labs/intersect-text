"""Fresh portable full-source generation/runtime record+replay, original producers unchanged."""
import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
for name in ['corpus', 'output', 'jdk', 'maven-cache']:
    parser.add_argument('--' + name, type=Path, required=True)
args = parser.parse_args()
CORPUS, OUT = args.corpus.resolve(), args.output.resolve()
assert OUT != CORPUS and CORPUS not in OUT.parents
OUT.mkdir(exist_ok=False)
def disk(p):
    p = Path(p).resolve()
    return p if os.name != 'nt' or str(p).startswith('\\\\?\\') else Path('\\\\?\\' + str(p))
def pin(p):
    b = disk(p).read_bytes()
    return {'bytes': len(b), 'sha256': hashlib.sha256(b).hexdigest()}
def read(p):
    return json.loads(disk(p).read_text(encoding='utf-8'))
def save(name, value):
    disk(OUT / name).write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8', newline='\n')
manifest = read(CORPUS / 'portable-manifest.json')
for relative, expected in manifest['files'].items():
    assert pin(CORPUS / relative) == expected, relative
verifier = HERE / 'original-verifier.py'
assert pin(verifier)['sha256'] == 'e24383db99855d10b39fd495cc5fa9f7459286550176b83ec26cdb6130c6d729'
spec = importlib.util.spec_from_file_location('original_frozen_full_verifier', verifier)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
stack = read(CORPUS / 'inputs/return12/inputs/stack.json')
assert len(stack['jars']) == 60
for item in [*stack['jars'], stack['source_archive']]:
    assert pin(args.maven_cache / item['path'])['sha256'] == item['sha256']
java, javac = args.jdk / 'bin/java.exe', args.jdk / 'bin/javac.exe'
assert pin(java)['sha256'] == 'abd12f952caba9b7404f15ca3939100f5ad5db5553187a3449814c57c2bf4b82'
assert pin(javac)['sha256'] == 'c81b3bece8daf28136b387841f924c66a9da88746246211cf769bfa6ac3a6f25'
historical_target = Path('D:/dev/repos/mercurio-labs/intersect/target')
cohorts = [
    ('original8', 'inputs/original8', 'produce.py', 'oracle/original8/record', historical_target / 'tc-full-oracle-v1/record'),
    ('typed8', 'inputs/typed8', 'produce.py', 'oracle/typed8/record', historical_target / 'tct-oracle-v1/record'),
    ('fragment2', 'inputs/fragment2', 'produce.py', 'oracle/fragment2/record', historical_target / 'tcf-oracle-v1/record'),
    ('return12', 'inputs/return12', 'produce.py', 'oracle/return12/record', historical_target / 'trm-oracle-v1/record'),
    ('inherited2', 'oracle/inherited-workflow/producer-and-receipts', 'produce-attempt4.py', 'oracle/inherited-workflow/record', historical_target / 'tilw-a4-record'),
]
relocations = []
for label, relative, producer_name, baseline, original_root in cohorts:
    root = CORPUS / relative
    cases = read(root / 'inputs/cases.json')
    relocations.append({'id': label, 'physical_inputs': relative, 'input_cases': pin(root / 'inputs/cases.json'),
        'producer': {'path': relative + '/' + producer_name, **pin(root / producer_name)},
        'baseline_observations': {'path': baseline + '/observations.json', **pin(CORPUS / baseline / 'observations.json')},
        'original_owned_output_root': str(original_root), 'graphs': len(cases), 'planned_inputs': sum(len(c['inputs']) for c in cases)})
save('preexecution.json', {'state': 'all_producer_relocation_and_toolchain_inputs_frozen_before_execution',
    'producer': pin(Path(__file__)), 'original_verifier': pin(verifier), 'corpus_manifest': pin(CORPUS / 'portable-manifest.json'),
    'relocations': relocations, 'java': pin(java), 'javac': pin(javac), 'stack': stack,
    'comparison_exception': 'Exact absolute owned provider-<ecoreSHA12>/provider.genmodel only',
    'source_or_producer_helper_bytes_changed': False, 'gate_closures': []})
commands, results = [], []
try:
    for label, relative, producer_name, baseline, original_root in cohorts:
        source = CORPUS / relative
        module.INPUT = source
        cases = read(source / 'inputs/cases.json')
        expected = module.comparable(read(CORPUS / baseline / 'observations.json'), original_root)
        seen = []
        for key in ['record', 'replay']:
            destination = OUT / (label + '-' + key)
            command = [sys.executable, '-B', str(source / producer_name), '--output', str(destination),
                '--jdk', str(args.jdk), '--maven-cache', str(args.maven_cache)]
            print('START_PORTABLE_FULL_' + label + '_' + key, flush=True)
            with (OUT / (label + '-' + key + '.log')).open('wb') as log:
                code = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT).returncode
            commands.append({'command': command, 'exit_code': code, 'log': pin(OUT / (label + '-' + key + '.log'))})
            save('commands.json', commands)
            assert code == 0, (label, key, code)
            rows, comparable, proof = module.check(destination, cases)
            assert comparable == expected, (label, key, 'Full original observations differ; no extra normalization allowed')
            seen.append(comparable)
            results.append({'id': label, 'run': key, 'complete_raw_outputs': proof, 'classification': module.classify(destination, rows)})
            save('results.json', results)
            print('COMPLETE_PORTABLE_FULL_' + label + '_' + key, flush=True)
        assert seen[0] == seen[1]
    for relative, expected_pin in manifest['files'].items():
        assert pin(CORPUS / relative) == expected_pin, relative
    save('receipt.json', {'state': 'all30_original_graphs_plus_distinct_same_source_inherited2_regenerated_fresh_replay_exact',
        'runs': results, 'preexecution': pin(OUT / 'preexecution.json'), 'commands': pin(OUT / 'commands.json'),
        'original_source_failures_workflow_failures_timeouts_partials_unexecuted': 'Exact full observations retained and equal, no source repair/reclassification',
        'comparison_exception': 'Exact absolute owned provider-<ecoreSHA12>/provider.genmodel only', 'gate_closures': []})
except BaseException as error:
    save('failure.json', {'state': 'portable_attempt_failed_or_unavailable_subset_preserved', 'class': type(error).__name__,
        'message': str(error), 'commands': pin(OUT / 'commands.json') if (OUT / 'commands.json').exists() else None,
        'completed_runs': results, 'no_source_repairs': True})
    raise
print(json.dumps({'path': str(OUT / 'receipt.json'), **pin(OUT / 'receipt.json')}), flush=True)
