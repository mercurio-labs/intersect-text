"""Exact additive X05 source/evidence checks; no Java/build/runtime qualification."""
from pathlib import Path
import hashlib
import json
import unittest

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / 'fixtures/xtext/terminal-negation'


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


class TerminalNegationEvidence(unittest.TestCase):
    def setUp(self):
        self.native = json.loads((FIXTURE / 'native-cases.json').read_text(encoding='utf-8'))
        self.authored = json.loads((FIXTURE / 'cases.json').read_text(encoding='utf-8'))
        self.observed = json.loads((FIXTURE / 'observations.json').read_text(encoding='utf-8'))

    def test_original_fixture_bytes_are_unchanged(self):
        self.assertEqual(len(self.native['original_fixture_files']), 28)
        for row in self.native['original_fixture_files']:
            raw = (ROOT / row['path']).read_bytes()
            self.assertEqual(len(raw), row['bytes'], row['path'])
            self.assertEqual(digest(raw), row['sha256'], row['path'])

    def test_exact_original_graphs_have_real_content_identities_and_dependencies(self):
        self.assertEqual(len(self.native['cases']), 22)
        self.assertEqual(len(self.native['nested_companions']), 2)
        self.assertEqual(digest((FIXTURE / 'nested-cases.json').read_bytes()), self.native['nested_cases_sha256'])
        self.assertEqual(digest((FIXTURE / 'cases.json').read_bytes()), self.native['oracle_cases_sha256'])
        nested = json.loads((FIXTURE / 'nested-cases.json').read_text(encoding='utf-8'))
        originals = {case['id']: case for case in self.authored + nested}
        for case in self.native['cases'] + self.native['nested_companions']:
            graph = case['compilation']['bundle']
            identities = {(s['id']['uri'], s['id']['sha256']) for s in graph['sources']}
            for source in graph['sources']:
                self.assertEqual(digest(source['text'].encode('utf-8')), source['id']['sha256'])
                for dependency in source['dependencies']:
                    self.assertIn((dependency['uri'], dependency['sha256']), identities)
            original = originals[case['id']]
            retained = [s['text'] for s in graph['sources'] if s['kind'] == 'xtext']
            self.assertEqual(retained, [s['text'] for s in original['sources']])

    def test_six_original_before_results_keep_the_original_profile_and_full_graphs(self):
        self.assertEqual(len(self.native['original_before']), 6)
        self.assertEqual(self.native['before_wasm_sha256'], '8ec9053548fbd0a461376b6a8f70521074e7f05a1043cc66a1496d1c089c8f6c')
        self.assertEqual([row['id'] for row in self.native['original_before']], ['old_character_negation','range_negation','fragment_negation','alternative_until','fragment_until','local_hidden'])
        for row in self.native['original_before']:
            self.assertEqual(row['request']['xtext_profile'], 'intersect.xtext.native.v56')
            self.assertEqual(row['request']['method'], 'compile_bundle')
            self.assertEqual(row['result']['ok'], row['id'] == 'old_character_negation')

    def test_upstream_success_and_inconclusive_timeout_are_retained(self):
        observed = {row['id']: row for row in self.observed}
        for name in ['multichar', 'astral_literal']:
            row = observed[name]
            self.assertEqual(row['generation']['exit_code'], 0)
            self.assertTrue(any(run.get('complete_single_value') for run in row['runtime']))
            self.assertTrue(all(not source['errors'] for source in row['syntax']))
            self.assertTrue(all(not source['errors'] for source in row['linking']))
        self.assertEqual(observed['sequence']['generation']['exit_code'], 1)
        timeout = [run for run in observed['negated_repeat']['runtime'] if 'timeout_seconds' in run]
        self.assertEqual([row['text'] for row in timeout], ['AbC'])
        self.assertTrue(all('complete_single_value' not in row for row in timeout))


if __name__ == '__main__':
    unittest.main()
