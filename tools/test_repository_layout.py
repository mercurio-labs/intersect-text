"""Reject hidden source/input dependencies before repository extraction."""
from pathlib import Path
import tempfile
import subprocess
import unittest

import repository_layout as layout


class ClosureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.package = self.root / 'tests/rust-bindings'
        self.package.mkdir(parents=True)
        self.source = self.package / 'lib.rs'

    def test_missing_and_escaping_literal_inputs_are_rejected(self):
        for source in ['include_str!("missing.json");', 'include_bytes!("../../../outside");']:
            self.source.write_text(source, encoding='utf-8')
            with self.assertRaisesRegex(ValueError, 'Missing or escaping'):
                layout.scan_source(self.source, self.root, self.package)

    def test_generated_input_requires_registered_owned_producer(self):
        self.source.write_text('include_str!(concat!(env!("OUT_DIR"), "/library.rs"));', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'Unregistered generated'):
            layout.scan_source(self.source, self.root, self.package)
        (self.package / 'build.rs').write_text('fn main() {}', encoding='utf-8')
        inputs, outputs = layout.scan_source(self.source, self.root, self.package)
        self.assertEqual(inputs, [])
        self.assertEqual(outputs[0]['producer'], 'tests/rust-bindings/build.rs')
        foreign = self.root / 'crates/foreign'
        foreign.mkdir(parents=True)
        (foreign / 'build.rs').write_text('fn main() {}', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'Unregistered generated'):
            layout.scan_source(self.source, self.root, foreign)

    def test_arbitrary_dynamic_include_is_rejected(self):
        self.source.write_text('include!(resolve_source!());', encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'plain string literal'):
            layout.scan_source(self.source, self.root, self.package)

    def test_unknown_paths_cannot_disappear_from_allocation(self):
        with self.assertRaisesRegex(ValueError, 'Unclassified tracked path'):
            layout.initial_owner('unreviewed/runtime-provider.rs')

    def test_deep_fixture_paths_are_not_silently_omitted(self):
        subprocess.run(['git', 'init', '--quiet', str(self.root)], check=True)
        path = self.root / 'fixtures' / ('nested-' * 26) / ('nested-' * 14) / 'source.xtext'
        layout.io_path(path).parent.mkdir(parents=True)
        layout.io_path(path).write_text('grammar pinned.Source', encoding='utf-8')
        def remove_deep_fixture():
            layout.io_path(path).unlink()
            layout.io_path(path.parent).rmdir()
            layout.io_path(path.parent.parent).rmdir()
        self.addCleanup(remove_deep_fixture)
        subprocess.run(['git', '-c', 'core.longpaths=true', 'add', '.'], cwd=self.root, check=True,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.assertIn(path, layout.tracked(self.root))


if __name__ == '__main__':
    unittest.main()
