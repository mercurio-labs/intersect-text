"""Verify three immutable full-source negation cohorts and optionally regenerate."""
import argparse
import base64
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('original_five_verifier', HERE / 'verify-original5.py')
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)

COHORTS = {
    'compositions': {'graphs': 6, 'planned': 54, 'runtime': 54, 'accepted': 19, 'rejected': 35, 'rejected_partial': 21},
    'exact-negatives': {'graphs': 4, 'planned': 36, 'runtime': 36, 'accepted': 12, 'rejected': 24, 'rejected_partial': 13},
}


def validate_extra(root, cohort, cases):
    manifest = base.load(root / 'artifacts-sha256.json')
    actual = set()
    for folder, _, names in os.walk(base.disk(root)):
        for name in names:
            relative = (Path(folder) / name).relative_to(base.disk(root)).as_posix()
            if relative != 'artifacts-sha256.json':
                actual.add(relative)
    if actual != set(manifest):
        raise ValueError('Incomplete raw artifact manifest: ' + str(root))
    for name, digest in manifest.items():
        if base.pin(root / name)['sha256'] != digest:
            raise ValueError('Raw artifact differs: ' + name)
    rows = base.load(root / 'observations.json')
    if [r['id'] for r in rows] != [c['id'] for c in cases]:
        raise ValueError('Original graph order/identity differs')
    counts = dict.fromkeys(['graphs', 'planned', 'runtime', 'accepted', 'rejected', 'rejected_partial'], 0)
    counts['graphs'] = len(rows)
    expected_commands, positives = [], []
    commands = base.load(root / 'commands.json')
    generation = [c for c in commands if 'org.eclipse.emf.mwe2.launch.runtime.Mwe2Launcher' in c['command']]
    if len(generation) != len(rows):
        raise ValueError('Actual full-source generation commands incomplete')
    for row, case, command in zip(rows, cases, generation):
        if row['entry'] != case['entry'] or row['original_inputs'] != case['inputs']:
            raise ValueError('Original source/runtime request differs')
        sources = cohort / 'inputs' / case['id']
        if row['source_pins'] != {p.name: base.pin(p)['sha256'] for p in sources.iterdir() if p.is_file()}:
            raise ValueError('Original source bytes differ')
        if row['metadata']['exit_code'] != 0:
            raise ValueError('Metadata process incomplete')
        metadata = row['metadata']['observation']
        if any(s['errors'] for s in metadata['syntax']) or any(s['errors'] for s in metadata['linking']):
            raise ValueError('Clean source syntax/linking stage differs')
        if metadata['linking_failure'] is not None or metadata['translation_failure'] is not None:
            raise ValueError('Clean source translation/linking failed')
        if any(s['failure'] is not None or any(i['severity'] == 'ERROR' for i in s['issues']) for s in metadata['validation']):
            raise ValueError('Clean source validation stage differs')
        if row['generation'] != {'exit_code': 0, 'source_unchanged': True} or row['compilation'] != {'exit_code': 0}:
            raise ValueError('Full generation/javac stage differs')
        if row['provider']['generation_exit'] != 0 or row['provider']['compile_exit'] != 0:
            raise ValueError('Original Ecore-generated provider missing')
        output = base.raw(root / command['stdout']) + base.raw(root / command['stderr'])
        if b'error(' in output or b'warning(' in output:
            raise ValueError('Previously clean ANTLR generation has diagnostics')
        if [r['text'] for r in row['runtime']] != case['inputs']:
            raise ValueError('Complete runtime requests differ')
        counts['planned'] += len(case['inputs'])
        for run in row['runtime']:
            observation = run['observation']
            resource, parser = observation['resource'], observation['parser']
            if run['exit_code'] != 0 or run['timeout_seconds'] is not None:
                raise ValueError('Actual runtime incomplete')
            if observation['text'] != run['text'] or not observation['lexer']['through_eof']:
                raise ValueError('Raw UTF16 input/token stream differs')
            accepted = resource['model'] is not None and not resource['errors'] and resource['failure'] is None
            if resource['accepted'] != accepted or parser['accepted'] != accepted:
                raise ValueError('Rejected partial incorrectly labeled successful')
            counts['runtime'] += 1
            counts['accepted' if accepted else 'rejected'] += 1
            counts['rejected_partial'] += int(not accepted and resource['model'] is not None)
            if accepted:
                positives.append({'id': case['id'], 'text': run['text'], 'resource': resource})
            expected_commands.append((case['entry'], base64.b64encode(run['text'].encode('utf-8')).decode('ascii'), run['exit_code']))
    runtime = [c for c in commands if 'intersect.oracle.FullRuntime' in c['command']]
    if len(runtime) != len(expected_commands):
        raise ValueError('Actual generated runtime invocation count differs')
    for command, (entry, text, code) in zip(runtime, expected_commands):
        if command['command'][-2:] != [entry, text] or command['exit_code'] != code:
            raise ValueError('Actual generated lexer/parser/resource request differs')
    if '21.0.6+7' not in base.load(root / 'toolchain.json')['jdk']:
        raise ValueError('Pinned JDK differs')
    return rows, counts, positives


def frozen():
    # The original five-source verifier and every original raw file stay exact.
    original = base.frozen()
    provenance = base.load(HERE / 'provenance.json')
    all_positives = []
    for name, expected_counts in COHORTS.items():
        cohort = HERE / 'expansion' / name
        cases = base.load(cohort / 'inputs/cases.json')
        for relative, digest in base.load(cohort / 'inputs/pins.json').items():
            if base.pin(cohort / 'inputs' / relative)['sha256'] != digest:
                raise ValueError('Expanded source pin differs')
        comparisons = []
        for key in ['record', 'replay']:
            rows, counts, positives = validate_extra(cohort / 'oracle' / key, cohort, cases)
            if counts != expected_counts:
                raise ValueError('Expanded complete denominator differs: ' + str(counts))
            comparisons.append(base.comparable(rows, provenance['expanded_output_roots'][name][key]))
        if comparisons[0] != comparisons[1]:
            raise ValueError('Complete expanded record/replay observations differ')
        all_positives.extend(positives)
        prefix = 'combinations' if name == 'compositions' else 'exact-negatives'
        for key in ['record', 'replay']:
            if base.raw(HERE / f'{prefix}-{key}-observations.json') != base.raw(cohort / 'oracle' / key / 'observations.json'):
                raise ValueError('Raw consumer observations differ')
    if all_positives != base.load(HERE / 'expanded-positive-resources.json'):
        raise ValueError('Expanded successful full model projection differs')
    return original


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--replay', type=Path)
    parser.add_argument('--jdk', type=Path)
    parser.add_argument('--maven-cache', type=Path)
    args = parser.parse_args()
    frozen()
    if args.replay:
        if args.jdk is None or args.maven_cache is None:
            parser.error('Fresh replay requires explicit pinned --jdk and --maven-cache')
        output = args.replay.resolve()
        if output == HERE or HERE in output.parents or output.exists():
            parser.error('Replay output must be NEW and outside the fixture')
        output.mkdir(parents=True)
        provenance = base.load(HERE / 'provenance.json')
        for name in ['original5', *COHORTS]:
            cohort = HERE if name == 'original5' else HERE / 'expansion' / name
            current = output / name
            subprocess.run([sys.executable, '-B', str(cohort / 'produce.py'), '--output', str(current), '--jdk', str(args.jdk.resolve()), '--maven-cache', str(args.maven_cache.resolve())], check=True)
            cases = base.load(cohort / 'inputs/cases.json')
            if name == 'original5':
                rows, _, _ = base.validate(current, cases)
                recorded_root = provenance['original_output_roots']['record']
            else:
                rows, counts, _ = validate_extra(current, cohort, cases)
                if counts != COHORTS[name]:
                    raise ValueError('Fresh runtime denominator differs')
                recorded_root = provenance['expanded_output_roots'][name]['record']
            if base.comparable(rows, current) != base.comparable(base.load(cohort / 'oracle/record/observations.json'), recorded_root):
                raise ValueError('Fresh complete observations differ: ' + name)
        frozen()
    print(json.dumps({'state': 'verified', 'cohorts': {'original5': {'graphs': 5, 'runtime': 40, 'accepted': 15, 'rejected': 25, 'rejected_partial': 18}, **COHORTS}, 'full_runtime_per_run': 130, 'clean_native_slice': {'graphs': 13, 'inputs': 117, 'successful_models': 42}, 'native_award': False, 'gate_closures': []}))


if __name__ == '__main__':
    main()
