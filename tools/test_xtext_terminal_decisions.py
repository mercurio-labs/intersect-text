"""Offline integrity for authored terminal controls and independent observations."""
import hashlib
import json
from pathlib import Path
import unittest

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures/xtext/terminal-decisions"

class TerminalDecisionProvenance(unittest.TestCase):
    def test_harness_inputs_and_observations_match_recorded_hashes(self):
        provenance = json.loads((FIXTURE / "provenance.json").read_text())
        for name, digest in provenance["files"].items():
            with self.subTest(file=name):
                self.assertEqual(hashlib.sha256((FIXTURE / name).read_bytes()).hexdigest(), digest)

    def test_oracle_failures_and_eof_difference_remain_explicit(self):
        rows = json.loads((FIXTURE / "observations.json").read_text())
        self.assertEqual(len(rows), 33)
        eof = [r for r in rows if r["body"] == "'@' . EOF" and r["text"] == "@"]
        self.assertEqual(len(eof), 1)
        self.assertTrue(eof[0]["accepted"])
        failures = json.loads((FIXTURE / "generation-rejections.json").read_text())
        self.assertEqual(len(failures), 1)
        self.assertEqual(failures[0]["body"], "'@' (.)+ EOF")
        self.assertIn("error(201)", failures[0]["diagnostic"])

if __name__ == "__main__":
    unittest.main()
