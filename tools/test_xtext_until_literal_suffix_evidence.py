"""Exact bytes and raw observations for the separate literal Until companion."""
import copy
import importlib.util
from pathlib import Path
from stage_text_packages import io_path
import unittest

ROOT = io_path(Path(__file__).resolve().parents[1])
FIXTURE = ROOT / 'fixtures/xtext/full-generated-until-literal-suffix'
SPEC = importlib.util.spec_from_file_location('literal_until_suffix_verify', FIXTURE / 'verify.py')
VERIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFY)


class LiteralUntilSuffixEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = VERIFY.load(FIXTURE / 'observations.json')
        cls.cases = VERIFY.load(FIXTURE / 'inputs/cases.json')
        cls.positive = VERIFY.load(FIXTURE / 'positive-resources.json')
        cls.provenance = VERIFY.load(FIXTURE / 'provenance.json')

    def test_frozen_bytes_identity_original_schema_and_separate_denominator(self):
        self.assertEqual(VERIFY.frozen(), VERIFY.EXPECTED_COUNTS)
        self.assertEqual(len(self.rows), 1)
        self.assertEqual(self.cases[0]['sources'][0]['source_id']['sha256'], 'f80f72ea0daf718e83753ef24cdf3072348dfdb352d006eaf027be06b3df10b3')
        self.assertEqual(VERIFY.digest(FIXTURE / 'inputs/literal_until_suffix_c/imported.ecore'), self.provenance['original_ecore_sha256'])
        self.assertEqual(VERIFY.digest(ROOT / 'fixtures/xtext/full-generated-until/provenance.json'), self.provenance['frozen_eighteen_graph_provenance_sha256_unchanged'])

    def test_raw_record_and_fresh_replay_preserved_without_projection(self):
        lineage = VERIFY.load(FIXTURE / 'lineage.json')
        record = VERIFY.load(FIXTURE / 'recorded/record/observations.json')
        replay = VERIFY.load(FIXTURE / 'recorded/replay/observations.json')
        self.assertEqual(record, self.rows)
        self.assertEqual(VERIFY.comparable(record, lineage['record_root']), VERIFY.comparable(replay, lineage['replay_root']))
        self.assertEqual(len(list((FIXTURE / 'emitted').rglob('*Lexer.java'))), 1)
        self.assertEqual(len(VERIFY.load(FIXTURE / 'inputs/stack.json')['jars']), 60)

    def test_early_incomplete_delimiter_and_rejected_partial_streams(self):
        values = {run['text']: run['observation'] for run in self.rows[0]['runtime']}
        for text in ['@abc', '@abxabc', '@xabc', '@ababc']:
            self.assertTrue(values[text]['resource']['accepted'])
            self.assertEqual(values[text]['resource']['model'][0]['values']['name'], text)
            self.assertEqual(values[text]['lexer']['tokens'][0]['utf16_stop'], len(text) - 1)
        for text in ['@abcc', '@abcabc']:
            value = values[text]
            self.assertFalse(value['resource']['accepted'])
            self.assertEqual(value['resource']['model'][0]['values']['name'], '@abc')
            self.assertEqual(value['lexer']['tokens'][0]['text'], '@abc')
            self.assertTrue(all(token['channel'] == 99 for token in value['lexer']['tokens'][1:-1]))
        self.assertIsNone(values['@abx']['resource']['model'])
        self.assertEqual(values['@abx']['resource']['errors'][0]['message'], "mismatched character '<EOF>' expecting 'a'")

    def test_partial_acceptance_and_owned_path_tampering_reject(self):
        altered = copy.deepcopy(self.rows)
        next(run['observation']['resource'] for run in altered[0]['runtime'] if run['text'] == '@abcc')['accepted'] = True
        with self.assertRaisesRegex(ValueError, 'mislabeled'):
            VERIFY.validate(altered, self.cases, self.positive)
        roots = self.provenance['recorded_provider_output_roots']
        for path in ['provider.genmodel', 'D:/wrong/provider.genmodel', roots[self.rows[0]['id']] + '/../provider-726b2116570f/provider.genmodel']:
            altered = copy.deepcopy(self.rows)
            altered[0]['provider']['genmodel'] = path
            with self.assertRaises(ValueError):
                VERIFY.comparable(altered, roots)
        altered = copy.deepcopy(self.rows)
        altered[0]['runtime'][0]['observation']['lexer']['antlr_syntax_error_count'] += 1
        self.assertNotEqual(VERIFY.comparable(altered, roots), VERIFY.comparable(self.rows, roots))

    def test_preserved_initial_location_failure_adapter_changes_only_guard(self):
        original = (FIXTURE / 'produce.py').read_text(encoding='utf-8')
        adapter = (FIXTURE / 'recorded/original-producer/produce-location-adapter-v1.py').read_text(encoding='utf-8')
        self.assertEqual(original.replace('protected=HERE.parents[1]', "protected=HERE/'inputs'"), adapter)
        failure = VERIFY.load(FIXTURE / 'recorded/original-producer/producer-location-failure-and-adapter.json')
        self.assertEqual(failure['initial_observed_exit'], 1)
        self.assertFalse(failure['attempted_semantic_execution_before_failure'])


if __name__ == '__main__':
    unittest.main()
