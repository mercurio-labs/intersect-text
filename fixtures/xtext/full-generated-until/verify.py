"""Frozen full-generated Until oracle integrity and optional fresh replay."""
import argparse
import copy
import hashlib
import json
import subprocess
import sys
from pathlib import Path, PurePosixPath, PureWindowsPath

FIXTURE = Path(__file__).resolve().parent
FAILED_GRAPHS = {'inherited_operand_override', 'explicit_parent_operand'}
EXPECTED_COUNTS = {
    'graphs': 18, 'planned_inputs': 228, 'generation_successes': 16,
    'generation_failures': 2, 'runtime_inputs': 204, 'unexecuted_inputs': 24,
    'accepted': 48, 'rejected': 156, 'rejected_partial': 60,
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding='utf-8'))


def comparable(rows, output_roots):
    """Validate exact owned GenModel locations before replacing that field only."""
    result = copy.deepcopy(rows)
    for row in result:
        provider = row['provider']
        raw = provider['genmodel']
        kind = PureWindowsPath if PureWindowsPath(raw).drive else PurePosixPath
        path = kind(raw)
        root = kind(str(output_roots[row['id']] if isinstance(output_roots, dict) else output_roots))
        sha = provider['ecore_sha256']
        if not path.is_absolute() or not root.is_absolute() or '..' in path.parts or '..' in root.parts:
            raise ValueError('Provider GenModel is not an absolute owned output path')
        if len(sha) != 64 or any(c not in '0123456789abcdef' for c in sha):
            raise ValueError('Provider Ecore SHA256 differs')
        if path != root / ('provider-' + sha[:12]) / 'provider.genmodel':
            raise ValueError('Provider GenModel lies outside the exact owned provider output')
        provider['genmodel'] = '<provider-output>/provider.genmodel'
    return result


def validate(rows, cases, positive):
    if [r['id'] for r in rows] != [c['id'] for c in cases]:
        raise ValueError('Complete frozen graph order/identity differs')
    expected_positive = []
    counts = dict.fromkeys(EXPECTED_COUNTS, 0)
    counts['graphs'] = len(rows)
    for case, row in zip(cases, rows):
        planned = case['inputs']
        counts['planned_inputs'] += len(planned)
        if row['entry'] != case['entry'] or row['original_inputs'] != planned:
            raise ValueError('Original entry/request graph differs')
        if row['provider']['generation_exit'] != 0 or row['provider']['compile_exit'] != 0:
            raise ValueError('Actual original Ecore generated provider missing')
        metadata = row['metadata']
        if metadata['exit_code'] != 0 or metadata['observation'] is None:
            raise ValueError('Metadata observation did not complete')
        meta = metadata['observation']
        if any(s['errors'] for s in meta['syntax'] + meta['linking']):
            raise ValueError('Frozen syntax/linking outcome differs')
        if meta['linking_failure'] is not None or meta['translation_failure'] is not None:
            raise ValueError('Frozen linking/translation stage differs')
        errors = [issue for source in meta['validation'] for issue in source['issues'] if issue['severity'] == 'ERROR']
        if case['id'] in FAILED_GRAPHS:
            if [e['message'] for e in errors] != ['The first rule must be a parser rule.']:
                raise ValueError('Original invalid inheritance diagnostic was repaired or changed')
            if row['generation'] != {'exit_code': 1, 'source_unchanged': True}:
                raise ValueError('Original failed full-generation stage differs')
            if row['compilation'] is not None or row['runtime']:
                raise ValueError('Original unexecuted requests became fabricated observations')
            counts['generation_failures'] += 1
            counts['unexecuted_inputs'] += len(planned)
            continue
        if errors or row['generation'] != {'exit_code': 0, 'source_unchanged': True} or row['compilation'] != {'exit_code': 0}:
            raise ValueError('Full generation/compilation success stage differs')
        counts['generation_successes'] += 1
        if [r['text'] for r in row['runtime']] != planned:
            raise ValueError('Runtime request graph incomplete')
        for run in row['runtime']:
            if run['exit_code'] != 0 or run['timeout_seconds'] is not None:
                raise ValueError('Runtime process did not complete')
            full = run['observation']
            if full is None or full['text'] != run['text'] or not full['lexer']['through_eof']:
                raise ValueError('Full runtime request/token closure differs')
            resource, parser = full['resource'], full['parser']
            accepted = resource['model'] is not None and not resource['errors'] and resource['failure'] is None
            if resource['accepted'] != accepted or parser['accepted'] != accepted:
                raise ValueError('Rejected partial/error resource mislabeled as successful')
            counts['runtime_inputs'] += 1
            counts['accepted' if accepted else 'rejected'] += 1
            if accepted:
                expected_positive.append({'id': case['id'], 'text': run['text'], 'resource': resource})
            elif resource['model'] is not None:
                counts['rejected_partial'] += 1
    if expected_positive != positive:
        raise ValueError('Actual accepted resource/model projection differs')
    if counts != EXPECTED_COUNTS:
        raise ValueError('Complete original evidence denominator differs: ' + str(counts))
    return counts


def frozen():
    provenance = load(FIXTURE / 'provenance.json')
    for name, sha in provenance['files'].items():
        if digest(FIXTURE / name) != sha:
            raise ValueError('Frozen bytes differ: ' + name)
    owned = {p.relative_to(FIXTURE).as_posix() for p in FIXTURE.rglob('*') if p.is_file() and '__pycache__' not in p.parts}
    expected = set(provenance['files']) | {'provenance.json'} | set(provenance['co_owned_files'])
    if owned - expected:
        raise ValueError('Unpinned corpus files: ' + str(sorted(owned - expected)))
    for name, sha in load(FIXTURE / 'inputs/pins.json').items():
        if digest(FIXTURE / 'inputs' / name) != sha:
            raise ValueError('Original source graph differs: ' + name)
    rows, cases = load(FIXTURE / 'observations.json'), load(FIXTURE / 'inputs/cases.json')
    for case in cases:
        for source in case['sources']:
            if digest(FIXTURE / 'inputs' / case['id'] / source['file']) != source['source_id']['sha256']:
                raise ValueError('Exact source identity differs')
    comparable(rows, provenance['recorded_provider_output_roots'])
    return validate(rows, cases, load(FIXTURE / 'positive-resources.json'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--replay', type=Path)
    parser.add_argument('--jdk', type=Path)
    parser.add_argument('--maven-cache', type=Path)
    args = parser.parse_args()
    counts = frozen()
    if args.replay:
        if args.jdk is None or args.maven_cache is None:
            parser.error('Replay requires explicit --jdk and --maven-cache')
        output = args.replay.resolve()
        subprocess.run([sys.executable, '-B', str(FIXTURE / 'produce.py'), '--output', str(output), '--jdk', str(args.jdk.resolve()), '--maven-cache', str(args.maven_cache.resolve())], check=True)
        current = load(output / 'observations.json')
        validate(current, load(FIXTURE / 'inputs/cases.json'), load(FIXTURE / 'positive-resources.json'))
        if comparable(current, output) != comparable(load(FIXTURE / 'observations.json'), load(FIXTURE / 'provenance.json')['recorded_provider_output_roots']):
            raise ValueError('Complete full generated-language replay differs')
        frozen()
    print(json.dumps({'state': 'verified', 'counts': counts, 'native_award': False}))


if __name__ == '__main__':
    main()
