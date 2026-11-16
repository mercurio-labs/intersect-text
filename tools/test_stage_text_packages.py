"""Packaging boundary controls; no Cargo or whole-library preparation required."""
import unittest
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
from unittest import mock

import stage_text_packages as stage


class StagingTests(unittest.TestCase):
    def test_vendoring_covers_optional_library_graph_and_checks_it_offline(self):
        with TemporaryDirectory() as directory:
            output = Path(directory).resolve()
            (output / 'metadata').mkdir()
            ledger = {'packages': {'intersect-model': {}}, 'consumers': {'model-only': {}}}
            result = SimpleNamespace(returncode=0, stdout=b'{"packages": []}', stderr=b'')
            with mock.patch.object(stage.subprocess, 'run', return_value=result) as run:
                stage.vendor_dependencies(output, ledger)
            vendor = run.call_args_list[0].args[0]
            self.assertIn('--sync', vendor)
            self.assertEqual(Path(vendor[vendor.index('--sync') + 1]),
                             stage.process_path(output / 'packages/intersect-model/Cargo.toml'))
            checked = run.call_args_list[1:]
            self.assertEqual(len(checked), 2)
            for call in checked:
                self.assertIn('--all-features', call.args[0])
                self.assertIn('--locked', call.args[0])
                self.assertIn('--offline', call.args[0])

    def test_tracked_consumer_lock_records_resolved_bytes_and_keeps_original_hash(self):
        with TemporaryDirectory() as directory:
            output = Path(directory)
            package = output / 'packages/intersect-model'
            package.mkdir(parents=True)
            manifest = package / 'Cargo.toml'
            manifest.write_text('[package]\nname = "intersect-model"\nversion = "0.1.0"\n', encoding='utf-8')
            resolved = b'version = 4\n# stage-local resolved sources\n'
            (package / 'Cargo.lock').write_bytes(resolved)
            original = stage.sha(b'original consumer Git sources')
            seed = stage.sha(b'workspace seed lock')
            row = {'staged': 'Cargo.lock', 'original_sha256': original, 'staged_sha256': original}
            entry = {'files': [row], 'seed_lock_sha256': seed}
            ledger = {'packages': {'intersect-model': entry}, 'consumers': {}}
            metadata = {'packages': [{'name': 'intersect-model', 'source': None,
                                      'manifest_path': str(manifest)}]}
            result = SimpleNamespace(returncode=0, stdout=json.dumps(metadata).encode(), stderr=b'')
            with mock.patch.object(stage.subprocess, 'run', return_value=result):
                stage.resolve_staged_locks(output, ledger, set())
            self.assertEqual(row['staged_sha256'], stage.sha(resolved))
            self.assertEqual(row['staged_sha256'], entry['lock_sha256'])
            self.assertEqual(row['original_sha256'], original)
            self.assertEqual(entry['seed_lock_sha256'], seed)
            self.assertIn('stage-local', row['normalization'])

    def test_extracted_readme_links_preserve_anchors_and_package_local_links(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            owner = root / 'crates/xtext'
            owner.mkdir(parents=True)
            (root / 'docs').mkdir()
            (root / 'docs/guide.md').write_text('Guide')
            text = '[guide](../../docs/guide.md#limits) [local](src/lib.rs) [web](https://example.com)'
            result = stage.standalone_readme(text, owner / 'README.md', owner, root, 'abc123')
            self.assertIn('/blob/abc123/docs/guide.md#limits)', result)
            self.assertIn('[local](src/lib.rs) [web](https://example.com)', result)
            with self.assertRaises(ValueError):
                stage.standalone_readme('[missing](../../docs/missing.md)', owner / 'README.md',
                                        owner, root, 'abc123')

    def test_comments_strings_and_raw_strings_do_not_become_dependencies(self):
        source = '''// include_str!("comment.txt")
/* nested /* include!("nested.rs") */ #[path = "comment.rs"] */
const EXAMPLE: &str = r###"include_bytes!("raw.bin")"###;
const QUOTED: &str = "include_str!(\\\"string.txt\\\")";
const ACTUAL: &str = include_str!(r#"actual.txt"#);
#[path = "actual.rs"] mod actual;
'''
        found = [stage.literal(match['literal']) for match in stage.embedded_references(source)]
        self.assertEqual(found, ['actual.txt', 'actual.rs'])
        relocated = stage.relocate_embeds(source, lambda match: match['prefix'] + '"owned"')
        self.assertIn('include_str!("comment.txt")', relocated)
        self.assertIn('r###"include_bytes!("raw.bin")"###', relocated)
        self.assertEqual(relocated.count('"owned"'), 2)

    def test_dynamic_inputs_cannot_silently_escape_the_copy_plan(self):
        for source in ['include_str!(concat!("a", "b"))', '#[path = unknown] mod x;']:
            with self.subTest(source=source), self.assertRaises(ValueError):
                list(stage.embedded_references(source))

    def test_standalone_dependencies_keep_versions_and_inherited_features(self):
        data = {'package': {'name': 'consumer', 'version': {'workspace': True}},
                'dependencies': {'model': {'workspace': True, 'features': ['xmi']}}}
        workspace = {'package': {'version': '1.2.3'},
                     'dependencies': {'model': {'package': 'intersect-model', 'path': 'original',
                                                'version': '0.1.0', 'features': ['codegen']}}}
        result = stage.standalone_manifest(data, workspace)
        self.assertEqual(result['package']['version'], '1.2.3')
        self.assertIs(result['package']['publish'], False)
        self.assertEqual(result['package']['license-file'], 'LICENSE')
        self.assertEqual(result['dependencies']['model'], {
            'package': 'intersect-model', 'version': '0.1.0', 'features': ['codegen', 'xmi']})
        stage.manifest_text(result)  # Requires an exact TOML value round trip.
        data['dependencies'] = {'unknown': {'path': '../original'}}
        with self.assertRaises(ValueError):
            stage.standalone_manifest(data, workspace)

    def test_registry_publish_opt_in_does_not_leak_into_source_bundle(self):
        data = {'package': {'name': 'model', 'version': '0.1.0', 'publish': True, 'license': 'MIT'}}
        result = stage.standalone_manifest(data, {'package': {}})
        self.assertIs(result['package']['publish'], False)
        self.assertNotIn('license', result['package'])
        self.assertEqual(result['package']['license-file'], 'LICENSE')

    def test_registry_dependencies_are_exact_and_sources_remain_local_only_in_patches(self):
        data = {'package': {'name': 'intersect-text', 'version': '0.1.0-alpha.1'},
                'dependencies': {'intersect-model': {'git': 'https://example.test/model',
                                                   'rev': 'a' * 40, 'version': '0.1.0-alpha.1'}}}
        result = stage.standalone_manifest(data, {'package': {}}, patches={}, registry=True)
        self.assertEqual(result['package']['publish'], ['crates-io'])
        self.assertEqual(result['dependencies']['intersect-model'], {'version': '=0.1.0-alpha.1'})
        self.assertNotIn('patch', result)
        stage.manifest_text(result)

    def test_registry_keeps_literal_inputs_without_unrelated_oracle_corpora(self):
        with TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            package = root / 'crates/example'
            source = package / 'src/lib.rs'
            source.parent.mkdir(parents=True)
            source.write_text('const INPUT: &str = include_str!("../../../fixtures/demo/used.txt");')
            used = root / 'fixtures/demo/used.txt'
            used.parent.mkdir(parents=True)
            used.write_text('required bytes')
            extra = used.with_name('unrelated-oracle.txt')
            extra.write_text('source-bundle evidence')
            tracked = {source, used, extra}
            full, _ = stage.package_inputs('example', tracked, package, root)
            registry, _ = stage.package_inputs('example', tracked, package, root, full_evidence=False)
            self.assertIn(extra, full)
            self.assertIn(used, registry)
            self.assertNotIn(extra, registry)


if __name__ == '__main__':
    unittest.main()
