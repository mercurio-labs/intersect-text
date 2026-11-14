"""Inventory repository ownership and required inputs; never award runtime support."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

from stage_text_packages import embedded_references, literal, fixture_owner, io_path

MODEL_PACKAGES = (
    'crates/intersect-build-support', 'crates/intersect-model',
    'crates/intersect-codegen', 'crates/intersect-run', 'crates/intersect-project-host',
    'tests/model-acceptance', 'tests/support', 'tests/rust-bindings',
    'tests/semantic-bindings', 'tests/model-only',
)
TEXT_PACKAGES = (
    'crates/intersect-text', 'crates/intersect-library', 'crates/intersect-web',
    'tests/text-acceptance', 'tests/resource-lifecycle', 'tests/model-text',
    'tests/text-only', 'tests/preparation-only', 'tests/contextual-entry-probe',
)
# These are pinned infrastructure snapshots in Text; Model is the sole source
# owner. They contain no language semantic implementation.
INFRASTRUCTURE = {
    'api_surface.py', 'test_api_surface.py',
    'api_contracts.py', 'test_api_contracts.py',
    'test_report.py', 'test_test_report.py', 'format_workspace.py',
    'qualify_support.py', 'test_qualify_support.py',
    'check_text_separation.py', 'test_text_separation_guard.py',
    'check_text_release_contract.py', 'test_text_release_contract.py',
    'stage_text_packages.py', 'test_stage_text_packages.py',
    'verify_text_stage.py', 'test_verify_text_stage.py',
    'definition_ledger.py', 'repository_layout.py', 'test_repository_layout.py',
    'prepare_repository_split.py', 'update_shared_tools.py',
    'test_model_boundary_wasm.mjs',
    'behavior_fixture_migration.py',
}
MODEL_EVIDENCE_SNAPSHOTS = {
    'crates/intersect-model/assets/fixtures/rcore-behavior-ast-baseline.json',
    'crates/intersect-model/assets/fixtures/rcore-behavior-compiler-baseline.json',
    'crates/intersect-model/assets/fixtures/rcore-lexical-baseline.json',
    'crates/intersect-model/assets/fixtures/rcore-structural-baseline.json',
    'crates/intersect-model/assets/fixtures/rcore-structural-factory-baseline.json',
    'crates/intersect-model/assets/fixtures/xtext/connector-prediction/program.json',
    'crates/intersect-model/src/definitions/compiler_enum_preparation_baseline.json',
    'crates/intersect-model/src/definitions/generic_inheritance_preparation_baseline.json',
    'crates/intersect-model/src/definitions/generic_runtime_members_baseline.json',
    'crates/intersect-model/src/definitions/generic_source_members_baseline.json',
    'crates/intersect-model/src/definitions/runtime_schema_baseline.json',
    'tools/compatibility/README.md',
    'tools/compatibility/Verify.java',
}
GENERATED_PACKAGES = {'tests/rust-bindings', 'tests/semantic-bindings', 'crates/intersect-web'}
REVIEWED_TOOLS = {
    'test_resource_lifecycle_wasm.mjs': ('text', 'executes the Text-owned resource-lifecycle integration harness'),
    'test_inferred_diagnostic_origins.mjs': ('text', 'executes inferred-grammar diagnostic Web requests'),
    'check_rcore_string_transport.mjs': ('text', 'compares persistent public JSON hosts; native Model string controls remain Model-owned'),
    'string_transport_controls.mjs': ('text', 'input codec used only by the combined public-host comparison'),
    'check_model_boundary.py': ('model', 'Model-only guard; Text names occur only in its forbidden dependency set'),
    'rcore_demo.mjs': ('text', 'invokes Web through rcore_host.mjs'),
    'rcore_construction_demo.mjs': ('text', 'invokes Web through rcore_host.mjs'),
    'rcore_operations_demo.mjs': ('text', 'invokes Web through rcore_host.mjs'),
}
GENERATED = re.compile(r'include(?:_str|_bytes)?!\s*\(\s*concat!\s*\(\s*env!\s*\(\s*"OUT_DIR"\s*\)\s*,\s*"(?P<file>/[^"\n]+)"\s*\)\s*\)')
MANIFEST_INPUT = re.compile(r'(?P<macro>include(?:_str|_bytes)?!)\s*\(\s*concat!\s*\(\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"(?P<suffix>/[^"\n]+)"\s*\)\s*\)')


def tracked(root):
    data = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=root)
    return {root / os.fsdecode(name) for name in data.split(b'\0') if name and io_path(root / os.fsdecode(name)).is_file()}


def initial_owner(relative, content=None):
    if relative in {'Cargo.toml', 'Cargo.lock', 'LICENSE', 'README.md', 'AGENTS.md',
                    '.gitignore', '.gitattributes', '.cargo/config.toml', 'rust-toolchain.toml',
                    'repository-checks.json'}:
        return {'model', 'text'}, 'independent root configuration'
    if relative in MODEL_EVIDENCE_SNAPSHOTS:
        return {'model', 'text'}, 'Model-owned immutable evidence snapshot, pinned by Text shared-model-inputs.json'
    if any(relative == prefix or relative.startswith(prefix + '/') for prefix in MODEL_PACKAGES):
        return {'model'}, 'Model package or harness'
    if any(relative == prefix or relative.startswith(prefix + '/') for prefix in TEXT_PACKAGES):
        return {'text'}, 'Text package or integration harness'
    if relative.startswith('toolkit/') or relative.startswith('experiments/'):
        return {'text'}, 'combined host or text experiment'
    if relative.startswith('benchmarks/'):
        return ({'text'} if relative.startswith('benchmarks/library/') else {'model'}), 'benchmark contract'
    if relative.startswith('fixtures/'):
        unit = relative.split('/')[1]
        if unit.startswith(('xtext', 'kerml', 'sysml')) or unit in {'native-pilot', 'prepared-language', 'basic-systems', 'compatibility'}:
            return {'text'}, 'text/language provenance unit'
        return {'model', 'text'}, 'immutable Model input snapshot used by integration consumers'
    if relative.startswith('tools/'):
        name = Path(relative).name
        if name in {'ecore_identifier.py', 'test_ecore_identifier.py', 'scalar_unicode.py', 'test_scalar_unicode.py'}:
            return {'model'}, 'Model-generated Unicode tables and their owning generator controls'
        if name in REVIEWED_TOOLS:
            owner, reason = REVIEWED_TOOLS[name]
            return {owner}, reason
        if name in INFRASTRUCTURE:
            return {'model', 'text'}, 'Model-owned pinned infrastructure snapshot'
        if name in {'text_definition_ledger.py', 'test_definition_ledger.py'}:
            return {'text'}, 'Text grammar inventory and independent oracle'
        if name.startswith(('test_xtext', 'xtext_', 'test_text', 'text_', 'test_model_text', 'basic_systems', 'test_basic_systems', 'verify_packaged')):
            return {'text'}, 'Text verification tool'
        if content and any(token in content for token in ('intersect-text', 'intersect_text', 'intersect-web', 'intersect_web', 'intersect-resource-lifecycle-tests', 'toolkit/')):
            return {'text'}, 'tool invokes a Text or combined host contract'
        return {'model'}, 'Model verification or generation tool'
    if relative.startswith('docs/'):
        name = Path(relative).name
        if name == 'shared-model-inputs.json':
            return {'text'}, 'Pinned immutable Model evidence snapshots needed by Text verifiers'
        if name in {'model-text-repository-split-plan.md', 'repository-split-state.json', 'repository-development.md', 'architecture.md', 'roadmap.md'} or relative.startswith('docs/repository-split/'):
            return {'model', 'text'}, 'split provenance or repository-specific entry point'
        if name.startswith(('xtext', 'text-', 'workbench', 'general-model-to-text', 'model-to-text', 'inference')) or name == 'definition-languages-ledger.json':
            return {'text'}, 'text API, host documentation or historical text evidence'
        return {'model'}, 'Model documentation or retained original history'
    if relative.startswith('.github/'):
        return {'model', 'text'}, 'repository-specific workflow configuration'
    if relative.startswith('.claude/'):
        return {'model', 'text'}, 'contributor guidance'
    raise ValueError(f'Unclassified tracked path: {relative}')


def scan_source(path, root, package):
    """Adapt only registered Cargo-generated mounts; other dynamic inputs fail."""
    source = io_path(path).read_text(encoding='utf-8')
    if not re.search(r'include(?:_str|_bytes)?!|#\[\s*path\s*=', source):
        return [], []
    generated = []
    for match in GENERATED.finditer(source):
        owner = package.relative_to(root).as_posix()
        if owner not in GENERATED_PACKAGES or not (package / 'build.rs').is_file():
            raise ValueError(f'Unregistered generated include: {path}')
        generated.append({'path': path.relative_to(root).as_posix(), 'producer': (package / 'build.rs').relative_to(root).as_posix(), 'output': match['file']})
    # The source itself is never rewritten. Build scripts remain owned inputs.
    adapted = GENERATED.sub('/* registered OUT_DIR output */', source)
    manifest_targets = []
    def manifest(match):
        target = (package / match['suffix'].lstrip('/')).resolve()
        manifest_targets.append(target)
        return '/* registered manifest-relative input */'
    adapted = MANIFEST_INPUT.sub(manifest, adapted)
    try:
        targets = [(path.parent / literal(match['literal'])).resolve() for match in embedded_references(adapted)]
    except ValueError as error:
        raise ValueError(f'{path}: {error}') from error
    targets += manifest_targets
    for target in targets:
        if not target.is_relative_to(root) or not io_path(target).is_file():
            raise ValueError(f'Missing or escaping required input: {path}: {target}')
    return targets, generated


def build(root):
    root = root.resolve()
    files = tracked(root)
    assignments = {}
    for path in sorted(files):
        relative = path.relative_to(root).as_posix()
        content = io_path(path).read_text(encoding='utf-8') if path.suffix in {'.py', '.mjs', '.js'} else None
        owners, reason = initial_owner(relative, content)
        assignments[relative] = {'owners': sorted(owners), 'reason': reason}
    generated = []
    # Every Rust file, including tests/examples, participates in the closure.
    for owner, packages in [('model', MODEL_PACKAGES), ('text', TEXT_PACKAGES)]:
        for relative in packages:
            package = root / relative
            if not package.is_dir():
                continue
            queue = [p for p in files if p.suffix == '.rs' and p.is_relative_to(package)]
            seen = set()
            units = set()
            while queue:
                path = queue.pop()
                if path in seen:
                    continue
                seen.add(path)
                targets, outputs = scan_source(path, root, package)
                generated += outputs
                for target in targets:
                    name = target.relative_to(root).as_posix()
                    if name not in assignments:
                        raise ValueError(f'Required input is not inventoried: {name}')
                    entry = assignments[name]
                    if owner not in entry['owners']:
                        if target.suffix == '.rs' and not name.startswith('fixtures/'):
                            raise ValueError(f'Cross-repository Rust module mount: {path}: {target}')
                        entry['owners'] = sorted(set(entry['owners']) | {owner})
                        entry['reason'] = 'required immutable input; source identity/provenance retained'
                    unit = fixture_owner(target, root)
                    if unit:
                        units.add(unit)
                    if target.suffix == '.rs':
                        queue.append(target)
            prefixes = tuple(unit.relative_to(root).as_posix() + '/' for unit in units)
            for name, entry in assignments.items():
                if prefixes and name.startswith(prefixes):
                    entry['owners'] = sorted(set(entry['owners']) | {owner})
    runtime_reads = []
    for path in sorted(files):
        if path.suffix not in {'.rs', '.py', '.mjs', '.js'}:
            continue
        for number, line in enumerate(io_path(path).read_text(encoding='utf-8').splitlines(), 1):
            if re.search(r'(?:read_to_string|read_dir|File::open|fs::read|readFile|read_text|read_bytes)\s*\(', line):
                runtime_reads.append({'path': path.relative_to(root).as_posix(), 'line': number,
                                      'expression': line.strip(), 'owners': assignments[path.relative_to(root).as_posix()]['owners']})
    units = {}
    for path in sorted(files):
        if path.relative_to(root).parts[0] != 'fixtures':
            continue
        unit = fixture_owner(path, root).relative_to(root).as_posix()
        units.setdefault(unit, []).append(path)
    provenance = {}
    for unit, members in units.items():
        digest = hashlib.sha256()
        notices = []
        for path in members:
            name = path.relative_to(root).as_posix()
            digest.update(name.encode('utf-8') + b'\0' + hashlib.sha256(io_path(path).read_bytes()).digest())
            if any(word in path.name.lower() for word in ('license', 'notice', 'provenance', 'readme', 'manifest', 'artifact')):
                notices.append(name)
        provenance[unit] = {'files': len(members), 'tree_sha256': digest.hexdigest(), 'notices_and_manifests': notices,
                            'owners': sorted({owner for path in members for owner in assignments[path.relative_to(root).as_posix()]['owners']})}
    return {'schema': 'intersect.repository-layout.v1', 'files': assignments,
            'generated_mounts': generated, 'runtime_read_sites': runtime_reads,
            'provenance_units': provenance,
            'scope': 'Required literal/generated input closure and runtime read-site inventory. Caller-provided runtime paths remain explicit host inputs; runtime execution is separate.'}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = build(args.root)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with gzip.open(args.output, 'wt', encoding='utf-8') as stream:
        json.dump(result, stream, sort_keys=True)
    print(json.dumps({'files': len(result['files']), 'model': sum('model' in x['owners'] for x in result['files'].values()),
                      'text': sum('text' in x['owners'] for x in result['files'].values()), 'registered_generated_mounts': len(result['generated_mounts'])}))


if __name__ == '__main__':
    main()
