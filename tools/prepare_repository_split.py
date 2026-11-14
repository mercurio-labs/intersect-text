"""Prepare reviewed split trees in disposable clones; never rename/push remotes.

The clone must be below this checkout's ignored target/repository-split area.
Model history is retained. Run git-filter-repo separately on the Text clone,
retaining its commit map; this command only allocates the current tree.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

from repository_layout import INFRASTRUCTURE, MODEL_PACKAGES, TEXT_PACKAGES
from stage_text_packages import manifest_text, io_path

ROOT = Path(__file__).resolve().parents[1]
MODEL_URL = 'https://github.com/mercurio-labs/intersect-model.git'


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def prepare(clone, owner, allocation, model_revision):
    clone = clone.resolve()
    area = (ROOT / 'target/repository-split').resolve()
    if clone == area or not clone.is_relative_to(area) or not (clone / '.git').is_dir():
        raise ValueError('Only a disposable clone below target/repository-split may be changed')
    if len(model_revision) != 40 or any(c not in '0123456789abcdef' for c in model_revision):
        raise ValueError('Model dependency must use a complete immutable revision')
    if owner == 'model':
        subprocess.run(['git', 'cat-file', '-e', model_revision + '^{commit}'], cwd=clone, check=True)
    with gzip.open(allocation, 'rt', encoding='utf-8') as stream:
        layout = json.load(stream)
    files = layout['files']
    names = subprocess.check_output(['git', 'ls-files', '-z'], cwd=clone).decode('utf-8').split('\0')
    removed = []
    for name in filter(None, names):
        if name in files and owner in files[name]['owners']:
            continue
        target = (clone / name).resolve()
        if not target.is_relative_to(clone):
            raise ValueError('Deletion escapes disposable clone: ' + name)
        if io_path(target).is_file():
            io_path(target).unlink()
        removed.append(name)
    # Retain empty-directory-independent behavior: every selected file must exist.
    missing = [name for name, entry in files.items() if owner in entry['owners'] and not io_path(clone / name).is_file()]
    if missing:
        raise ValueError('Extraction omitted selected paths: ' + repr(missing[:20]))
    package_roots = MODEL_PACKAGES if owner == 'model' else TEXT_PACKAGES
    workspace_path = clone / 'Cargo.toml'
    workspace = tomllib.loads(workspace_path.read_text(encoding='utf-8'))
    workspace['workspace']['members'] = [name for name in workspace['workspace']['members'] if name in package_roots]
    workspace['workspace']['exclude'] = [name for name in workspace['workspace'].get('exclude', []) if (clone / name / 'Cargo.toml').is_file()]
    workspace['workspace']['package']['repository'] = 'https://github.com/mercurio-labs/intersect-' + owner
    if owner == 'model':
        workspace['workspace']['dependencies'].pop('intersect-text', None)
    workspace_path.write_text(manifest_text(workspace), encoding='utf-8')
    # Resolve each declared path using its old location. Foreign Model packages
    # become a single Git source identity, including host build/test support.
    external = []
    for relative in sorted(files):
        if Path(relative).name != 'Cargo.toml' or relative.startswith('fixtures/') or owner not in files[relative]['owners']:
            continue
        path = clone / relative
        data = tomllib.loads(path.read_text(encoding='utf-8'))
        changed = False
        def visit(table):
            nonlocal changed
            for key, value in list(table.items()):
                if key in ('dependencies', 'dev-dependencies', 'build-dependencies') and isinstance(value, dict):
                    for alias, spec in value.items():
                        if not isinstance(spec, dict):
                            continue
                        if spec.get('git') == MODEL_URL:
                            if spec.get('rev') != model_revision:
                                raise ValueError('Inconsistent Model Git revision: ' + str(path))
                            external.append({'manifest': path.relative_to(clone).as_posix(), 'package': spec.get('package', alias), 'revision': model_revision})
                        if 'path' not in spec:
                            continue
                        target = (path.parent / spec['path']).resolve()
                        if not target.is_relative_to(clone):
                            raise ValueError('Dependency escapes repository: ' + str(path))
                        relative = target.relative_to(clone).as_posix()
                        if (target / 'Cargo.toml').is_file():
                            continue
                        if owner != 'text' or relative not in MODEL_PACKAGES:
                            raise ValueError('Unallocated dependency: ' + relative)
                        spec.pop('path')
                        spec.update(git=MODEL_URL, rev=model_revision)
                        spec.setdefault('version', '0.1.0')
                        external.append({'manifest': path.relative_to(clone).as_posix(), 'package': spec.get('package', alias), 'revision': model_revision})
                        changed = True
                elif isinstance(value, dict):
                    visit(value)
        visit(data)
        if changed:
            path.write_text(manifest_text(data), encoding='utf-8')
    configuration = {
        'repository': owner,
        'definition_ledger': ('definition_ledger.py' if owner == 'model' else 'text_definition_ledger.py'),
        'host_wasm_controls': ([] if owner == 'model' else [
            ['resource-lifecycle', 'test_resource_lifecycle_wasm.mjs', 'intersect_resource_lifecycle_tests.wasm'],
            ['resource-interchange', 'test_resource_interchange_wasm.mjs', 'intersect_web.wasm'],
        ]),
        'scoped_packages': ({'model': ['intersect-model'], 'host': ['intersect-run', 'intersect-project-host']}
                            if owner == 'model' else
                            {'text': ['intersect-text'], 'host': ['intersect-library', 'intersect-web', 'intersect-text-acceptance', 'intersect-resource-lifecycle-tests', 'intersect-model-text-tests']}),
        'feature_controls': ([{'package': 'intersect-semantic-bindings-tests', 'features': 'rhai'}] if owner == 'model' else []),
        'examples': ([['intersect-model', name] for name in ('library', 'dynamic_library', 'rcore_model')] + [['intersect-rust-bindings-tests', 'generated_library']]
                     if owner == 'model' else [['intersect-text-acceptance', 'model_api']]),
    }
    write_json(clone / 'repository-checks.json', configuration)
    compatibility = json.loads((ROOT / 'docs/text-compatibility.json').read_text(encoding='utf-8'))
    if owner == 'model':
        write_json(clone / 'docs/model-compatibility.json', {'model': compatibility['model'], 'qualification': 'Independent Model release. Split acceptance is recorded in repository-split-state.json; hosted CI is user-skipped.'})
    else:
        compatibility['model']['minimum_tested_revision'] = model_revision
        compatibility['qualification'] = 'Pinned Model API candidate; execution status is recorded in repository-split-state.json. Hosted CI is explicitly skipped, not passed.'
        write_json(clone / 'docs/text-compatibility.json', compatibility)
        snapshots = {('tools/' + name): hashlib.sha256((clone / 'tools' / name).read_bytes()).hexdigest()
                     for name in sorted(INFRASTRUCTURE) if (clone / 'tools' / name).is_file()}
        write_json(clone / 'docs/shared-tooling.json', {'repository': MODEL_URL, 'revision': model_revision, 'ownership': 'Model is source-authoritative; Text stores pinned infrastructure snapshots, not independent semantic implementations.', 'files': snapshots,
                   'update': 'python -B tools/update_shared_tools.py --model-checkout <explicit-checkout> --revision <full-SHA>'})
    # Repository-specific workflow ownership; no hosted runs are dispatched.
    ci = clone / '.github/workflows/ci.yml'
    text = ci.read_text(encoding='utf-8')
    if owner == 'model':
        text = text.replace('scope: [model, text, host]', 'scope: [model, host]')
        browser = text.find('\n  browser:')
        if browser >= 0:
            text = text[:browser] + '\n'
        text = text.replace(',intersect-web/milestone-tests,intersect-text-acceptance/milestone-tests', ',intersect-model-acceptance/milestone-tests')
        text = text.replace('          cargo +1.99.1 run -p intersect-text-acceptance --example model_api --locked\n', '')
        # Node is needed for Model's own WASM runtime checks, with no npm/toolkit input.
        start = text.find('      - name: Install Node\n')
        end = text.find('      - name: Model-only dependency boundary', start)
        if start >= 0 and end >= 0:
            text = text[:start] + "      - uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020\n        with:\n          node-version: '22.15.1'\n" + text[end:]
    else:
        text = text.replace('scope: [model, text, host]', 'scope: [text, host]')
        start = text.find('      - name: Model-only dependency boundary')
        end = text.find('\n  browser:', start)
        if start >= 0 and end >= 0:
            text = text[:start] + text[end:]
        text = text.replace('intersect-model/milestone-tests,', '').replace(',intersect-model/rhai,intersect-semantic-bindings-tests/rhai', '')
        for line in text.splitlines(keepends=True):
            if 'cargo +1.99.1 run -p intersect-model ' in line or 'cargo +1.99.1 run -p intersect-rust-bindings-tests ' in line:
                text = text.replace(line, '')
    ci.write_text(text, encoding='utf-8')
    isolation = clone / '.github/workflows/text-isolation.yml'
    text = isolation.read_text(encoding='utf-8').replace('name: Xtext package isolation', 'name: ' + owner.title() + ' package isolation')
    if owner == 'model':
        text = text.replace('tests/text-only/', 'tests/model-only/')
    isolation.write_text(text, encoding='utf-8')
    # Entry points describe the independent product; original mixed documents
    # remain available at the preserved pre-split commit in Model history.
    role = ('Native modeling, definitions, Rcore/Ecore import, execution, code generation and host adapters.' if owner == 'model' else
            'Grammar compilation, inference, text editing and interpretation over Intersect Model; Library, Web, Toolkit and combined integration harnesses.')
    readme = f'''# Intersect {owner.title()}

{role}

The dependency direction is host/application → language package → Model → definitions.
Model does not depend on Text. Production use requires no Java.

## Development

Use `develop` as the explicit branch base. Install Rust 1.99.1, Python 3.11+,
Node 22.15.1 for WASM execution, and the `wasm32-unknown-unknown` Rust target.
Run `cargo fetch --locked` once; ordinary builds use the committed lockfile.

### Test tiers

After a change, run only changed crates, without `milestone-tests`:

```sh
python -B tools/format_workspace.py --check
cargo clippy -p intersect-{owner} --all-targets --locked -- -D warnings
python -B tools/test_report.py run -- --profile test-fast -p intersect-{owner} --lib --tests --locked
cargo check -p intersect-{owner} --lib --target wasm32-unknown-unknown --locked
```

Quick checks must finish in under ten minutes, build included. Whole-library,
language equivalence, binding rebuilds, Pilot and full workspace checks are
milestone-only. Preserve existing case names, assertions and ignore statuses.
The report dashboard is http://localhost:8765/.

See [architecture](docs/architecture.md), [roadmap](docs/roadmap.md),
[split acceptance](docs/repository-split-state.json), and [checkout/patch/recovery recipes](docs/repository-development.md).
Hosted CI execution is explicitly user-skipped during this split; retained
workflow definitions and skipped platform checks are not passing evidence.
Registry publication is a separate operation.
'''
    (clone / 'README.md').write_text(readme, encoding='utf-8')
    original = 'https://github.com/mercurio-labs/intersect-model/blob/e3225af7/docs/'
    architecture = f'''# Architecture

Intersect {owner.title()} owns: {role}

The dependency direction is host/application → language package → Model → definitions.
Model owns structural services, native resource storage and execution. Text
owns grammar compilation, inference, scoping adapters and text interpretation.
Language packages own domain algorithms, standard libraries and delegates.
Unsupported constructs remain explicit. Importing data does not qualify native
semantic support; imported, consumed and semantically verified are separate.

Model-only acceptance, Rust bindings, semantic bindings and shared Model test
support live in Model. Text acceptance, model/text parity, resource-lifecycle
integration, Library, Web and Toolkit live in Text. Production libraries do not
depend on harness support. The shared harness helpers are implemented once in
Model and consumed at the same Model Git revision as the runtime library.

Clocks, I/O, scheduling and host resources use explicit adapters. Libraries
remain compatible with wasm32-unknown-unknown; Java is an independent oracle,
never a production requirement. Generated and handwritten boundaries are marked.

See [the detailed pre-split architecture]({original}architecture.md),
[repository development](repository-development.md), and the retained split plan.
The original history in Model records the full architecture before extraction.
'''
    (clone / 'docs/architecture.md').write_text(architecture, encoding='utf-8')
    (clone / 'docs/roadmap.md').write_text(f'''# Roadmap

The repository split changes ownership and distribution; it does not add language
capability. Implementation and verification status are recorded separately in
[split acceptance](repository-split-state.json). Existing capability limits,
independent-oracle deviations and milestone-only tests remain in force.

Current work belongs with the owning API in Intersect {owner.title()}. Coordinate
cross-repository API changes by landing Model first and updating Text's immutable
Model/support pin together, then updating language and application consumers.
Registry publication and wider platform qualification remain separate work.

The [pre-split roadmap]({original}roadmap.md) retains the existing modeling and
language milestones and their original evidence; extraction does not complete them.
''', encoding='utf-8')
    write_json(clone / 'docs/repository-split/extraction.json', {'owner': owner, 'external_dependencies': external, 'removed_current_paths': len(removed), 'allocation_sha256': hashlib.sha256(allocation.read_bytes()).hexdigest(),
               'history': 'Model retains complete original history. Text path-filter history and commit-map are separate extraction artifacts.'})
    print(json.dumps({'owner': owner, 'removed': len(removed), 'external_dependencies': len(external)}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--clone', type=Path, required=True)
    parser.add_argument('--owner', choices=['model', 'text'], required=True)
    parser.add_argument('--allocation', type=Path, required=True)
    parser.add_argument('--model-revision', required=True)
    args = parser.parse_args()
    prepare(args.clone, args.owner, args.allocation, args.model_revision)


if __name__ == '__main__':
    main()
