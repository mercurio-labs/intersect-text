"""Pooled-allocator benchmark host: build it, then pair it with the Java reference.

The host is `native.rs` unchanged except for a global allocator (`host_pool.rs`)
and one `recycle` call where the first graph has been dropped, the point at
which the Java host requests a full collection. The consumer is built with fat
LTO and one codegen unit. Allocator and build profile are host choices: runtime
sources are the explicit Model and Text worktrees, with no production change.

    python -B benchmarks/java-parity/pooled_host.py build --model-root ../intersect-model --text-root . --project target/java-parity/pooled-host --lock <benchmark-consumer-Cargo.lock>
    python -B benchmarks/java-parity/pooled_host.py run --baseline-root <fixture-root> --jdk <jdk-root> --java-scope-classes target/java-parity/indexed-reference-v1/scope-classes --output target/java-parity/pooled-new --size 1000000 --degree 8 --repeats 5 pooled=target/java-parity/pooled-host/target/release/sysml-scale-benchmark.exe java

`run` uses the baseline runner's own protocol (fresh serial processes, paused
memory samples, graph checksum checks) and alternates profile order across
repeats. Each profile is `label=executable` or `java`.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
ALLOCATOR = 'mod pool;\n#[global_allocator]\nstatic POOL: pool::Pool = pool::Pool::new();\n'
BETWEEN = '        if trial == "cold" {\n            event("between", json!({}))?;'
RECYCLE = ('        if trial == "cold" {\n'
           '            // The first graph is dropped: as the Java host requests a full\n'
           '            // collection here, return its freed chunks to sequential use.\n'
           '            POOL.recycle();\n'
           '            event("between", json!({}))?;')


def build(a):
    model, text, project = a.model_root.resolve(), a.text_root.resolve(), a.project.resolve()
    (project / 'src').mkdir(parents=True, exist_ok=True)
    host = (HERE / 'native.rs').read_text(encoding='utf-8')
    assert host.count(BETWEEN) == 1, 'native.rs no longer has the expected between-loads event'
    (project / 'src/main.rs').write_text(ALLOCATOR + host.replace(BETWEEN, RECYCLE), encoding='utf-8')
    shutil.copyfile(HERE / 'host_pool.rs', project / 'src/pool.rs')
    path = lambda root, crate: json.dumps((root / 'crates' / crate).as_posix())
    manifest = '''[package]
name="sysml-scale-benchmark"
version="0.1.0"
edition="2024"
[dependencies]
intersect-model={path=MODEL,features=["xmi"]}
intersect-text={path=TEXT}
serde_json="1.0.149"
sha2="0.10.9"
[patch."https://github.com/mercurio-labs/intersect-model.git"]
intersect-model={path=MODEL}
intersect-build-support={path=SUPPORT}
[workspace]
[profile.release]
lto="fat"
codegen-units=1
panic="abort"
'''.replace('MODEL', path(model, 'intersect-model')).replace('TEXT', path(text, 'intersect-text')).replace(
        'SUPPORT', path(model, 'intersect-build-support'))
    (project / 'Cargo.toml').write_text(manifest, encoding='utf-8')
    if not (project / 'Cargo.lock').exists():
        shutil.copyfile(a.lock, project / 'Cargo.lock')
    subprocess.run(['cargo', 'build', '--release', '--locked', '--offline', '--manifest-path',
                    str(project / 'Cargo.toml')], check=True)
    print(project / 'target/release/sysml-scale-benchmark.exe')


def run(a):
    base, out = a.baseline_root.resolve(), a.output.resolve()
    scope = a.java_scope_classes.resolve()
    assert (scope / 'bench/FlatScopeProvider.class').is_file(), 'Missing corrected Java scope overlay'
    profiles = [p.split('=', 1) if '=' in p else [p, None] for p in a.profiles]
    assert all(exe or label == 'java' for label, exe in profiles), 'Profiles are label=executable or java'
    spec = importlib.util.spec_from_file_location('baseline_runner', base / 'run.py')
    runner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(runner)
    original = runner.command
    current = {}

    def command(profile, folder, manifest):
        if current['exe']:
            return [current['exe'], 'text', str(folder), str(manifest['objects']), manifest['checksum']]
        cmd = original('xtext', folder, manifest)
        cmd[0] = str(a.jdk.resolve() / 'bin/java.exe')
        cmd[cmd.index('-cp') + 1] = str(scope) + os.pathsep + cmd[cmd.index('-cp') + 1]
        return cmd

    runner.command = command
    folder = base / 'inputs' / f'n{a.size}-d{a.degree}'
    manifest = json.loads((folder / 'manifest.json').read_text())
    out.mkdir(parents=True, exist_ok=False)
    rows = {label: [] for label, _ in profiles}
    for repeat in range(a.repeats):
        for label, exe in (profiles if repeat % 2 == 0 else profiles[::-1]):
            current['exe'] = str(Path(exe).resolve()) if exe else None
            dest = out / f'{label}-r{repeat}'
            dest.mkdir()
            row = runner.once('intersect-text' if exe else 'xtext', folder, manifest, dest, a.timeout)
            assert row['status'] == 'ok', (label, row.get('error'), row['stderr_tail'][-400:])
            event = {e['phase']: e for e in row['events']}
            rows[label].append({
                'first_seconds': event['cold_loaded']['pipeline_ms'] / 1000,
                'second_seconds': event['warm_loaded']['pipeline_ms'] / 1000,
                'first_retained_rss_bytes': event['cold_retained']['memory']['rss_bytes'],
                'second_retained_rss_bytes': event['warm_retained']['memory']['rss_bytes'],
                'first_load_peak_rss_bytes': event['cold_loaded']['memory']['peak_rss_bytes'],
                'lifetime_peak_rss_bytes': row['observed_peak_rss_bytes'],
            })
            print(label, repeat, rows[label][-1], flush=True)
    summary = {}
    for label, _ in profiles:
        summary[label] = {}
        for key in rows[label][0]:
            values = sorted(row[key] for row in rows[label])
            summary[label][key] = {'min': values[0], 'median': statistics.median(values), 'max': values[-1]}
    (out / 'summary.json').write_text(json.dumps({
        'case': folder.name, 'repeats': a.repeats, 'profiles': dict(profiles), 'rows': rows, 'summary': summary,
        'note': 'Fresh serial processes in alternating order. Fewer than five repeats is a smoke comparison.',
    }, indent=1) + '\n', encoding='utf-8')
    for label, values in summary.items():
        print(label)
        for key, value in values.items():
            scale = 2**30 if key.endswith('bytes') else 1
            print(f"  {key:28s} min {value['min']/scale:8.3f} median {value['median']/scale:8.3f} max {value['max']/scale:8.3f}")


def main():
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = p.add_subparsers(dest='action', required=True)
    b = sub.add_parser('build')
    for name in ['model-root', 'text-root', 'project']:
        b.add_argument('--' + name, type=Path, required=True)
    b.add_argument('--lock', type=Path, required=True,
                   help='Lockfile seeding a new consumer project')
    r = sub.add_parser('run')
    for name in ['baseline-root', 'jdk', 'java-scope-classes', 'output']:
        r.add_argument('--' + name, type=Path, required=True)
    r.add_argument('--size', type=int, default=1000000)
    r.add_argument('--degree', type=int, default=8)
    r.add_argument('--repeats', type=int, default=5)
    r.add_argument('--timeout', type=int, default=400)
    r.add_argument('profiles', nargs='+')
    a = p.parse_args()
    (build if a.action == 'build' else run)(a)


if __name__ == '__main__':
    main()
