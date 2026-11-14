"""Supplemental construction outcomes distinguish empty resources from errors."""
import copy
import importlib.util
from pathlib import Path
import unittest

FIXTURE = Path(__file__).resolve().parent.parent / 'fixtures/xtext/inference-parameter-empty'
SPEC = importlib.util.spec_from_file_location('parameter_empty_evidence', FIXTURE / 'verify.py')
ORACLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORACLE)


class EmptyResourceEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cases, cls.rows, cls.counts = ORACLE.frozen(FIXTURE)
        cls.base, cls.shared = ORACLE.base(FIXTURE)

    def validate(self, cases, rows):
        ORACLE.validate(self.base, cases, rows, self.shared.validate)

    def test_empty_resource_is_distinct_from_rooted_success_and_syntax_error(self):
        self.assertEqual(self.counts, {'graphs': 2, 'requests': 4, 'rooted_successes': 1,
                                     'syntax_errors': 2, 'error_free_empty_resources': 1})
        empty = next(r for r in self.rows['runs'] if not r['errors'] and r['model'] is None)
        self.assertFalse(empty['accepted'])
        self.assertEqual(empty['resource_contents'], 0)
        self.assertIsNone(empty['exception'])

    def test_empty_observation_cannot_acquire_a_root_or_success_label(self):
        for field, value in [('accepted', True), ('resource_contents', 1), ('entry', 'Node')]:
            rows = copy.deepcopy(self.rows)
            row = next(r for r in rows['runs'] if not r['errors'] and r['model'] is None)
            row[field] = value
            with self.assertRaises(ValueError):
                self.validate(self.cases, rows)

    def test_omitted_request_and_failed_generation_are_rejected(self):
        rows = copy.deepcopy(self.rows)
        rows['runs'].pop()
        with self.assertRaises(ValueError):
            self.validate(self.cases, rows)
        rows = copy.deepcopy(self.rows)
        rows['generation'][0]['generation_exit'] = 1
        with self.assertRaises(ValueError):
            self.validate(self.cases, rows)

    def test_ordinary_rooted_and_error_outcomes_keep_shared_validation(self):
        rows = copy.deepcopy(self.rows)
        rows['runs'][0]['model'] = None
        with self.assertRaises(ValueError):
            self.validate(self.cases, rows)
        rows = copy.deepcopy(self.rows)
        row = rows['runs'][1]
        row['errors'] = []
        row['exception'] = None
        with self.assertRaises(ValueError):
            self.validate(self.cases, rows)


if __name__ == '__main__':
    unittest.main()
