"""Negative controls for the staged runtime qualification boundary."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import verify_text_stage as verifier


class VerificationTests(unittest.TestCase):
    def sources(self, original, model):
        return {'source_root': str(original), 'source_roots': [str(original), str(model)],
                'packages': {'intersect-text': {'source_root': str(original)},
                             'intersect-model': {'source_root': str(model)}},
                'consumers': {'text-only': {'source_root': str(original)}}}

    def test_model_git_cache_prevents_isolation_even_when_text_checkout_is_absent(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            model = root / 'model-git-cache'
            model.mkdir()
            ledger = self.sources(root / 'absent-text', model)
            state = verifier.original_source_state(ledger)
            self.assertTrue(state['provenance_complete'])
            self.assertFalse(state['original_workspace_unavailable'])
            self.assertIn(str(model), state['present_roots'])
            stage = root / 'stage'
            stage.mkdir()
            (stage / 'staging-receipt.json').write_text(json.dumps(ledger))
            with self.assertRaisesRegex(ValueError, 'Original workspace is available'):
                verifier.verify(stage, stage / 'result', None, True)
            self.assertFalse((stage / 'result').exists())

    def test_missing_or_inconsistent_provenance_cannot_award_strict_isolation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            legacy = {'source_root': str(root / 'absent')}
            stage = root / 'stage'
            stage.mkdir()
            (stage / 'staging-receipt.json').write_text(json.dumps(legacy))
            with self.assertRaisesRegex(ValueError, 'provenance is incomplete'):
                verifier.verify(stage, stage / 'result', None, True)
            self.assertFalse((stage / 'result').exists())
            ledger = self.sources(root / 'absent-text', root / 'absent-model')
            ledger['source_roots'].pop()
            self.assertFalse(verifier.original_source_state(ledger)['provenance_complete'])
            ledger = self.sources(root / 'absent-text', root / 'absent-model')
            del ledger['packages']['intersect-model']['source_root']
            self.assertFalse(verifier.original_source_state(ledger)['provenance_complete'])

    def test_all_recorded_sources_must_be_absent_and_host_checkout_is_checked(self):
        with tempfile.TemporaryDirectory() as temporary, mock.patch.dict(verifier.os.environ, {}, clear=True):
            root = Path(temporary)
            ledger = self.sources(root / 'absent-text', root / 'absent-model')
            self.assertTrue(verifier.original_source_state(ledger)['original_workspace_unavailable'])
            (root / 'Cargo.toml').write_text('[workspace]')
            with mock.patch.dict(verifier.os.environ, {'GITHUB_WORKSPACE': str(root)}):
                self.assertFalse(verifier.original_source_state(ledger)['original_workspace_unavailable'])

    def test_present_original_cannot_be_reported_as_unavailable(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = root / 'original'
            original.mkdir()
            (original / 'Cargo.toml').write_text('[workspace]\n')
            stage = root / 'stage'
            stage.mkdir()
            (stage / 'staging-receipt.json').write_text(json.dumps({'source_root': str(original)}))
            with self.assertRaisesRegex(ValueError, 'Original workspace is available'):
                verifier.verify(stage, stage / 'result', None, True)
            self.assertFalse((stage / 'result').exists())

    def test_modified_verification_payload_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary)
            (stage / 'verification').mkdir()
            (stage / 'verification/check.py').write_text('modified')
            ledger = {'packages': {}, 'consumers': {}, 'verification': {'check.py': 'previous'}}
            with self.assertRaisesRegex(ValueError, 'Verification payload changed'):
                verifier.audit(stage, ledger)

    def test_matching_file_hash_cannot_allow_an_embedded_path_escape(self):
        with tempfile.TemporaryDirectory() as temporary:
            stage = Path(temporary)
            owner = stage / 'packages/model'
            (owner / 'src').mkdir(parents=True)
            source = owner / 'src/lib.rs'
            source.write_text('const DATA: &str = include_str!("../../outside.txt");')
            (stage / 'packages/outside.txt').write_text('foreign')
            (owner / 'Cargo.lock').write_text('lock')
            entry = {'files': [{'staged': 'src/lib.rs', 'staged_sha256': verifier.digest(source)}],
                     'lock_sha256': verifier.digest(owner / 'Cargo.lock')}
            ledger = {'packages': {'model': entry}, 'consumers': {}, 'verification': {}}
            with self.assertRaisesRegex(ValueError, 'input escapes package'):
                verifier.audit(stage, ledger)


if __name__ == '__main__':
    unittest.main()
