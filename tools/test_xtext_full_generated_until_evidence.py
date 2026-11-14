"""Byte and stage integrity for the additive full-generated Until oracle."""
import copy
import importlib.util
from pathlib import Path
from stage_text_packages import io_path
import unittest

ROOT = io_path(Path(__file__).resolve().parents[1])
FIXTURE = ROOT / 'fixtures/xtext/full-generated-until'
SPEC = importlib.util.spec_from_file_location('full_generated_until_verify', FIXTURE / 'verify.py')
VERIFY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFY)


class FullGeneratedUntilEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = VERIFY.load(FIXTURE / 'observations.json')
        cls.cases = VERIFY.load(FIXTURE / 'inputs/cases.json')
        cls.positive = VERIFY.load(FIXTURE / 'positive-resources.json')
        cls.provenance = VERIFY.load(FIXTURE / 'provenance.json')

    def test_all_frozen_bytes_exact_sources_and_complete_denominator(self):
        self.assertEqual(VERIFY.frozen(), VERIFY.EXPECTED_COUNTS)
        self.assertEqual(len(VERIFY.load(FIXTURE / 'inputs/stack.json')['jars']), 60)
        for case in self.cases:
            self.assertEqual(VERIFY.digest(FIXTURE / 'inputs' / case['id'] / 'imported.ecore'), self.provenance['original_ecore_sha256'])
        old = ROOT / 'fixtures/xtext/full-generated-lexical/provenance.json'
        self.assertEqual(VERIFY.digest(old), self.provenance['old_seven_fixture_provenance_sha256_unchanged'])

    def test_complete_raw_cohorts_are_preserved_without_runtime_projection(self):
        rows = []
        for cohort in VERIFY.load(FIXTURE / 'lineage.json')['cohorts']:
            recorded = FIXTURE / 'recorded' / cohort['cohort']
            raw = VERIFY.load(recorded / 'record/observations.json')
            replay = VERIFY.load(recorded / 'replay/observations.json')
            self.assertEqual(VERIFY.digest(recorded / 'record/observations.json'), cohort['record_observations_sha256'])
            self.assertEqual(VERIFY.digest(recorded / 'replay/observations.json'), cohort['replay_observations_sha256'])
            self.assertEqual(VERIFY.comparable(raw, cohort['record_root']), VERIFY.comparable(replay, cohort['replay_root']))
            rows.extend(raw)
        self.assertEqual(rows, self.rows)
        self.assertEqual(len(list((FIXTURE / 'emitted').rglob('*Lexer.java'))), 18)

    def test_original_failed_graphs_retain_stage_error_and_unexecuted_requests(self):
        failed = [row for row in self.rows if row['id'] in VERIFY.FAILED_GRAPHS]
        self.assertEqual(len(failed), 2)
        self.assertEqual(sum(len(row['original_inputs']) for row in failed), 24)
        for row in failed:
            self.assertEqual(row['runtime'], [])
            self.assertIsNone(row['compilation'])
            self.assertEqual(row['generation']['exit_code'], 1)
        repaired = copy.deepcopy(self.rows)
        row = next(row for row in repaired if row['id'] in VERIFY.FAILED_GRAPHS)
        for source in row['metadata']['observation']['validation']:
            source['issues'] = [issue for issue in source['issues'] if issue['severity'] != 'ERROR']
        with self.assertRaisesRegex(ValueError, 'repaired or changed'):
            VERIFY.validate(repaired, self.cases, self.positive)

    def test_rejected_partial_models_never_become_accepted_observations(self):
        altered = copy.deepcopy(self.rows)
        partial = next(run['observation'] for row in altered for run in row['runtime'] if not run['observation']['resource']['accepted'] and run['observation']['resource']['model'] is not None)
        partial['resource']['accepted'] = True
        with self.assertRaisesRegex(ValueError, 'mislabeled'):
            VERIFY.validate(altered, self.cases, self.positive)

    def test_owned_genmodel_path_is_the_only_comparison_exception(self):
        roots = self.provenance['recorded_provider_output_roots']
        clean = VERIFY.comparable(self.rows, roots)
        altered = copy.deepcopy(self.rows)
        run = next(run for row in altered for run in row['runtime'])
        run['observation']['lexer']['antlr_syntax_error_count'] += 1
        self.assertNotEqual(clean, VERIFY.comparable(altered, roots))
        for invalid in ['provider.genmodel', 'D:/wrong/provider.genmodel', str(roots[self.rows[0]['id']]) + '/../provider-726b2116570f/provider.genmodel']:
            altered = copy.deepcopy(self.rows)
            altered[0]['provider']['genmodel'] = invalid
            with self.assertRaises(ValueError):
                VERIFY.comparable(altered, roots)
        altered = copy.deepcopy(self.rows)
        altered[0]['provider']['ecore_sha256'] = 'X' * 64
        with self.assertRaises(ValueError):
            VERIFY.comparable(altered, roots)

    def test_exact_source_graphs_and_raw_invalid_stream_discriminators(self):
        indexed = {row['id']: row for row in self.rows}
        def observation(case, text):
            return next(run['observation'] for run in indexed[case]['runtime'] if run['text'] == text)
        direct = observation('direct_until_suffix_c', '@abc')
        fragment = observation('until_fragment_operand_suffix_c', '@abc')
        self.assertTrue(direct['resource']['accepted'])
        self.assertFalse(fragment['resource']['accepted'])
        self.assertEqual(fragment['lexer']['tokens'][0]['channel'], 99)
        self.assertEqual(fragment['lexer']['tokens'][0]['text'], '@abc')
        self.assertEqual(fragment['resource']['errors'][0]['message'], "mismatched character '<EOF>' expecting 'c'")
        block = observation('block_until_fragment_suffix_c', '@abc')
        self.assertEqual(fragment, block)


if __name__ == '__main__':
    unittest.main()
