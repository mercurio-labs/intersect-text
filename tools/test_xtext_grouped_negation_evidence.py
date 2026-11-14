"""Portable evidence checks; no Java or native execution is required."""
import argparse
import copy
import importlib.util
import json
import sys
import unittest
from pathlib import Path

FIXTURE = Path(__file__).resolve().parents[1] / 'fixtures/xtext/grouped-terminal-negation'


class GroupedNegationEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location('grouped_negation_verify', FIXTURE / 'verify.py')
        cls.top = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.top)
        cls.verify = cls.top.base

    def test_all_frozen_raw_artifacts_and_complete_observations(self):
        _, _, counts, pins = self.top.frozen()
        self.assertEqual(counts, {'graphs': 5, 'planned': 40, 'runtime': 40, 'accepted': 15, 'rejected': 25, 'rejected_partial': 18})
        self.assertEqual(pins, 374)

    def test_original_sequence_and_exact_source_envelopes_are_not_repaired(self):
        cases = self.verify.load(FIXTURE / 'inputs/cases.json')
        native = self.verify.load(FIXTURE / 'native-cases.json')['cases']
        self.assertEqual([c['id'] for c in native], [c['id'] for c in cases])
        for source_case, native_case in zip(cases, native):
            compilation = native_case['compilation']
            for source in source_case['sources']:
                data = self.verify.raw(FIXTURE / 'inputs' / source_case['id'] / source['file'])
                self.assertEqual(self.verify.hashlib.sha256(data).hexdigest(), source['source_id']['sha256'])
                retained = next(s for s in compilation['bundle']['sources'] if s['id'] == source['source_id'])
                self.assertEqual(retained['text'].encode('utf-8'), data)
        sequence = cases[0]
        self.assertEqual(sequence['id'], 'original_sequence')
        self.assertEqual(sequence['sources'][0]['source_id']['sha256'], 'f0fe667706a9369837f459ee3ccbc54fa48fe940866a9b5a93a1068a293eed07')

    def test_recovered_generation_diagnostics_and_partial_models_remain_visible(self):
        for key in ['record', 'replay']:
            root = FIXTURE / 'oracle' / key
            rows = self.verify.load(root / 'observations.json')
            commands = [c for c in self.verify.load(root / 'commands.json') if 'org.eclipse.emf.mwe2.launch.runtime.Mwe2Launcher' in c['command']]
            self.assertEqual(len(commands), 5)
            for row, command in zip(rows, commands):
                output = self.verify.raw(root / command['stdout']) + self.verify.raw(root / command['stderr'])
                self.assertEqual(command['exit_code'], 0)
                if row['id'] == 'original_sequence':
                    self.assertIn(b'error(100)', output)
                    self.assertIn(b'EOA', output)
                elif row['id'] == 'fragment_mixed':
                    self.assertIn(b'error(100)', output)
                    self.assertIn(b'INVALID', output)
                else:
                    self.assertNotIn(b'error(100)', output)
            qualified = {'grouped_multichar', 'fragment_multichar', 'mixed_group'}
            inputs = [run for row in rows if row['id'] in qualified for run in row['runtime']]
            self.assertEqual(len(inputs), 27)
            self.assertEqual(sum(run['observation']['resource']['accepted'] for run in inputs), 11)

    def test_genmodel_comparison_exception_cannot_hide_unowned_paths(self):
        provenance = self.verify.load(FIXTURE / 'provenance.json')
        rows = self.verify.load(FIXTURE / 'observations.json')
        root = provenance['original_output_roots']['record']
        self.verify.comparable(rows, root)
        for value in ['relative/provider.genmodel', 'D:/outside/provider.genmodel', root + '/provider-000000000000/provider.genmodel', root + '/../provider-726b2116570f/provider.genmodel']:
            forged = copy.deepcopy(rows)
            forged[0]['provider']['genmodel'] = value
            with self.assertRaises(ValueError):
                self.verify.comparable(forged, root)


    def test_expanded_exact_sources_and_successful_models_remain_complete(self):
        actual = self.verify.load(FIXTURE / 'expanded-native-cases.json')['cases']
        originals = []
        observations = []
        for name in ['compositions', 'exact-negatives']:
            cohort = FIXTURE / 'expansion' / name
            cases = self.verify.load(cohort / 'inputs/cases.json')
            originals.extend(cases)
            rows, counts, positives = self.top.validate_extra(cohort / 'oracle/record', cohort, cases)
            self.assertEqual(counts, self.top.COHORTS[name])
            observations.extend(rows)
        self.assertEqual([c['id'] for c in actual], [c['id'] for c in originals])
        self.assertEqual(len(actual), 10)
        self.assertEqual(sum(len(r['runtime']) for r in observations), 90)
        self.assertEqual(len(self.verify.load(FIXTURE / 'expanded-positive-resources.json')), 31)
        for source_case, native_case in zip(originals, actual):
            cohort = 'compositions' if source_case['id'] in {c['id'] for c in originals[:6]} else 'exact-negatives'
            for source in source_case['sources']:
                data = self.verify.raw(FIXTURE / 'expansion' / cohort / 'inputs' / source_case['id'] / source['file'])
                self.assertEqual(self.verify.hashlib.sha256(data).hexdigest(), source['source_id']['sha256'])
                retained = next(s for s in native_case['compilation']['bundle']['sources'] if s['id'] == source['source_id'])
                self.assertEqual(retained['text'].encode('utf-8'), data)

    def test_original_profiles_and_private_negative_observations_are_retained(self):
        before = self.verify.load(FIXTURE / 'lineage/private-attempt4-negatives.json')
        self.assertEqual(len(before['rows']), 4)
        self.assertTrue(all(row['compile']['ok'] is False for row in before['rows']))
        complete = []
        for relative in ['lineage/compositions-before-v61.json', 'lineage/exact-negatives-before-v61.json']:
            before = self.verify.load(FIXTURE / relative)
            complete.extend(before['transcript'])
        self.assertEqual(len(complete), 20)
        for row in complete:
            request = row['request']
            if request['method'] == 'compile_bundle':
                self.assertEqual(request['xtext_profile'], 'intersect.xtext.native.v61')
                self.assertIs(row['result']['ok'], False)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--fixture-root', type=Path)
    options, remaining = parser.parse_known_args()
    if options.fixture_root is not None:
        FIXTURE = options.fixture_root.resolve()
    unittest.main(argv=[sys.argv[0], *remaining])
