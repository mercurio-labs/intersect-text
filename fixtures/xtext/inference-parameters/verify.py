"""Read-only pinned Xtext parameter inference/generation/model evidence replay."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import zipfile

VARIANTS = ('original_obstructions', 'lowercase_counterparts', 'additional_controls')


def checked(path, digest):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != digest:
        raise ValueError(f'SHA-256 mismatch: {path}')
    return data


def runtime_oracle(fixture):
    base = fixture.parent / 'inference-fragment-runtime'
    spec = importlib.util.spec_from_file_location('parameter_shared_runtime_oracle', base / 'verify.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return base, module


def counts(cases, observations):
    graphs = [c for variant in VARIANTS for c in cases[variant]]
    metadata = [m for variant in VARIANTS for m in observations[variant]['metadata']]
    generation = [g for variant in VARIANTS for g in observations[variant]['generation']]
    runs = [r for variant in VARIANTS for r in observations[variant]['runs']]
    return {
        'graphs': len(graphs), 'sources': sum(len(c['sources']) for c in graphs),
        'inference_accepted': sum(m['all_sources_accepted'] for m in metadata),
        'inference_rejected': sum(not m['all_sources_accepted'] for m in metadata),
        'compiled_graphs': sum(g['stage'] == 'compiled' for g in generation),
        'generation_failed': sum(g['stage'] == 'generation_failed' for g in generation),
        'runtime_requests': len(runs), 'runtime_accepted': sum(r['accepted'] for r in runs),
        'runtime_rejected': sum(not r['accepted'] for r in runs),
        'planned_unexecuted_requests': sum(len(c['planned_runs']) - len(c['runs']) for c in graphs),
    }


def validate(fixture, cases, observations):
    base, oracle = runtime_oracle(fixture)
    oracle.frozen(base)
    if set(cases) != set(VARIANTS) or set(observations) != set(VARIANTS):
        raise ValueError('Parameter variant inventory differs')
    for variant in VARIANTS:
        oracle.validate(base, cases[variant], observations[variant])
        for case in cases[variant]:
            if variant == 'original_obstructions':
                if case['runs']:
                    raise ValueError('Original generator obstruction acquired runtime authority')
            elif case['runs'] != case['planned_runs']:
                raise ValueError('A planned counterpart/additional request was omitted')
    originals, counterparts = cases['original_obstructions'], cases['lowercase_counterparts']
    if [c['id'] for c in originals] != [c['id'] for c in counterparts]:
        raise ValueError('Namespace counterpart case identity differs')
    for original, counterpart in zip(originals, counterparts):
        if original['planned_runs'] != counterpart['planned_runs'] or original['entry'] != counterpart['entry']:
            raise ValueError('Namespace counterpart request/entry scope changed')
        pairs = list(zip(original['sources'], counterpart['sources']))
        if len(original['sources']) != len(counterpart['sources']):
            raise ValueError('Namespace counterpart source denominator changed')
        replacements = {}
        for before, after in pairs:
            parts = before['grammar'].split('.')
            expected = '.'.join([p.lower() for p in parts[:-1]] + parts[-1:])
            if after['grammar'] != expected:
                raise ValueError('Namespace counterpart changed a language identity beyond capitalization')
            replacements[before['grammar']] = expected
        for before, after in pairs:
            expected = before['source']
            for old, new in replacements.items():
                expected = expected.replace(old, new)
            if after['source'] != expected or before['id'] != after['id'] or before['dependencies'] != after['dependencies']:
                raise ValueError('Namespace counterpart changed source semantics or dependency grants')
    before = observations['original_obstructions']['metadata']
    after = observations['lowercase_counterparts']['metadata']
    if [m['all_sources_accepted'] for m in before] != [m['all_sources_accepted'] for m in after]:
        raise ValueError('Namespace counterpart inference acceptance changed')


def frozen(fixture):
    p = json.loads((fixture / 'provenance.json').read_text(encoding='utf-8'))
    if set(p['files']) != {'cases.json', 'observations.json', 'README.md', 'verify.py'}:
        raise ValueError('Parameter evidence component inventory differs')
    for name, digest in p['files'].items():
        checked(fixture / name, digest)
    for name, digest in p['dependencies'].items():
        checked(fixture.parents[2] / name, digest)
    cases = json.loads((fixture / 'cases.json').read_text(encoding='utf-8'))
    rows = json.loads((fixture / 'observations.json').read_text(encoding='utf-8'))
    validate(fixture, cases, rows)
    if counts(cases, rows) != p['counts']:
        raise ValueError('Parameter evidence denominator differs')
    return p, cases, rows


def replay(fixture, p, cases, jdk, cache, output):
    base, oracle = runtime_oracle(fixture)
    old, _, _ = oracle.frozen(base)
    archive = cache / p['source_archive']['path']
    checked(archive, p['source_archive']['sha256'])
    with zipfile.ZipFile(archive) as source:
        for name, digest in p['parameter_source_members'].items():
            if hashlib.sha256(source.read(name)).hexdigest() != digest:
                raise ValueError('Pinned parameter semantic source member differs')
    output.mkdir(parents=True, exist_ok=False)
    rows = {}
    for variant, directory in zip(VARIANTS, ('o', 'l', 'a')):
        rows[variant] = oracle.replay(base, old, cases[variant], jdk, cache, output / directory)
    validate(fixture, cases, rows)
    (output / 'observations-replay.json').write_text(json.dumps(rows, indent=2) + '\n', encoding='utf-8', newline='\n')
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--integrity-only', action='store_true')
    parser.add_argument('--jdk', type=Path)
    parser.add_argument('--artifact-dir', type=Path)
    parser.add_argument('--maven-cache', type=Path, default=Path.home() / '.m2/repository')
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    p, cases, expected = frozen(fixture)
    if not args.integrity_only:
        if args.jdk is None or args.artifact_dir is None:
            parser.error('--jdk and --artifact-dir are required for independent replay')
        actual = replay(fixture, p, cases, args.jdk, args.maven_cache, args.artifact_dir.resolve())
        if actual != expected:
            raise ValueError('Complete parameter inference/generation/runtime observations differ')
    print(json.dumps(p['counts'], sort_keys=True))
    print('Independent oracle evidence only; native parameter support and full X03/X07/S09 remain open.')


if __name__ == '__main__':
    main()
