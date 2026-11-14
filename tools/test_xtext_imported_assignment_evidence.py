"""Qualification evidence must not hide stages or normalize unrelated authority."""
from pathlib import Path
from stage_text_packages import io_path
import copy
import importlib.util
import unittest

ROOT = io_path(Path(__file__).resolve().parents[1])
CORPUS = ROOT / 'fixtures/xtext/imported-assignment-v66'
SPEC = importlib.util.spec_from_file_location('imported_assignment_evidence', CORPUS / 'verify.py')
EVIDENCE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(EVIDENCE)

class ImportedAssignmentEvidence(unittest.TestCase):
    def test_complete_frozen_corpus_and_stages(self):
        value = EVIDENCE.verify()
        self.assertEqual(value['complete_ABI_calls_per_run'], 116)
        self.assertEqual(value['native_duplicate_diagnostic_disagreements'], 4)

    def test_unrelated_absolute_genmodel_cannot_be_ignored(self):
        row = EVIDENCE.read('oracle-record/observations.json')[0]
        root = EVIDENCE.read('recorded-output-roots.json')['main']['record']
        forged = copy.deepcopy(row)
        forged['provider']['genmodel'] = 'C:/unrelated/provider-' + row['provider']['ecore_sha256'][:12] + '/provider.genmodel'
        with self.assertRaises(AssertionError):
            EVIDENCE.comparable(forged, root)

    def test_unrelated_provider_identity_cannot_be_ignored(self):
        row = EVIDENCE.read('oracle-record/observations.json')[0]
        root = EVIDENCE.read('recorded-output-roots.json')['main']['record']
        forged = copy.deepcopy(row)
        forged['provider']['ecore_sha256'] = '0' * 64
        with self.assertRaises(AssertionError):
            EVIDENCE.comparable(forged, root)

    def test_diagnostic_or_runtime_changes_are_never_normalized(self):
        row = EVIDENCE.read('oracle-record/observations.json')[0]
        root = EVIDENCE.read('recorded-output-roots.json')['main']['record']
        changed = copy.deepcopy(row)
        changed['runtime'][0]['observation']['resource']['accepted'] = not changed['runtime'][0]['observation']['resource']['accepted']
        self.assertNotEqual(EVIDENCE.comparable(row, root), EVIDENCE.comparable(changed, root))

    def test_original_failed_containment_migration_has_no_repaired_selection(self):
        rows = EVIDENCE.read('native-negative-before-v64.json')['cases']
        original = next(row for row in rows if row['selection'] is None)
        self.assertFalse(original['migration']['ok'])
        self.assertIsNone(original['migration']['report']['converted'])
        self.assertEqual(len([row for row in rows if row['selection'] is not None]), 5)

if __name__ == '__main__':
    unittest.main()
