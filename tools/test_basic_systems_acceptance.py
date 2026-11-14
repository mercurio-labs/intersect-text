"""Portable acceptance-delivery controls; no real adapter, compiler or Pilot needed."""
from pathlib import Path
import contextlib
import copy
import io
import json
import shutil
import subprocess
import tempfile
import unittest
from unittest import mock

import basic_systems_acceptance as acceptance


class BasicSystemsAcceptanceTests(unittest.TestCase):
    def fixture(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        fixture = root / "fixtures/basic-systems/v1"
        shutil.copytree(acceptance.HERE, fixture)
        self.enterContext(mock.patch.object(acceptance, "REPO", root))
        self.enterContext(mock.patch.object(acceptance, "HERE", fixture))
        return root, fixture

    def run_baseline(self, root, name="baseline"):
        output = root / "target/basic-systems-acceptance" / name
        with contextlib.redirect_stdout(io.StringIO()):
            code = acceptance.main(["--output", str(output)])
        return code, output, acceptance.read(output / "terminal.json")

    def test_missing_upstream_and_adapters_block_all_80_without_execution(self):
        root, _ = self.fixture()
        with mock.patch.object(acceptance.subprocess, "run", side_effect=AssertionError("No adapter may run")):
            code, _, terminal = self.run_baseline(root)
        self.assertEqual(code, 2)
        self.assertEqual(terminal["counts"], {
            "pass": 0, "candidate_pass_pending_review": 0, "blocked": 80, "fail": 0})
        self.assertEqual(terminal["input_postguard"]["state"], "pass")
        self.assertFalse(terminal["input_postguard"]["integrity"]["upstream_verified"])
        self.assertFalse(any(row["semantically_verified"] for row in terminal["results"]))
        self.assertEqual(terminal["gate_closures"], [])

    def test_reused_output_is_rejected_without_overwriting_receipt(self):
        root, _ = self.fixture()
        _, output, _ = self.run_baseline(root)
        before = acceptance.pin(output / "terminal.json")
        with self.assertRaises(FileExistsError):
            acceptance.main(["--output", str(output)])
        self.assertEqual(acceptance.pin(output / "terminal.json"), before)

    def test_changed_source_identity_is_rejected(self):
        _, fixture = self.fixture()
        contract = copy.deepcopy(acceptance.read(fixture / "contract.json"))
        contract["sources"]["kerml/library.kerml"]["sha256"] = "0" * 64
        with self.assertRaises(AssertionError):
            acceptance.integrity(contract)

    def test_observed_grammar_rejection_does_not_satisfy_admission(self):
        root, fixture = self.fixture()
        receipt = root / "synthetic-not-qualification.json"
        acceptance.save(receipt, {"state": "pass", "ok": False, "observation_complete": True})
        inputs = {"admission": {"native": {"kerml": {"receipt": acceptance.pin(receipt)}}}}
        with self.assertRaises(AssertionError):
            acceptance.admission_for(acceptance.read(fixture / "contract.json"), inputs, "native", "kerml")

    def test_matching_adapter_and_truth_role_pins_stay_candidate_unverified(self):
        root, fixture = self.fixture()
        contract = acceptance.read(fixture / "contract.json")
        case = contract["cases"][0]
        fake = root / "synthetic-not-independent-truth.json"
        acceptance.save(fake, {"test_control_only": True})
        response = {"schema": contract["protocol"]["response_schema"], "case_id": case["id"],
                    "lane": "native", "state": "pass", "imported": True, "consumed": True,
                    "observations": [step["expected"] for step in case["steps"]],
                    "source_postguard": {"state": "pass"}, "excluded_result_fields": [],
                    "evidence": [{"role": role, "file": acceptance.pin(fake)} for role in (
                        "complete-raw-model", "complete-raw-result", "complete-raw-diagnostics",
                        "language-context", "native-or-WASM-artifact", "independent-semantic-truth")]}
        child = subprocess.CompletedProcess(["synthetic-never-executed"], 0,
                                            stdout=json.dumps(response).encode())
        with mock.patch.object(acceptance.subprocess, "run", return_value=child):
            result = acceptance.execute_case(contract, case, "native", ["synthetic"], root / "candidate")
        self.assertEqual(result["state"], "candidate_pass_pending_review")
        self.assertFalse(result["imported"])
        self.assertFalse(result["consumed"])
        self.assertFalse(result["semantically_verified"])

    def test_fixture_addition_is_caught_in_terminal_even_when_no_adapter_runs(self):
        root, fixture = self.fixture()
        contract = acceptance.read(fixture / "contract.json")
        ordinary_integrity = acceptance.integrity(contract)

        def mutate(*_):
            (fixture / "unexpected.kerml").write_text("package unexpected;", encoding="utf-8")
            return "Synthetic blocked admission"

        output = root / "target/basic-systems-acceptance/mutated"
        with mock.patch.object(acceptance, "integrity", return_value=ordinary_integrity), \
                mock.patch.object(acceptance, "admission_for", side_effect=mutate), \
                contextlib.redirect_stdout(io.StringIO()):
            code = acceptance.main(["--pilot-root", str(root / "unexecuted-pilot"), "--output", str(output)])
        terminal = acceptance.read(output / "terminal.json")
        self.assertEqual(code, 2)
        self.assertEqual(terminal["input_postguard"]["state"], "fail")
        self.assertEqual(terminal["counts"]["pass"], 0)


if __name__ == "__main__":
    unittest.main()
