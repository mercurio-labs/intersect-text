"""Notice-closure controls; no license substitutions or fixture rewrites."""
import json
from pathlib import Path
import tempfile
import unittest

import release_notices as notices


class NoticeTests(unittest.TestCase):
    def test_pinned_registry_supplement_is_retained_and_tampering_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / 'vendor/rhai_codegen-3.2.0'
            package.mkdir(parents=True)
            (package / 'Cargo.toml').write_text('[package]\nname="rhai_codegen"\nversion="3.2.0"\nlicense="MIT OR Apache-2.0"\n')
            vcs = package / '.cargo_vcs_info.json'
            vcs.write_text(json.dumps({'git': {'sha1': notices.SUPPLEMENTS[('rhai_codegen', '3.2.0')]}}))
            ledger = {'packages': {}, 'consumers': {}, 'vendor': {'files': {}}}
            notices.write_index(root, ledger)
            entry = notices.inventory(root, ledger)['registry'][0]
            self.assertEqual(len(entry['notices']), 3)
            self.assertFalse((package / 'LICENSE-MIT.txt').exists())
            license_path = root / 'notices/rhai_codegen-3.2.0/LICENSE-MIT.txt'
            original = license_path.read_bytes()
            license_path.write_text('changed')
            with self.assertRaisesRegex(ValueError, 'hash/path mismatch'):
                notices.inventory(root, ledger)
            license_path.write_bytes(original)
            vcs.write_text(json.dumps({'git': {'sha1': 'different'}}))
            with self.assertRaisesRegex(ValueError, 'revision mismatch'):
                notices.inventory(root, ledger)

    def test_generated_unicode_requires_license_even_without_literal_embed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            source = root / 'crates/intersect-model/src/definitions/scalar_unicode.rs'
            mapping = {source: Path('src/definitions/scalar_unicode.rs')}
            with self.assertRaisesRegex(ValueError, 'Missing tracked notice'):
                notices.add_notice_inputs(mapping, {source}, root)
            tracked = {source, root / 'fixtures/unicode-13/LICENSE.txt', root / 'fixtures/unicode-13/README.md'}
            notices.add_notice_inputs(mapping, tracked, root)
            self.assertEqual(set(mapping), tracked)

    def test_missing_registry_license_or_escaping_declared_file_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / 'vendor/example-1.0'
            package.mkdir(parents=True)
            manifest = package / 'Cargo.toml'
            base = '[package]\nname="example"\nversion="1.0"\n'
            manifest.write_text(base + 'license="MIT"\n')
            ledger = {'packages': {}, 'consumers': {}, 'vendor': {'files': {}}}
            with self.assertRaisesRegex(ValueError, 'no retained license'):
                notices.inventory(root, ledger)
            (package / 'LICENCE.md').write_text('original license')
            self.assertEqual(len(notices.inventory(root, ledger)['registry']), 1)
            (root / 'outside').write_text('license')
            manifest.write_text(base + 'license-file="../../outside"\n')
            with self.assertRaisesRegex(ValueError, 'escaping'):
                notices.inventory(root, ledger)

    def test_index_preserves_declared_alternatives_and_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / 'vendor/example-1.0'
            package.mkdir(parents=True)
            (package / 'Cargo.toml').write_text('[package]\nname="example"\nversion="1.0"\nlicense="Apache-2.0 OR GPL-2.0-only"\n')
            for name in ('LICENSE-APACHE', 'LICENSE-GPLv2'):
                (package / name).write_text(name)
            ledger = {'packages': {}, 'consumers': {}, 'vendor': {'files': {}}}
            notices.write_index(root, ledger)
            data = json.loads((root / 'third-party-notices.json').read_text())
            self.assertEqual(data['registry'][0]['license'], 'Apache-2.0 OR GPL-2.0-only')
            self.assertEqual(len(data['registry'][0]['notices']), 2)
            self.assertEqual(ledger['notices']['third-party-notices.json'], notices.digest(root / 'third-party-notices.json'))

    def test_upstream_notices_cannot_silently_disappear_from_receipt(self):
        with tempfile.TemporaryDirectory() as directory:
            ledger = {'packages': {'model': {'files': [{'original': 'crates/intersect-model/src/definitions/scalar_unicode.rs'}]}}, 'consumers': {}}
            with self.assertRaisesRegex(ValueError, 'Missing staged notice'):
                notices.inventory(Path(directory), ledger)


if __name__ == '__main__':
    unittest.main()
