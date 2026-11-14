"""Execute staged public consumers; no source checkout is needed by this payload."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time

from stage_text_packages import embedded_references, io_path, literal, process_path
from release_notices import inventory


def digest(path):
    return hashlib.sha256(io_path(path).read_bytes()).hexdigest()


def clean_environment(output):
    env = os.environ.copy()
    for key in list(env):
        if key.startswith(('JAVA_HOME', 'JDK_HOME')):
            del env[key]
    original = env.get('PATH', '')
    retained = [part for part in original.split(os.pathsep)
                if part and shutil.which('java', path=part) is None]
    # On Linux Java may share /usr/bin with the linker. Preserve only named
    # build tools through temporary aliases; no system installation is changed.
    aliases = output / 'host-tools'
    aliases.mkdir()
    if os.name != 'nt':
        for name in ('cargo', 'rustc', 'rustdoc', 'rustup', 'rustfmt', 'cargo-clippy',
                     'clippy-driver', 'node', 'git', 'cc', 'gcc', 'clang', 'as', 'ld',
                     'ld.lld', 'ar', 'sh', 'bash', 'uname', 'pkg-config', 'python3'):
            found = shutil.which(name, path=original)
            if found:
                (aliases / name).symlink_to(found)
        retained.insert(0, str(aliases))
    env['PATH'] = os.pathsep.join(retained)
    if shutil.which('java', path=env['PATH']):
        raise ValueError('Java remains available to verification processes')
    return env


def audit(stage, ledger):
    count = 0
    literals = 0
    for group in ('packages', 'consumers'):
        for name, entry in ledger[group].items():
            owner = stage / group / name
            queue = []
            for row in entry['files']:
                path = owner / row['staged']
                if digest(path) != row['staged_sha256']:
                    raise ValueError(f'Staged input changed: {path}')
                count += 1
                if path.suffix == '.rs' and not row['staged'].startswith('packaged-inputs/'):
                    queue.append(path)
            if digest(owner / 'Cargo.lock') != entry['lock_sha256']:
                raise ValueError(f'Staged lock changed: {name}')
            seen = set()
            while queue:
                path = process_path(queue.pop())
                if path in seen:
                    continue
                seen.add(path)
                for match in embedded_references(io_path(path).read_bytes().decode('utf-8')):
                    target = process_path(path.parent / literal(match['literal']))
                    if not target.is_relative_to(owner) or not io_path(target).is_file():
                        raise ValueError(f'Embedded/module input escapes package: {path}: {target}')
                    literals += 1
                    if target.suffix == '.rs':
                        queue.append(target)
    for name, expected in ledger['verification'].items():
        if digest(stage / 'verification' / name) != expected:
            raise ValueError('Verification payload changed: ' + name)
    vendor = ledger.get('vendor')
    if vendor:
        if digest(stage / '.cargo/config.toml') != vendor['config_sha256']:
            raise ValueError('Vendored Cargo source configuration changed')
        for name, expected in vendor['files'].items():
            if digest(stage / name) != expected:
                raise ValueError('Vendored source changed: ' + name)
    if ledger.get('notices'):
        for name, expected in ledger['notices'].items():
            if digest(stage / name) != expected:
                raise ValueError('Notice inventory changed: ' + name)
        recorded = json.loads((stage / 'third-party-notices.json').read_text(encoding='utf-8'))
        if recorded != inventory(stage, ledger):
            raise ValueError('Notice inventory does not match staged inputs')
    return {'files': count, 'literal_inputs': literals, 'escapes': 0,
            'vendored_files': len(vendor['files']) if vendor else 0}


def original_source_state(ledger):
    """A Text stage also depends on the original Model Git checkout/cache.

    Old receipts can still run non-strict checks, but cannot prove all original
    sources unavailable because they did not record every source repository.
    """
    roots = ledger.get('source_roots')
    complete = isinstance(roots, list) and bool(roots) and all(isinstance(p, str) and p for p in roots)
    recorded = set(roots) if complete else {ledger['source_root']}
    required = {ledger['source_root']}
    for group in ('packages', 'consumers'):
        for entry in ledger.get(group, {}).values():
            source = entry.get('source_root')
            if not isinstance(source, str) or not source:
                complete = False
            else:
                required.add(source)
    complete = complete and recorded == required
    present = [root for root in sorted(recorded | required) if Path(root).exists()]
    workspace = os.environ.get('GITHUB_WORKSPACE')
    if workspace and (Path(workspace) / 'Cargo.toml').is_file():
        present.append(workspace)
    return {'provenance_complete': bool(complete), 'present_roots': sorted(set(present)),
            'original_workspace_unavailable': bool(complete and not present)}


def verify(stage, output, report_root, require_original_unavailable):
    stage = process_path(stage)
    output = process_path(output)
    if output.exists() or not output.is_relative_to(stage):
        raise ValueError('Use a new verification output directory inside the stage')
    ledger = json.loads((stage / 'staging-receipt.json').read_text(encoding='utf-8'))
    sources = original_source_state(ledger)
    unavailable = sources['original_workspace_unavailable']
    if require_original_unavailable and sources['present_roots']:
        raise ValueError('Original workspace is available; cannot claim isolated execution')
    if require_original_unavailable and not sources['provenance_complete']:
        raise ValueError('Original source provenance is incomplete; restage before isolated execution')
    if (stage / 'toolkit').exists():
        raise ValueError('Toolkit must not be present in the verification payload')
    before = audit(stage, ledger)
    output.mkdir(parents=True)
    env = clean_environment(output)
    env['CARGO_TARGET_DIR'] = str(stage / 'build')
    env['CARGO_NET_OFFLINE'] = 'true'
    report_root = process_path(report_root or stage / 'reports')
    env['INTERSECT_TEST_REPORTS'] = str(report_root)
    receipt = {'stage': str(stage), 'source_commit': ledger.get('source_commit'),
               'source_dirty': ledger.get('source_dirty'),
               'original_sources': sources,
               'original_workspace_unavailable': unavailable, 'java_on_path': False,
               'toolkit_present': False, 'input_audit': before, 'commands': []}
    receipt['environment'] = {'platform': platform.platform(), 'machine': platform.machine(),
                              'python': platform.python_version()}
    for tool in ('rustc', 'cargo', 'node'):
        receipt['environment'][tool] = subprocess.check_output(
            [tool, '--version'], cwd=stage, env=env, text=True).strip()
    def run(name, args):
        print('Running ' + name, flush=True)
        started = time.monotonic()
        log = output / (name + '.log')
        with log.open('wb') as stream:
            result = subprocess.run(args, cwd=stage, env=env, stdout=stream, stderr=subprocess.STDOUT)
        row = {'name': name, 'command': args, 'exit_code': result.returncode,
               'seconds': round(time.monotonic() - started, 1), 'log_sha256': digest(log)}
        receipt['commands'].append(row)
        (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
        print(json.dumps(row), flush=True)
        if result.returncode:
            raise ValueError(log.read_text(encoding='utf-8')[-6000:])
    for name in ledger['consumers']:
        manifest = str(stage / 'consumers' / name / 'Cargo.toml')
        command = ['cargo', 'metadata', '--manifest-path', manifest, '--format-version', '1', '--locked', '--offline']
        metadata = json.loads(subprocess.check_output(command, cwd=stage, env=env))
        forbidden = {'intersect-library', 'intersect-web', 'intersect-text-acceptance'}
        if name == 'model-only':
            forbidden.add('intersect-text')
        for package in metadata['packages']:
            if package['name'] in forbidden:
                raise ValueError('Forbidden consumer dependency: ' + package['name'])
            if package['source'] is None or ledger.get('vendor'):
                if not process_path(package['manifest_path']).is_relative_to(stage):
                    raise ValueError('Resolved input escapes stage: ' + package['manifest_path'])
        run(name + '-clippy', ['cargo', 'clippy', '--manifest-path', manifest,
                              '--all-targets', '--locked', '--offline', '--', '-D', 'warnings'])
        report = report_root / 'xtext-separation' / (stage.name + '-' + output.name + '-' + name)
        run(name + '-native', [sys.executable, '-B', str(stage / 'verification/test_report.py'),
                              'run', '--out', str(report), '--no-server', '--', '--manifest-path', manifest,
                              '--profile', 'test-fast', '--lib', '--locked', '--offline'])
        summary = json.loads((report / 'summary.json').read_text(encoding='utf-8'))
        receipt.setdefault('native', {})[name] = {'totals': summary['totals'], 'report': str(report)}
        run(name + '-wasm-build', ['cargo', 'build', '--manifest-path', manifest, '--lib',
                                  '--target', 'wasm32-unknown-unknown', '--locked', '--offline'])
        artifact = stage / 'build/wasm32-unknown-unknown/debug' / (
            'intersect_' + name.replace('-', '_') + '_consumer.wasm')
        runner = 'test_model_boundary_wasm.mjs' if name == 'model-only' else 'test_text_boundary_wasm.mjs'
        run(name + '-wasm-execute', ['node', str(stage / 'verification' / runner), str(artifact)])
        receipt.setdefault('wasm', {})[name] = {'artifact': artifact.name,
                                               'bytes': artifact.stat().st_size,
                                               'sha256': digest(artifact)}
    after = audit(stage, ledger)
    if before != after:
        raise ValueError('Staged inputs changed during verification')
    receipt['postguard'] = 'pass'
    (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n', encoding='utf-8')
    print('Staged native/WASM public consumers passed; original unavailable: ' + str(unavailable), flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--stage', type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--report-root', type=Path)
    parser.add_argument('--require-original-unavailable', action='store_true')
    args = parser.parse_args()
    verify(args.stage, args.output, args.report_root, args.require_original_unavailable)


if __name__ == '__main__':
    main()
