"""Integrity and phase controls for the independent guard-sequence checkpoint."""
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/xtext/guard-sequences"
SPEC = importlib.util.spec_from_file_location("guard_evidence", FIXTURE / "verify.py")
PROOF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROOF)


class GuardSequenceEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.cases, self.observed, self.before = [json.loads((FIXTURE / name).read_text(encoding="utf-8"))
            for name in ("cases.json", "observations.json", "native-before.json")]

    def rejected(self):
        with self.assertRaises((AssertionError, ValueError, KeyError)):
            PROOF.validate(self.cases, self.observed, self.before)

    def test_frozen_complete_stages_and_known_native_defect(self):
        provenance, _, _, _, counts = PROOF.frozen(ROOT)
        self.assertEqual(counts, PROOF.COUNTS)
        self.assertFalse(provenance["native_support"])
        self.assertEqual(counts["ordinary_requests"] + counts["inferred_requests"], 135)

    def test_changed_source_or_uri_cannot_keep_qualification(self):
        for key, value in [("source", "changed"), ("uri", "memory:/wrong/entry.xtext")]:
            with self.subTest(key=key):
                cases = copy.deepcopy(self.cases)
                cases["ordinary"][0]["sources"][0][key] = value
                with self.assertRaises(AssertionError):
                    PROOF.validate(cases, self.observed, self.before)

    def test_unexecuted_requests_cannot_be_discarded(self):
        self.observed["ordinary"]["unexecuted_requests"].pop()
        self.rejected()

    def test_generator_failure_is_not_metadata_rejection(self):
        row = next(r for r in self.observed["ordinary"]["generation"] if r["id"] == "root_false")
        row["stage"] = "metadata_rejected"
        self.rejected()

    def test_expected_generator_failure_requires_exact_diagnostics(self):
        row = next(r for r in self.observed["ordinary"]["generation"] if r["id"] == "root_false")
        row["diagnostics"] = ["unrelated compiler failure"]
        self.rejected()

    def test_dead_guard_arity_rejection_cannot_be_called_compiled(self):
        row = next(r for r in self.observed["ordinary"]["generation"] if r["id"] == "dead_arity_false")
        row["stage"] = "compiled"
        self.rejected()

    def test_imported_unchanged_flag_cannot_hide_structural_mutation(self):
        self.observed["ordinary"]["metadata"][0]["imported_after"][0]["name"] = "changed"
        self.rejected()

    def test_provider_artifact_identity_cannot_be_substituted(self):
        self.observed["ordinary"]["generation"][0]["provider"]["generation"]["ecore_sha256"] = {}
        self.rejected()

    def test_provider_source_identity_is_separate_from_ecore_identity(self):
        self.observed["ordinary"]["generation"][0]["provider"]["generation"]["source_sha256"] = {}
        self.rejected()

    def test_baseline_request_identity_cannot_be_omitted(self):
        self.before.pop()
        self.rejected()

    def test_no_request_can_disappear(self):
        self.observed["ordinary"]["runs"].pop()
        self.rejected()

    def test_error_cannot_become_error_free_rootless_success(self):
        row = next(r for r in self.observed["ordinary"]["runs"] if not r["accepted"])
        row.update(errors=[], exception=None, model=None)
        self.rejected()

    def test_baseline_native_defect_is_not_relabeled_as_desired_behavior(self):
        self.before[0]["diagnostics"] = []
        self.rejected()

    def test_inferred_models_require_actual_generation(self):
        self.observed["inferred"]["generation"][0]["generation_exit"] = 1
        self.rejected()

    def test_component_pin_covers_complete_partial_models(self):
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory)
            for source in FIXTURE.iterdir():
                if source.is_file():
                    shutil.copyfile(source, destination / source.name)
            changed = copy.deepcopy(self.observed)
            row = next(r for r in changed["ordinary"]["runs"] if r["model"])
            row["model"][0]["class"] = "substituted::Model"
            (destination / "observations.json").write_text(json.dumps(changed), encoding="utf-8")
            with patch.object(PROOF, "HERE", destination), self.assertRaises(AssertionError):
                PROOF.frozen(ROOT)


if __name__ == "__main__":
    unittest.main()
