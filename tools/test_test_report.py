"""Regression controls for explicit rustdoc runs in the shared test reporter."""
import contextlib
import io
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import test_report


class DocRunTests(unittest.TestCase):
    def run_cargo(self, transcript, exit_code):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            process = SimpleNamespace(stdout=io.StringIO(transcript), wait=lambda: exit_code)
            arguments = SimpleNamespace(
                cargo_args=["--", "-p", "intersect-model", "--doc", "CheckedStructure"],
                out=str(root), baseline=None, no_server=True, quiet=True,
            )
            with (
                patch.object(test_report, "worktree_info", return_value={}),
                patch.object(test_report, "report_root", return_value=root),
                patch.object(test_report.subprocess, "Popen", return_value=process) as cargo,
                contextlib.redirect_stdout(io.StringIO()),
            ):
                result = test_report.cmd_run(arguments)
            self.assertEqual(cargo.call_count, 1)
            self.assertNotIn("--no-run", cargo.call_args.args[0])
            return result, json.loads((root / "summary.json").read_text(encoding="utf-8"))

    def test_compile_fail_cases_are_reported_as_passing_tests(self):
        result, report = self.run_cargo(
            "   Doc-tests intersect_model\n"
            "test definitions::CheckedStructure (line 42) - compile fail ... ok\n"
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n", 0,
        )
        self.assertEqual(result, 0)
        self.assertEqual(report["state"], "finished")
        self.assertEqual(report["totals"]["passed"], 1)
        self.assertEqual(report["tests"][0]["status"], "pass")
        self.assertIsNotNone(report["totals"]["build_seconds"])

    def test_failed_ownership_case_retains_failure(self):
        result, report = self.run_cargo(
            "   Doc-tests intersect_model\n"
            "test definitions::CheckedStructure (line 42) - compile fail ... FAILED\n"
            "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n", 1,
        )
        self.assertEqual(result, 1)
        self.assertEqual(report["totals"]["failed"], 1)
        self.assertEqual(report["tests"][0]["status"], "fail")

    def test_compile_error_is_a_build_failure(self):
        result, report = self.run_cargo("error: could not compile intersect-model\n", 101)
        self.assertEqual(result, 101)
        self.assertEqual(report["state"], "build-failed")
        self.assertEqual(report["totals"]["passed"], 0)


if __name__ == "__main__":
    unittest.main()
