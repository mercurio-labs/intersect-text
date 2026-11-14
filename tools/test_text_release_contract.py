"""Refusal controls for synchronized release fields and historical wire drift."""
import json
from pathlib import Path
import tempfile
import unittest

import check_text_release_contract as guard

REPOSITORY = 'https://github.com/mercurio-labs/intersect-model.git'
REVISION = '1' * 40
DEPENDENCY = f'intersect-model = {{ version = "0.1.0", git = "{REPOSITORY}", rev = "{REVISION}" }}'


class ReleaseContractTests(unittest.TestCase):
    def fixture(self, root):
        contract = {'model': {'version': '0.1.0', 'minimum_tested_revision': '0' * 40},
                    'text': {'version': '0.1.0', 'model_version_requirement': '0.1.0'},
                    'model_dependency': {'repository': REPOSITORY, 'revision': REVISION},
                    'legacy_ordinary_compiler': {'name': 'intersect-model', 'version': '0.1.0'}}
        fixtures = {
            'Cargo.toml': '[workspace]\nmembers = []\n[workspace.dependencies]\n' + DEPENDENCY + '\n',
            'Cargo.lock': self.lock(),
            'docs/text-compatibility.json': json.dumps(contract),
            'docs/model-compatibility.json': json.dumps(contract),
            'fixtures/xtext/inference-archive/ordinary-baseline.json': json.dumps({'manifest': {'compiler': contract['legacy_ordinary_compiler']}}),
            'crates/intersect-model/Cargo.toml': '[package]\nname = "intersect-model"\nversion = "0.1.0"\n',
            'crates/intersect-text/Cargo.toml': '[package]\nname = "intersect-text"\nversion = "0.1.0"\n[dependencies]\n' + DEPENDENCY + '\n',
        }
        for name, text in fixtures.items():
            target = root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text, encoding='utf-8')

    def lock(self, revision=REVISION):
        return 'version = 4\n' + ''.join(
            f'[[package]]\nname = "{name}"\nversion = "0.1.0"\n'
            f'source = "git+{REPOSITORY}?rev={revision}#{revision}"\n'
            for name in ('intersect-model', 'intersect-build-support'))

    def test_each_owner_checks_without_the_other_manifest(self):
        for owner in ('model', 'text'):
            with self.subTest(owner=owner), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.fixture(root)
                other = 'text' if owner == 'model' else 'model'
                (root / f'crates/intersect-{other}/Cargo.toml').unlink()
                self.assertEqual(guard.check(root), [])
                path = root / f'crates/intersect-{owner}/Cargo.toml'
                path.write_text(path.read_text(encoding='utf-8').replace('version = "0.1.0"', 'version = "99"', 1), encoding='utf-8')
                self.assertTrue(guard.check(root))

    def test_inherited_versions_or_dependency_constraints_cannot_restore_lockstep(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            self.assertEqual(guard.check(root), [])
            model = root / 'crates/intersect-model/Cargo.toml'
            model.write_text(model.read_text().replace('version = "0.1.0"', 'version.workspace = true', 1))
            self.assertIn('model must declare its own recorded release version', guard.check(root))
            text = root / 'crates/intersect-text/Cargo.toml'
            text.write_text(text.read_text().replace(
                DEPENDENCY,
                'intersect-model = { workspace = true }'))
            self.assertIn('Text must declare its own model compatibility requirement', guard.check(root))

    def test_archive_descriptor_drift_and_ambiguous_revision_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            path = root / 'docs/text-compatibility.json'
            data = json.loads(path.read_text())
            data['legacy_ordinary_compiler']['version'] = '99'
            data['model']['minimum_tested_revision'] = 'short'
            path.write_text(json.dumps(data))
            errors = guard.check(root)
            self.assertTrue(any('frozen archive' in error for error in errors))
            self.assertTrue(any('complete commit SHA' in error for error in errors))

    def test_selected_revision_is_distinct_from_historical_minimum(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            self.assertEqual(guard.check(root), [])
            path = root / 'docs/text-compatibility.json'
            data = json.loads(path.read_text())
            data['model_dependency']['revision'] = 'short'
            path.write_text(json.dumps(data))
            self.assertTrue(any('full selected revision' in e for e in guard.check(root)))

    def test_runtime_build_dev_target_and_renamed_dependencies_must_share_pin(self):
        cases = [
            ('dependencies', 'intersect-model'),
            ('build-dependencies', 'intersect-build-support'),
            ('dev-dependencies', 'intersect-model-test-support'),
            ('target.\'cfg(windows)\'.dependencies', 'intersect-model'),
        ]
        for section, package in cases:
            with self.subTest(section=section), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.fixture(root)
                path = root / 'tests/consumer/Cargo.toml'
                path.parent.mkdir(parents=True)
                path.write_text(f'[package]\nname = "consumer"\nversion = "0.0.0"\n'
                                f'[{section}]\nrenamed = {{ package = "{package}", '
                                f'git = "{REPOSITORY}", rev = "{"2" * 40}", version = "0.1.0" }}\n')
                self.assertTrue(any('recorded immutable Model source' in e for e in guard.check(root)))

    def test_inherited_support_dependency_and_workspace_pin_are_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            (root / 'Cargo.toml').write_text('[workspace]\n[workspace.dependencies]\n' + DEPENDENCY + '\n'
                f'support = {{ package = "intersect-model-test-support", git = "{REPOSITORY}", rev = "{"2" * 40}" }}\n')
            path = root / 'tests/consumer/Cargo.toml'
            path.parent.mkdir(parents=True)
            path.write_text('[package]\nname = "consumer"\nversion = "0.0.0"\n'
                            '[dev-dependencies]\nsupport.workspace = true\n')
            errors = guard.check(root)
            self.assertTrue(any('workspace.dependencies' in e for e in errors))
            self.assertTrue(any('tests/consumer/Cargo.toml' in e for e in errors))

    def test_path_branch_tag_and_redirected_url_are_not_immutable_canonical_pins(self):
        for extra in ('path = "../local"', 'branch = "develop"', 'tag = "v0.1.0"'):
            with self.subTest(extra=extra), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.fixture(root)
                path = root / 'crates/intersect-text/Cargo.toml'
                path.write_text(path.read_text().replace(DEPENDENCY, DEPENDENCY[:-1] + ', ' + extra + '}'))
                self.assertTrue(any('immutable Model source' in e for e in guard.check(root)))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            path = root / 'crates/intersect-text/Cargo.toml'
            path.write_text(path.read_text().replace(REPOSITORY, REPOSITORY.removesuffix('.git')))
            self.assertTrue(any('immutable Model source' in e for e in guard.check(root)))

    def test_standalone_lock_missing_stale_duplicate_and_wrong_resolved_commit_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            path = root / 'tests/standalone/Cargo.toml'
            path.parent.mkdir(parents=True)
            path.write_text('[package]\nname = "standalone"\nversion = "0.0.0"\n[workspace]\n'
                            '[dependencies]\n' + DEPENDENCY + '\n')
            lock = path.with_name('Cargo.lock')
            self.assertTrue(any('missing maintained lockfile' in e for e in guard.check(root)))
            lock.write_text(self.lock('2' * 40))
            self.assertTrue(any('different resolved Model source' in e for e in guard.check(root)))
            lock.write_text(self.lock().replace('#' + REVISION, '#' + '2' * 40))
            self.assertTrue(any('different resolved Model source' in e for e in guard.check(root)))
            lock.write_text(self.lock() + self.lock().split('\n', 1)[1])
            self.assertTrue(any('duplicate Model package' in e for e in guard.check(root)))
            lock.write_text('version = 4\n')
            self.assertTrue(any('missing resolved intersect-model' in e for e in guard.check(root)))
            lock.write_text(self.lock())
            self.assertEqual(guard.check(root), [])

    def test_historical_fixture_manifests_are_outside_maintained_scope(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            path = root / 'fixtures/old/native/Cargo.toml'
            path.parent.mkdir(parents=True)
            path.write_text('historical input, not an active manifest')
            self.assertEqual(guard.check(root), [])

    def test_legacy_record_must_be_explicitly_superseded_without_rewriting_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            path = root / 'docs/xtext-compatibility.json'
            path.write_text('{}')
            self.assertTrue(any('labeled historical' in e for e in guard.check(root)))
            path.write_text(json.dumps({'status': 'historical', 'superseded_by': 'text-compatibility.json',
                                        'model': {'minimum_tested_revision': '0' * 40}}))
            self.assertEqual(guard.check(root), [])


if __name__ == '__main__':
    unittest.main()
