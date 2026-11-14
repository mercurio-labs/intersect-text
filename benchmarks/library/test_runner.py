"""Focused controls for corpus identity and failure-safe reporting."""
from pathlib import Path
import sys
import tempfile
import unittest

import run


class RunnerTests(unittest.TestCase):
    def test_deterministic_corpus_and_exact_count(self):
        with tempfile.TemporaryDirectory() as directory:
            first, second = (Path(directory) / name for name in ("a.library", "b.library"))
            self.assertEqual(run.generate(first, 1000), run.generate(second, 1000))
            data = first.read_bytes()
            self.assertNotIn(b"\r", data)
            self.assertEqual(data.count(b"member "), 999)
            self.assertTrue(data.endswith(b"  member m999;\n}\n"))

    def test_rejection_latency_is_not_successful_parse_time(self):
        report = run.report([{"engine": "intersect", "requested_elements": 1000,
                              "status": "rejected", "elapsed_ms": 0.123}])
        self.assertNotIn("0.123", report)
        self.assertIn("rejected | —", report)

    def test_timeout_and_process_failure_are_explicit(self):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "child.log"
            timed = run.measure([sys.executable, "-c", "import time; time.sleep(10)"], 0.1, log)
            self.assertEqual(timed["status"], "timeout")
            failed = run.measure([sys.executable, "-c", "raise SystemExit(7)"], 5, log)
            self.assertEqual(failed["status"], "process_error")
            self.assertEqual(failed["exit_code"], 7)


if __name__ == "__main__":
    unittest.main()
