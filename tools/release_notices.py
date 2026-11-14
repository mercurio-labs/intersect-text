"""Explicit attribution closure for the reviewed source-bundle inputs.

This is an inventory of retained upstream terms, not an SPDX license resolver.
New third-party inputs require a reviewed rule; generated data counts as input.
"""
import hashlib
import json
from pathlib import Path
import re
import shutil
import tomllib


RULES = (
    ('unicode13', 'Unicode 13 generated tables', 'Unicode-3.0',
     ('crates/intersect-model/src/definitions/scalar_unicode.rs',
      'crates/intersect-model/src/definitions/ecore_identifier.rs'),
     ('fixtures/unicode-13/LICENSE.txt', 'fixtures/unicode-13/README.md')),
    ('emf', 'Eclipse EMF Ecore 2.38.0', 'EPL-2.0',
     ('crates/intersect-model/assets/fixtures/definition-conformance/upstream/ecore/',
      'fixtures/definition-conformance/upstream/ecore/'),
     ('fixtures/definition-conformance/upstream/ecore/about.html',
      'fixtures/definition-conformance/upstream/artifacts.json',
      'fixtures/definition-conformance/mercurio/LICENSE-Pilot')),
    ('xtext', 'Eclipse Xtext 2.38.0', 'EPL-2.0',
     ('fixtures/definition-conformance/upstream/xtext/',
      'fixtures/xtext-full-predicates-v68/Terminals.xtext'),
     ('fixtures/definition-conformance/upstream/xtext/about.html',
      'fixtures/definition-conformance/upstream/artifacts.json',
      'fixtures/definition-conformance/mercurio/LICENSE-Pilot')),
    ('pilot-operands', 'Pilot / Mercurio operand evidence', 'EPL-2.0; MIT (see provenance)',
     ('fixtures/xtext/pinned-kerml-operands/',),
     ('fixtures/xtext/pinned-kerml-operands/LICENSE-UPSTREAM',
      'fixtures/xtext/pinned-kerml-operands/LICENSE-MERCURIO',
      'fixtures/xtext/pinned-kerml-operands/README.md',
      'fixtures/xtext/pinned-kerml-operands/provenance.json')),
    ('pilot-preview', 'Pilot / Mercurio conformance evidence', 'EPL-2.0; MIT (see provenance)',
     ('fixtures/definition-conformance/mercurio/',),
     ('fixtures/definition-conformance/mercurio/LICENSE-Pilot',
      'fixtures/definition-conformance/mercurio/LICENSE-Mercurio',
      'fixtures/definition-conformance/mercurio/artifacts.json',
      'fixtures/definition-conformance/README.md')),
)

# The registry tarball omits these repository-root licenses. Never substitute
# notices from another version or silently change the checksummed vendor tree.
SUPPLEMENTS = {('rhai_codegen', '3.2.0'): 'b247dd730de3f2c97fa1e46f138a17f9d844de67'}


def registry_supplement(stage, manifest, package):
    key = (package['name'], package['version'])
    if key not in SUPPLEMENTS:
        return []
    revision = json.loads((manifest.parent / '.cargo_vcs_info.json').read_text())['git']['sha1']
    if revision != SUPPLEMENTS[key]:
        raise ValueError('Registry notice revision mismatch: ' + package['name'])
    root = stage / 'notices' / ('-'.join(key))
    provenance = root / 'provenance.json'
    data = json.loads(provenance.read_text())
    if (data['package'], data['version']) != key or data['revision'] != revision or data['license'] != package.get('license'):
        raise ValueError('Registry notice provenance mismatch: ' + package['name'])
    paths = [provenance]
    for entry in data['files']:
        path = root / entry['file']
        if path.parent != root or digest(path) != entry['sha256']:
            raise ValueError('Registry notice hash/path mismatch: ' + package['name'])
        paths.append(path)
    if {p.name for p in paths} != {'provenance.json', 'LICENSE-MIT.txt', 'LICENSE-APACHE.txt'}:
        raise ValueError('Incomplete registry notice supplement: ' + package['name'])
    return paths


def matches(path, prefix):
    return path.startswith(prefix) if prefix.endswith('/') else path == prefix


def applicable(originals):
    return [rule for rule in RULES if any(matches(path, prefix)
            for path in originals for prefix in rule[3])]


def add_notice_inputs(mapping, tracked, root):
    """Add obligations for original inputs only; license copies don't add owners."""
    original = [path.relative_to(root).as_posix() for path in mapping]
    for rule in applicable(original):
        for name in rule[4]:
            path = (root / name).resolve()
            if path not in tracked:
                raise ValueError(f'Missing tracked notice for {rule[0]}: {name}')
            mapping.setdefault(path, Path('packaged-inputs') / name)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(stage, ledger):
    """Build a portable index from receipt-owned paths, including vendor licenses."""
    result = {'scope': 'Reviewed staged source inputs and vendored registry packages',
              'packages': {}, 'registry': []}
    for group in ('packages', 'consumers'):
        for name, entry in ledger[group].items():
            rows = {row['original']: row for row in entry['files']}
            # Restrict triggers to original program/evidence content. Supplementary
            # license files can share another project's fixture directory.
            triggers = [p for p in rows if not p.endswith(('LICENSE-Pilot', 'LICENSE-Mercurio'))]
            components = []
            for rule in applicable(triggers):
                notices = []
                for original in rule[4]:
                    if original not in rows:
                        raise ValueError(f'Missing staged notice for {name}/{rule[0]}: {original}')
                    row = rows[original]
                    path = stage / group / name / row['staged']
                    notices.append({'path': path.relative_to(stage).as_posix(),
                                    'sha256': digest(path), 'original': original})
                components.append({'id': rule[0], 'component': rule[1], 'license': rule[2],
                                   'notices': notices})
            result['packages'][name] = components
    if ledger.get('vendor'):
        for manifest in sorted((stage / 'vendor').glob('*/Cargo.toml')):
            package = tomllib.loads(manifest.read_text(encoding='utf-8'))['package']
            notices = [p for p in sorted(manifest.parent.rglob('*')) if p.is_file()
                       and re.search(r'licen[cs]e|notice|copying|copyright', p.name, re.I)]
            notices += registry_supplement(stage, manifest, package)
            explicit = package.get('license-file')
            if explicit:
                path = (manifest.parent / explicit).resolve()
                if not path.is_relative_to(manifest.parent.resolve()) or not path.is_file():
                    raise ValueError('Missing or escaping registry license-file: ' + package['name'])
                if path not in notices:
                    notices.append(path)
            if not notices or not (package.get('license') or explicit):
                raise ValueError('Registry package has no retained license: ' + package['name'])
            result['registry'].append({'name': package['name'], 'version': package['version'],
                'license': package.get('license'), 'license_file': explicit,
                'repository': package.get('repository'),
                'notices': [{'path': p.relative_to(stage).as_posix(), 'sha256': digest(p)} for p in notices]})
    return result


def write_index(stage, ledger):
    if ledger.get('vendor'):
        for key in SUPPLEMENTS:
            name = '-'.join(key)
            if (stage / 'vendor' / name / 'Cargo.toml').is_file():
                shutil.copytree(Path(__file__).parent / 'release-notices' / name,
                                stage / 'notices' / name, dirs_exist_ok=True)
    data = inventory(stage, ledger)
    index = stage / 'third-party-notices.json'
    index.write_text(json.dumps(data, indent=2) + '\n', encoding='utf-8')
    lines = ['# Third-party notices', '',
             'Intersect-authored code uses MIT. Imported sources and generated data retain their own terms.',
             'This index covers this source bundle, not Toolkit, binary releases or every repository fixture.',
             'Original copyright headers, license alternatives and upstream notices are preserved.',
             'Eclipse about notices may describe runtime dependencies that are not included in this bundle.',
             'The standard EPL-2.0 text is retained under its original LICENSE-Pilot filename.', '',
             'See [the machine-readable inventory](third-party-notices.json) for exact versions and hashes.', '']
    for name, components in data['packages'].items():
        lines.extend(['## ' + name, ''])
        for component in components:
            lines.append('- ' + component['component'] + ': ' + component['license'] + '. ' +
                         ', '.join(f"[{Path(n['path']).name}]({n['path']})" for n in component['notices']))
        if not components:
            lines.append('No additional third-party content identified in this package; dependencies are listed below.')
        lines.append('')
    lines.extend(['## Vendored Rust dependencies', ''])
    for package in data['registry']:
        lines.append(f"- {package['name']} {package['version']}: {package['license'] or package['license_file']}. " +
                     ', '.join(f"[{Path(n['path']).name}]({n['path']})" for n in package['notices']))
    notice = stage / 'THIRD_PARTY_NOTICES.md'
    notice.write_text('\n'.join(lines) + '\n', encoding='utf-8')
    ledger['notices'] = {p.name: digest(p) for p in (index, notice)}
    for path in sorted((stage / 'notices').rglob('*')):
        if path.is_file():
            ledger['notices'][path.relative_to(stage).as_posix()] = digest(path)
