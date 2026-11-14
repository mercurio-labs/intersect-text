"""Stage standalone model/Xtext packages with an auditable embedded-input closure.

This prepares distributions, not a claim that staged consumers have passed.
Every attempt uses a new directory. No original checkout path is retained in
dependency specifications or literal embedded/module inputs.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib
from urllib.parse import quote, urlsplit
from release_notices import add_notice_inputs, write_index


ROOT = Path(__file__).resolve().parent.parent
PACKAGES = ("intersect-build-support", "intersect-model", "intersect-text")
CONSUMERS = ("model-only", "text-only")
EMBED = re.compile(
    r'(?P<prefix>include(?:_str|_bytes)?!\s*\(\s*|#\[\s*path\s*=\s*)'
    r'(?P<literal>r(?P<hashes>\#*)".*?"(?P=hashes)|"(?:\\.|[^"\\])*")',
    re.DOTALL,
)
EMBED_PREFIX = re.compile(r'include(?:_str|_bytes)?!\s*\(\s*|#\[\s*path\s*=\s*')
RUST_STRING = re.compile(r'(?:b|c)?r(?P<hashes>\#*)".*?"(?P=hashes)|(?:b|c)?"(?:\\.|[^"\\])*"', re.DOTALL)
RUST_CHAR = re.compile(r"'(?:\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'")


def io_path(path):
    """Use long Windows paths when distributing deeply nested oracle evidence."""
    path = path.absolute()
    if os.name == 'nt' and not str(path).startswith('\\\\?\\'):
        return Path('\\\\?\\' + str(path))
    return path


def embedded_references(text):
    """Find real literal embeds; scanner examples in comments/strings are data."""
    offset = 0
    while offset < len(text):
        if text.startswith('//', offset):
            end = text.find('\n', offset + 2)
            offset = len(text) if end < 0 else end + 1
        elif text.startswith('/*', offset):
            depth = 1
            offset += 2
            while depth and offset < len(text):
                if text.startswith('/*', offset):
                    depth += 1
                    offset += 2
                elif text.startswith('*/', offset):
                    depth -= 1
                    offset += 2
                else:
                    offset += 1
            if depth:
                raise ValueError('Unterminated Rust block comment')
        elif EMBED_PREFIX.match(text, offset):
            match = EMBED.match(text, offset)
            if not match:
                raise ValueError('Embedded/module input must use a plain string literal')
            yield match
            offset = match.end()
        else:
            quoted = RUST_STRING.match(text, offset) or RUST_CHAR.match(text, offset)
            offset = quoted.end() if quoted else offset + 1


def relocate_embeds(text, replace):
    parts = []
    offset = 0
    for match in embedded_references(text):
        parts.extend([text[offset:match.start()], replace(match)])
        offset = match.end()
    parts.append(text[offset:])
    return ''.join(parts)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def standalone_readme(text, source, owner, root, revision, repository='https://github.com/mercurio-labs/intersect'):
    """Keep workspace documentation links usable after package extraction."""
    def replace(match):
        target = match['target']
        parsed = urlsplit(target)
        if parsed.scheme or parsed.netloc or not parsed.path:
            return match.group(0)
        path = (source.parent / parsed.path).resolve()
        if path.is_relative_to(owner.resolve()):
            return match.group(0)
        if not path.is_relative_to(root.resolve()) or not path.is_file():
            raise ValueError(f'Readme link escapes checkout or is missing: {source}: {target}')
        permalink = (repository.rstrip('/') + '/blob/' + revision + '/'
                     + quote(path.relative_to(root.resolve()).as_posix(), safe='/'))
        if parsed.fragment:
            permalink += '#' + parsed.fragment
        return match['prefix'] + permalink + ')'
    return re.sub(r'(?P<prefix>\[[^\]\n]+\]\()(?P<target>[^\s)]+)\)', replace, text)


def literal(text):
    if text.startswith('r'):
        start = text.index('"') + 1
        return text[start:len(text) - start + 1]
    return json.loads(text)


def toml_value(value):
    if isinstance(value, str):
        return json.dumps(value)
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, (int, float)):
        return str(value)
    if isinstance(value, list):
        return '[' + ', '.join(map(toml_value, value)) + ']'
    if isinstance(value, dict):
        return '{ ' + ', '.join(json.dumps(k) + ' = ' + toml_value(v)
                               for k, v in value.items()) + ' }'
    raise ValueError(f'Unsupported manifest value: {type(value).__name__}')


def manifest_text(data):
    lines = []
    for section, values in data.items():
        if not isinstance(values, dict):
            lines.append(json.dumps(section) + ' = ' + toml_value(values))
    for section, values in data.items():
        if isinstance(values, dict):
            lines += ['', '[' + json.dumps(section) + ']']
            lines += [json.dumps(k) + ' = ' + toml_value(v) for k, v in values.items()]
    text = '\n'.join(lines) + '\n'
    if tomllib.loads(text) != data:
        raise ValueError('Manifest normalization changed values')
    return text


def standalone_manifest(data, workspace, patches=None):
    data = copy.deepcopy(data)
    for key, value in list(data['package'].items()):
        if isinstance(value, dict) and value.get('workspace'):
            data['package'][key] = workspace['package'][key]
    def dependencies(tables):
        for kind in ('dependencies', 'dev-dependencies', 'build-dependencies'):
            for alias, value in list(tables.get(kind, {}).items()):
                if isinstance(value, str):
                    continue
                if value.get('workspace'):
                    inherited = workspace['dependencies'][alias]
                    inherited = {'version': inherited} if isinstance(inherited, str) else inherited
                    merged = dict(inherited)
                    merged.update({k: v for k, v in value.items() if k != 'workspace'})
                    if 'features' in inherited and 'features' in value:
                        merged['features'] = sorted(set(inherited['features'] + value['features']))
                    value = merged
                else:
                    value = dict(value)
                if 'path' in value:
                    if 'version' not in value:
                        raise ValueError(f'Unversioned package dependency: {alias}')
                    del value['path']
                if 'git' in value:
                    if value.get('package', alias) not in PACKAGES or 'version' not in value:
                        raise ValueError(f'Unstaged Git dependency: {alias}')
                    for key in ('git', 'rev', 'tag', 'branch'):
                        value.pop(key, None)
                tables[kind][alias] = value
    dependencies(data)
    for target in data.get('target', {}).values():
        dependencies(target)
    data['workspace'] = {'resolver': '3'}
    # Controlled local overrides; published dependency requirements stay versioned.
    if patches is None:
        patches = {name: {'path': '../' + name} for name in PACKAGES}
    if patches:
        data['patch'] = {'crates-io': patches}
    else:
        data.pop('patch', None)
    data['package']['license-file'] = 'LICENSE'
    data['package'].pop('license', None)
    # These are GitHub source-bundle artifacts, not registry publish candidates.
    data['package']['publish'] = False
    return data


def tracked_files(root=None):
    root = root or ROOT
    result = subprocess.check_output(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=root)
    return {root / os.fsdecode(name) for name in result.split(b'\0') if name}


def fixture_owner(path, root=None):
    root = root or ROOT
    parts = path.relative_to(root).parts
    if not parts or parts[0] != 'fixtures':
        return None
    depth = 3 if len(parts) > 3 and parts[1] in ('xtext', 'rcore') else 2
    return root.joinpath(*parts[:depth])


def package_inputs(name, tracked, package=None, root=None):
    root = root or ROOT
    package = package or root / 'crates' / name
    owned = {path for path in tracked if path.is_relative_to(package) and io_path(path).is_file()}
    mapping = {path.resolve(): path.relative_to(package) for path in owned}
    queue = [path for path in mapping if path.suffix == '.rs']
    scanned = set()
    references = {}
    while queue:
        path = queue.pop()
        if path in scanned:
            continue
        scanned.add(path)
        references[path] = []
        for match in embedded_references(io_path(path).read_bytes().decode('utf-8')):
            target = (path.parent / literal(match['literal'])).resolve(strict=True)
            if not io_path(target).is_file() or not target.is_relative_to(root.resolve()):
                raise ValueError(f'Input escapes checkout or is not a file: {path}: {target}')
            references[path].append(target)
            if target not in mapping:
                relative = target.relative_to(root.resolve())
                mapping[target] = Path('packaged-inputs') / relative
                if target.suffix == '.rs':
                    queue.append(target)
    # Carry the entire authoritative evidence unit, including notices and cached
    # oracle results. Copies are distribution artifacts, not new golden owners.
    owners = {owner for source in mapping if (owner := fixture_owner(source, root)) is not None}
    for path in tracked:
        if any(path.is_relative_to(owner) for owner in owners) and io_path(path).is_file():
            resolved = path.resolve()
            mapping.setdefault(resolved, Path('packaged-inputs') / path.relative_to(root))
    add_notice_inputs(mapping, {path.resolve() for path in tracked}, root.resolve())
    return mapping, references


def process_path(path):
    value = str(path)
    return Path(value[4:] if value.startswith('\\\\?\\') else value).resolve()


def resolve_staged_locks(output, ledger, seed_versions):
    """Resolve each root independently; local dependencies must stay in the stage."""
    metadata_dir = output / 'metadata'
    metadata_dir.mkdir()
    for group in ('packages', 'consumers'):
        for name, entry in ledger[group].items():
            manifest = process_path(output / group / name / 'Cargo.toml')
            command = ['cargo', 'metadata', '--manifest-path', str(manifest),
                       '--format-version', '1', '--offline']
            run = subprocess.run(command, cwd=process_path(output), capture_output=True)
            (metadata_dir / f'{group}-{name}.log').write_bytes(run.stderr)
            if run.returncode:
                raise ValueError(f'Cannot resolve {group}/{name}: {run.stderr.decode("utf-8")}')
            metadata = json.loads(run.stdout)
            local = []
            for package in metadata['packages']:
                if package['source'] is None:
                    path = process_path(package['manifest_path'])
                    if not path.is_relative_to(process_path(output)):
                        raise ValueError(f'Dependency escapes stage: {path}')
                    local.append(package['name'])
                elif (package['name'], package['version'], package['source']) not in seed_versions:
                    raise ValueError(f'Unrequested registry version change: {package["name"]} {package["version"]}')
            allowed = {'intersect-build-support', 'intersect-model'}
            if name not in ('intersect-build-support', 'intersect-model', 'model-only'):
                allowed.add('intersect-text')
            if group == 'consumers':
                allowed.add('intersect-' + name + '-consumer')
            if set(local) - allowed:
                raise ValueError(f'Unexpected local dependency of {name}: {set(local) - allowed}')
            verified = subprocess.run(command + ['--locked'], cwd=process_path(output), capture_output=True, check=True)
            (metadata_dir / f'{group}-{name}.json').write_bytes(verified.stdout)
            entry['resolved_local_packages'] = sorted(local)
            entry['lock_sha256'] = sha(io_path(manifest.with_name('Cargo.lock')).read_bytes())
            for row in entry['files']:
                if row['staged'] == 'Cargo.lock':
                    row['staged_sha256'] = entry['lock_sha256']
                    row['normalization'] = 'locked stage-local dependency resolution; original and workspace seed hashes retained'


def vendor_dependencies(output, ledger):
    vendor = output / 'vendor'
    consumer = 'text-only' if 'text-only' in ledger['consumers'] else 'model-only'
    command = ['cargo', 'vendor', '--manifest-path', str(process_path(output / 'consumers' / consumer / 'Cargo.toml')),
               '--locked', '--offline', '--versioned-dirs', str(process_path(vendor))]
    # A consumer's lock omits optional dependencies of its libraries. Retain
    # each shipped package's full locked graph so its declared features work
    # with the bundle's offline source replacement too.
    for name in sorted(ledger['packages']):
        command += ['--sync', str(process_path(output / 'packages' / name / 'Cargo.toml'))]
    run = subprocess.run(command, cwd=process_path(output), capture_output=True)
    (output / 'metadata/vendor.log').write_bytes(run.stderr)
    if run.returncode:
        raise ValueError('Vendoring failed: ' + run.stderr.decode('utf-8'))
    config = b'[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "vendor"\n[net]\noffline = true\n'
    (output / '.cargo').mkdir()
    (output / '.cargo/config.toml').write_bytes(config)
    files = {path.relative_to(output).as_posix(): sha(path.read_bytes())
             for path in vendor.rglob('*') if path.is_file()}
    ledger['vendor'] = {'files': files, 'config_sha256': sha(config)}
    for group in ('packages', 'consumers'):
        for name in ledger[group]:
            command = ['cargo', 'metadata', '--manifest-path', str(process_path(output / group / name / 'Cargo.toml')),
                       '--format-version', '1', '--all-features', '--locked', '--offline']
            result = subprocess.run(command, cwd=process_path(output), capture_output=True, check=True)
            metadata = json.loads(result.stdout)
            for package in metadata['packages']:
                if not process_path(package['manifest_path']).is_relative_to(process_path(output)):
                    raise ValueError('Vendored dependency escapes stage: ' + package['manifest_path'])
            (output / 'metadata' / f'vendored-{name}.json').write_bytes(result.stdout)


def source_workspace(package):
    for directory in (package, *package.parents):
        manifest = directory / 'Cargo.toml'
        if manifest.is_file() and 'workspace' in tomllib.loads(manifest.read_text(encoding='utf-8')):
            return directory
    raise ValueError(f'No source workspace for {package}')


def resolved_targets():
    local = {name: ROOT / 'crates' / name for name in PACKAGES
             if (ROOT / 'crates' / name / 'Cargo.toml').is_file()}
    owns_text = 'intersect-text' in local
    if owns_text and 'intersect-model' not in local:
        command = ['cargo', 'metadata', '--format-version', '1', '--locked', '--offline']
        metadata = json.loads(subprocess.check_output(command, cwd=ROOT))
        for name in ('intersect-model', 'intersect-build-support'):
            packages = [p for p in metadata['packages'] if p['name'] == name]
            if len(packages) != 1:
                raise ValueError(f'Require exactly one resolved {name} source')
            local[name] = process_path(Path(packages[0]['manifest_path']).parent)
    required = ('intersect-build-support', 'intersect-model') + (('intersect-text',) if owns_text else ())
    if set(local) != set(required):
        raise ValueError('Missing or unexpected owned public packages')
    targets = []
    for name in required:
        owner = process_path(local[name])
        targets.append(('packages', name, owner, source_workspace(owner)))
    for name in (('model-only', 'text-only') if owns_text else ('model-only',)):
        repository = ROOT if (ROOT / 'tests' / name / 'Cargo.toml').is_file() else source_workspace(local['intersect-model'])
        owner = repository / 'tests' / name
        if not (owner / 'Cargo.toml').is_file():
            raise ValueError(f'Missing public consumer source: {name}; Git-based source packages are required')
        targets.append(('consumers', name, owner, repository))
    return targets


def stage(output, vendor=False):
    output = output.resolve()
    if output.exists():
        raise ValueError(f'Use a new output directory: {output}')
    if output.is_relative_to(ROOT / 'crates'):
        raise ValueError('Distribution output cannot be inside source packages')
    output = io_path(output)
    targets = resolved_targets()
    roots = {repository for _, _, _, repository in targets}
    tracked = {repository: tracked_files(repository) for repository in roots}
    workspaces = {repository: tomllib.loads((repository / 'Cargo.toml').read_text(encoding='utf-8'))['workspace'] for repository in roots}
    plans = {(group, name): package_inputs(name, tracked[repository], owner, repository)
             for group, name, owner, repository in targets}
    sources = {path: io_path(path).read_bytes() for mapping, _ in plans.values() for path in mapping}
    output.mkdir(parents=True)
    ledger = {'source_root': str(ROOT), 'source_roots': sorted(str(process_path(path)) for path in roots),
              'scope': 'staged inputs; execution acceptance is separate',
              'packages': {}, 'consumers': {}, 'original_inputs': len(sources)}
    ledger['source_commit'] = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    ledger['source_dirty'] = bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True).strip())
    for group, name, owner, repository in targets:
        workspace = workspaces[repository]
        revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repository, text=True).strip()
        mapping, references = plans[(group, name)]
        destination = output / group / name
        destination.mkdir(parents=True)
        rows = []
        for source, relative in sorted(mapping.items()):
            target = destination / relative
            if not target.resolve().is_relative_to(destination):
                raise ValueError(f'Destination escapes package: {target}')
            target.parent.mkdir(parents=True, exist_ok=True)
            original = sources[source]
            content = original
            rewrites = []
            if source.name == 'README.md' and source.is_relative_to(owner):
                content = standalone_readme(original.decode('utf-8'), source, owner, repository,
                                            revision, workspace['package'].get('repository', 'https://github.com/mercurio-labs/intersect')).encode('utf-8')
            if source in references:
                def replace(match):
                    embedded = (source.parent / literal(match['literal'])).resolve(strict=True)
                    staged = destination / mapping[embedded]
                    rewritten = Path(os.path.relpath(staged, target.parent)).as_posix()
                    if rewritten == literal(match['literal']):
                        return match.group(0)
                    rewrites.append({'previous': literal(match['literal']), 'staged': rewritten,
                                     'target_sha256': sha(sources[embedded])})
                    return match['prefix'] + json.dumps(rewritten)
                content = relocate_embeds(original.decode('utf-8'), replace).encode('utf-8')
            target.write_bytes(content)
            rows.append({'original': source.relative_to(repository.resolve()).as_posix(),
                         'staged': relative.as_posix(), 'original_sha256': sha(original),
                         'staged_sha256': sha(content), 'literal_relocations': rewrites})
            if source.name == 'README.md' and content != original:
                rows[-1]['normalization'] = 'workspace documentation links become source-commit permalinks'
        required = ('intersect-build-support',) if name == 'intersect-model' else PACKAGES
        if name == 'intersect-build-support':
            required = ()
        elif name == 'model-only':
            required = ('intersect-build-support', 'intersect-model')
        prefix = '../' if group == 'packages' else '../../packages/'
        patches = {dependency: {'path': prefix + dependency} for dependency in required if dependency != name}
        manifest = standalone_manifest(tomllib.loads((owner / 'Cargo.toml').read_text(encoding='utf-8')), workspace, patches)
        normalized = manifest_text(manifest).encode('utf-8')
        (destination / 'Cargo.toml').write_bytes(normalized)
        manifest_row = next(row for row in rows if row['staged'] == 'Cargo.toml')
        manifest_row['staged_sha256'] = sha(normalized)
        manifest_row['normalization'] = 'explicit versions and controlled stage-local overrides'
        license_bytes = (repository / 'LICENSE').read_bytes()
        (destination / 'LICENSE').write_bytes(license_bytes)
        rows.append({'original': 'LICENSE', 'staged': 'LICENSE',
                     'original_sha256': sha(license_bytes), 'staged_sha256': sha(license_bytes),
                     'literal_relocations': []})
        # Seed the independently resolved lock without upgrading existing versions.
        (destination / 'Cargo.lock').write_bytes((repository / 'Cargo.lock').read_bytes())
        ledger[group][name] = {'files': rows, 'manifest': manifest,
                                    'seed_lock_sha256': sha((repository / 'Cargo.lock').read_bytes()),
                                    'source_root': str(process_path(repository)),
                                    'source_commit': revision,
                                    'execution': 'not yet run'}
    for source, original in sources.items():
        if io_path(source).read_bytes() != original:
            raise ValueError(f'Source changed during staging: {source}')
    seed_versions = {(p['name'], p['version'], p.get('source')) for repository in roots
                     for p in tomllib.loads((repository / 'Cargo.lock').read_text(encoding='utf-8'))['package']}
    resolve_staged_locks(output, ledger, seed_versions)
    if vendor:
        vendor_dependencies(output, ledger)
    write_index(output, ledger)
    verification = output / 'verification'
    verification.mkdir()
    ledger['verification'] = {}
    for name in ('test_report.py', 'stage_text_packages.py', 'verify_text_stage.py', 'release_notices.py',
                 'test_model_boundary_wasm.mjs', 'test_text_boundary_wasm.mjs'):
        if not (ROOT / 'tools' / name).is_file():
            continue
        content = (ROOT / 'tools' / name).read_bytes()
        (verification / name).write_bytes(content)
        ledger['verification'][name] = sha(content)
    (output / 'staging-receipt.json').write_text(json.dumps(ledger, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'output': str(output), 'original_inputs': len(sources),
                      'packaged_files': sum(len(p['files']) for group in ('packages', 'consumers')
                                            for p in ledger[group].values())}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--vendor', action='store_true', help='Package locked registry dependencies for offline workers')
    args = parser.parse_args()
    stage(args.output, args.vendor)


if __name__ == '__main__':
    main()
