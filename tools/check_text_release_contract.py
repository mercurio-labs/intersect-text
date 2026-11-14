"""Check release metadata and exact Model pins; never award runtime qualification.

Maintained manifests are the root plus crates/* and tests/* Cargo.toml files.
Historical fixture/experiment manifests are deliberately outside this contract.
"""
import json
from pathlib import Path
import re
import tomllib

MODEL_PACKAGES = {
    'intersect-model', 'intersect-build-support', 'intersect-codegen',
    'intersect-project-host', 'intersect-run', 'intersect-model-test-support',
    'intersect-rust-bindings-tests', 'intersect-semantic-bindings-tests',
}
DEPENDENCY_KINDS = ('dependencies', 'build-dependencies', 'dev-dependencies')


def full_revision(value):
    return isinstance(value, str) and re.fullmatch('[0-9a-f]{40}', value) is not None


def manifest_paths(root):
    return sorted({root / 'Cargo.toml', *root.glob('crates/*/Cargo.toml'),
                   *root.glob('tests/*/Cargo.toml')})


def dependency_tables(manifest):
    for kind in DEPENDENCY_KINDS:
        yield kind, manifest.get(kind, {})
    for target, tables in manifest.get('target', {}).items():
        for kind in DEPENDENCY_KINDS:
            yield f'target.{target}.{kind}', tables.get(kind, {})


def check_model_pins(root, contract):
    policy = contract.get('model_dependency', {})
    repository, revision = policy.get('repository'), policy.get('revision')
    if repository != 'https://github.com/mercurio-labs/intersect-model.git' or not full_revision(revision):
        return ['Text must record the canonical Model repository and full selected revision']
    manifests = {path: tomllib.loads(path.read_text(encoding='utf-8'))
                 for path in manifest_paths(root)}
    errors = []
    lock_roots = set()
    for path, manifest in manifests.items():
        # A standalone consumer's own workspace takes precedence over the root.
        workspace = next((parent for parent in path.parent.parents
                          if parent / 'Cargo.toml' in manifests
                          and 'workspace' in manifests[parent / 'Cargo.toml']), root)
        if 'workspace' in manifest:
            workspace = path.parent
        workspace_data = manifests[workspace / 'Cargo.toml'].get('workspace', {})
        lock_roots.add(workspace)
        tables = list(dependency_tables(manifest))
        if 'workspace' in manifest:
            tables.append(('workspace.dependencies', workspace_data.get('dependencies', {})))
        for section, dependencies in tables:
            for alias, declared in dependencies.items():
                spec = {'version': declared} if isinstance(declared, str) else dict(declared)
                if spec.get('workspace'):
                    inherited = workspace_data.get('dependencies', {}).get(alias)
                    if inherited is None:
                        errors.append(f'{path.relative_to(root)}: missing workspace dependency {alias}')
                        continue
                    inherited = {'version': inherited} if isinstance(inherited, str) else inherited
                    spec = {**inherited, **{k: v for k, v in spec.items() if k != 'workspace'}}
                package = spec.get('package', alias)
                if package not in MODEL_PACKAGES and spec.get('git') != repository:
                    continue
                location = f'{path.relative_to(root).as_posix()} [{section}] {alias}'
                if (spec.get('git') != repository or spec.get('rev') != revision
                        or any(key in spec for key in ('path', 'branch', 'tag'))):
                    errors.append(f'{location}: must use the recorded immutable Model source')
                if package == 'intersect-model' and spec.get('version') != contract['text']['model_version_requirement']:
                    errors.append(f'{location}: must use the recorded Model version requirement')
    expected = f'git+{repository}?rev={revision}#{revision}'
    for workspace in sorted(lock_roots):
        path = workspace / 'Cargo.lock'
        if not path.is_file():
            errors.append(f'{path.relative_to(root)}: missing maintained lockfile')
            continue
        packages = tomllib.loads(path.read_text(encoding='utf-8')).get('package', [])
        seen = set()
        for package in packages:
            name, source = package['name'], package.get('source', '')
            if name not in MODEL_PACKAGES and repository not in source:
                continue
            if source != expected:
                errors.append(f'{path.relative_to(root)}: {name} has a different resolved Model source')
            if name in seen:
                errors.append(f'{path.relative_to(root)}: duplicate Model package {name}')
            seen.add(name)
        for required in ('intersect-model', 'intersect-build-support'):
            if required not in seen:
                errors.append(f'{path.relative_to(root)}: missing resolved {required}')
    legacy = root / 'docs/xtext-compatibility.json'
    if legacy.is_file():
        record = json.loads(legacy.read_text(encoding='utf-8'))
        if record.get('status') != 'historical' or record.get('superseded_by') != 'text-compatibility.json':
            errors.append('Legacy Xtext compatibility must be labeled historical and point to text-compatibility.json')
    return errors


def check(root):
    owns_text = (root / 'crates/intersect-text/Cargo.toml').is_file()
    contract_path = 'docs/text-compatibility.json' if owns_text else 'docs/model-compatibility.json'
    contract = json.loads((root / contract_path).read_text(encoding='utf-8'))
    errors = []
    for name in ('model', 'text'):
        path = root / f'crates/intersect-{name}/Cargo.toml'
        if not path.is_file():
            continue
        manifest = tomllib.loads(path.read_text(encoding='utf-8'))
        version = manifest['package']['version']
        if not isinstance(version, str) or version != contract[name]['version']:
            errors.append(f'{name} must declare its own recorded release version')
        if name == 'text':
            model = manifest.get('dependencies', {}).get('intersect-model', {})
            requirement = model if isinstance(model, str) else model.get('version')
            inherited = isinstance(model, dict) and model.get('workspace')
            if inherited or requirement != contract['text']['model_version_requirement']:
                errors.append('Text must declare its own model compatibility requirement')
    if owns_text:
        frozen = json.loads((root / 'fixtures/xtext/inference-archive/ordinary-baseline.json').read_text(encoding='utf-8'))
        if contract['legacy_ordinary_compiler'] != frozen['manifest']['compiler']:
            errors.append('Legacy ordinary compiler descriptor differs from the frozen archive')
        errors.extend(check_model_pins(root, contract))
    revision = contract['model']['minimum_tested_revision']
    if not full_revision(revision):
        errors.append('Minimum tested model revision must be a complete commit SHA')
    return errors


def main():
    try:
        errors = check(Path(__file__).resolve().parent.parent)
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise SystemExit(f'Invalid release contract input: {error}') from error
    if errors:
        raise SystemExit('\n'.join(errors))
    print('Release metadata, maintained Model pins and locks are consistent. Runtime qualification is separate.')


if __name__ == '__main__':
    main()
