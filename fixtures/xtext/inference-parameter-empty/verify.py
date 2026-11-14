"""Read-only supplemental parameter forwarding/empty-resource evidence."""
import argparse
import copy
import hashlib
import importlib.util
import json
from pathlib import Path


def base(fixture):
    path = fixture.parent / 'inference-fragment-runtime'
    spec = importlib.util.spec_from_file_location('empty_shared_oracle', path / 'verify.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return path, module


def validate(fixture, cases, rows, original):
    # Keep all exact metadata/generation prerequisite checks in the frozen verifier.
    empty_cases, prerequisites = copy.deepcopy(cases), copy.deepcopy(rows)
    for case in empty_cases:
        case['runs'] = []
    prerequisites['runs'] = []
    original(fixture, empty_cases, prerequisites)
    requests = [(c['id'], r['entry'], r['text']) for c in cases for r in c['runs']]
    if [(r['case'], r['entry'], r['text']) for r in rows['runs']] != requests:
        raise ValueError('Supplemental request identity/order differs')
    for row in rows['runs']:
        if row['model'] is None and row['errors'] == [] and row['exception'] is None:
            if row['accepted'] is not False or row['entry'] != 'resource' or row['resource_contents'] != 0:
                raise ValueError('Error-free empty-resource shape differs')
        else:
            single_cases = copy.deepcopy(empty_cases)
            case = next(c for c in single_cases if c['id'] == row['case'])
            case['runs'] = [{'entry': row['entry'], 'text': row['text']}]
            single = {**prerequisites, 'runs': [row]}
            original(fixture, single_cases, single)


def frozen(fixture):
    provenance = json.loads((fixture / 'provenance.json').read_text(encoding='utf-8'))
    if set(provenance['files']) != {'cases.json', 'observations.json', 'README.md', 'verify.py'}:
        raise ValueError('Supplemental component inventory differs')
    for name, digest in provenance['files'].items():
        if hashlib.sha256((fixture / name).read_bytes()).hexdigest() != digest:
            raise ValueError('Supplemental source/observation digest differs: ' + name)
    path, oracle = base(fixture)
    oracle.frozen(path)
    cases = json.loads((fixture / 'cases.json').read_text(encoding='utf-8'))
    rows = json.loads((fixture / 'observations.json').read_text(encoding='utf-8'))
    validate(path, cases, rows, oracle.validate)
    counts = {'graphs': len(cases), 'requests': len(rows['runs']),
              'rooted_successes': sum(r['accepted'] for r in rows['runs']),
              'syntax_errors': sum(bool(r['errors']) for r in rows['runs']),
              'error_free_empty_resources': sum(r['model'] is None and not r['errors'] and r['exception'] is None for r in rows['runs'])}
    if counts != provenance['counts']:
        raise ValueError('Supplemental outcome denominator differs')
    return cases, rows, counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--integrity-only', action='store_true')
    parser.add_argument('--jdk', type=Path)
    parser.add_argument('--artifact-dir', type=Path)
    parser.add_argument('--maven-cache', type=Path, default=Path.home() / '.m2/repository')
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    cases, expected, counts = frozen(fixture)
    if not args.integrity_only:
        if args.jdk is None or args.artifact_dir is None:
            parser.error('--jdk and --artifact-dir are required')
        path, oracle = base(fixture)
        provenance, _, _ = oracle.frozen(path)
        original = oracle.validate
        oracle.validate = lambda f, c, rows: validate(f, c, rows, original)
        actual = oracle.replay(path, provenance, cases, args.jdk, args.maven_cache, args.artifact_dir)
        if actual != expected:
            raise ValueError('Complete supplemental observations differ')
    print(json.dumps(counts, sort_keys=True))
    print('Independent supplemental evidence only; full support gates remain open.')


if __name__ == '__main__':
    main()
