"""Adversarial checks for the conformance inventory, not language conformance."""

import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest

import text_definition_ledger as ledger


class InventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.pins = ledger.verify_artifacts(ledger.FIXTURES)

    def temporary_fixtures(self):
        temporary = tempfile.TemporaryDirectory(prefix="intersect-ledger-")
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name) / "fixtures"
        shutil.copytree(ledger.FIXTURES, root)
        return root

    def write_json(self, root, path, value):
        (root / path).write_text(json.dumps(value), encoding="utf-8")

    def test_pinned_denominators_and_overloads(self):
        result = ledger.build()
        rows = result["obligations"]
        self.assertEqual(sum(r["domain"] == "ecore" and r["kind"] == "classifier" for r in rows), 53)
        self.assertEqual(sum(r["domain"] == "xtext" and r["kind"] == "classifier" for r in rows), 36)
        self.assertEqual(sum(r["kind"] == "grammar_rule" for r in rows), 64)
        overloaded = [r for r in rows if r["title"] == "EClass.getEStructuralFeature" and r["kind"] == "eOperations"]
        self.assertEqual(len(overloaded), 2)
        self.assertEqual(len({r["id"] for r in overloaded}), 2)
        self.assertEqual(len({r["source"]["selector"] for r in overloaded}), 2)
        for row in rows:
            self.assertEqual(row["semantically_verified"], "not_assessed")
            self.assertEqual(row["verification_evidence"], [])
        oracle = result["portable_oracle"]
        self.assertEqual((oracle["positive"], oracle["syntax_negative"], oracle["nodes"], oracle["reference_slots"]), (90, 8, 1403, 2595))
        self.assertEqual(oracle["intersect_execution"], "not_run")

    def test_tampered_source_rejected(self):
        root = self.temporary_fixtures()
        with (root / ledger.ECORE).open("ab") as output:
            output.write(b"\n")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            ledger.build(root)

    def test_reordered_oracle_cases_rejected(self):
        root = self.temporary_fixtures()
        data = ledger.read_json(root / "mercurio/pilot-controls.json")
        data["controls"][0], data["controls"][1] = data["controls"][1], data["controls"][0]
        self.write_json(root, "mercurio/pilot-controls.json", data)
        with self.assertRaisesRegex(ValueError, "Oracle source mismatch"):
            ledger.oracle_cases(root, self.pins)

    def test_false_positive_oracle_stage_rejected(self):
        root = self.temporary_fixtures()
        data = ledger.read_json(root / "mercurio/pilot-controls.json")
        next(c for c in data["controls"] if not c["accepted"])["accepted"] = True
        self.write_json(root, "mercurio/pilot-controls.json", data)
        with self.assertRaisesRegex(ValueError, "stage/language mismatch"):
            ledger.oracle_cases(root, self.pins)

    def test_missing_observation_rejected(self):
        root = self.temporary_fixtures()
        data = ledger.read_json(root / "mercurio/pilot-controls.json")
        data["controls"][0]["links"].pop()
        self.write_json(root, "mercurio/pilot-controls.json", data)
        with self.assertRaisesRegex(ValueError, "denominator changed"):
            ledger.oracle_cases(root, self.pins)

    def test_unknown_classifier_needs_owner(self):
        root = self.temporary_fixtures()
        path = root / ledger.XTEXT
        text = path.read_text(encoding="utf-8").replace('name="Grammar"', 'name="NewGrammar"', 1)
        path.write_text(text, encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "Unowned upstream classifier"):
            ledger.metamodel_obligations(root, ledger.XTEXT, "xtext", self.pins)

    def test_duplicate_service_identity_rejected(self):
        root = self.temporary_fixtures()
        data = ledger.read_json(root / "extension-obligations.json")
        data["obligations"].append(copy.deepcopy(data["obligations"][0]))
        self.write_json(root, "extension-obligations.json", data)
        with self.assertRaisesRegex(ValueError, "Duplicate obligation identity"):
            ledger.build(root)

    def test_missing_service_selector_rejected(self):
        root = self.temporary_fixtures()
        data = ledger.read_json(root / "extension-obligations.json")
        data["obligations"][0]["selector"] = "MissingDelegateThatDoesNotExist"
        self.write_json(root, "extension-obligations.json", data)
        with self.assertRaisesRegex(ValueError, "Missing service source selector"):
            ledger.build(root)

    def test_path_escape_rejected(self):
        for path in ["../secret", "/etc/passwd", "C:/secret", "..\\secret"]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                ledger.local_path(ledger.FIXTURES, path)


if __name__ == "__main__":
    unittest.main()
