"""Portable integrity controls for independent Xtext inference evidence."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures/xtext/metamodel-inference"
SPEC = importlib.util.spec_from_file_location("metamodel_inference_oracle", FIXTURE / "verify.py")
ORACLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORACLE)


class XtextMetamodelInferenceEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.provenance, cls.cases, cls.rows = ORACLE.frozen(FIXTURE)
        cls.by_id = {row["id"]: row for row in cls.rows}

    def classifier(self, case, name):
        return next(c for entry in self.by_id[case]["generated_packages"]
                    for c in entry["package"]["classifiers"] if c["name"] == name)

    def test_full_source_evidence_pins_denominators_and_imported_identity(self):
        self.assertEqual((len(self.rows), sum(row["accepted"] for row in self.rows)), (41, 27))
        self.assertEqual(self.provenance["rejected_cases"], 14)
        self.assertTrue(all(row["imported_unchanged"] for row in self.rows))
        original = self.rows[0]["imported_package"]
        self.assertEqual(original["uri"], "urn:intersect:inference:imported")
        self.assertTrue(all(row["imported_package"] == original for row in self.rows))
        mixed = self.classifier("mixed_imported_generated_reference", "Model")
        external = next(feature for feature in mixed["features"] if feature["name"] == "external")
        self.assertEqual(external["type"], original["uri"] + "#//External")
        self.assertFalse(external["containment"])

    def test_partial_metadatas_do_not_award_acceptance(self):
        for name in ["cross_reference_only_type", "generated_terminal_datatype", "multiple_inheritance_lifting"]:
            with self.subTest(case=name):
                row = self.by_id[name]
                self.assertFalse(row["accepted"])
                self.assertGreater(row["error_count"], 0)
                self.assertTrue(any(entry["package"]["classifiers"] for entry in row["generated_packages"]))
        self.assertTrue(self.by_id["multiple_inheritance_without_feature_collision"]["accepted"])
        self.assertEqual(self.classifier("explicit_new_return_type", "Token")["kind"], "EClass")

    def test_independent_current_type_and_enum_identity_controls(self):
        optional = self.classifier("optional_unassigned_call", "A")
        self.assertEqual(optional["super_types"], ["urn:intersect:inference:optional_unassigned_call#//Model"])
        self.assertEqual([f["name"] for f in self.classifier("optional_unassigned_call", "Model")["features"]], ["suffix"])
        union = self.classifier("assigned_model_type_alternative", "Model")
        self.assertEqual(union["features"][0]["type"], "http://www.eclipse.org/emf/2002/Ecore#//EObject")
        literal = self.classifier("enum_duplicate_literal_name", "Shade")["literals"][0]
        self.assertEqual((literal["name"], literal["literal"], literal["value"]), ("RED", "red", 0))
        self.assertEqual(len(literal["origins"]), 2)
        common = self.classifier("alternative_supertypes_and_lifting", "Common")
        self.assertEqual([f["name"] for f in common["features"]], ["name"])
        self.assertEqual(self.classifier("alternative_supertypes_and_lifting", "A")["features"], [])

    def test_corrupt_inputs_outcomes_and_origins_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "changed.json"
            path.write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                ORACLE.checked(path, self.provenance["files"]["observations.json"])
        changes = [
            lambda rows: rows[0].update(accepted=False),
            lambda rows: rows[0].update(imported_unchanged=False),
            lambda rows: rows[0]["generated_packages"][0]["package"]["classifiers"][0].update(identity="forged#//Model"),
            lambda rows: rows[0]["generated_packages"][0]["package"]["classifiers"][0]["features"][0].update(type=None),
            lambda rows: rows[0]["generated_packages"][0]["package"]["classifiers"][0]["features"][0]["origins"][0].update(end=10**9),
        ]
        for index, change in enumerate(changes):
            with self.subTest(change=index):
                rows = copy.deepcopy(self.rows)
                change(rows)
                with self.assertRaises(ValueError):
                    ORACLE.validate_rows(self.cases, rows)
        with self.assertRaisesRegex(ValueError, "order/identity"):
            ORACLE.validate_rows(self.cases, list(reversed(self.rows)))


if __name__ == "__main__":
    unittest.main()
