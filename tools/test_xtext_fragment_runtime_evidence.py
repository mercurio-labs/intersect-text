"""Independent generated parser, caller, conversion and partial-model evidence."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures/xtext/inference-fragment-runtime"
SPEC = importlib.util.spec_from_file_location("fragment_construction_oracle", FIXTURE / "verify.py")
ORACLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORACLE)


class FragmentConstructionEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.provenance, cls.cases, cls.rows = ORACLE.frozen(FIXTURE)
        cls.metadata = {r["id"]: r for r in cls.rows["metadata"]}
        cls.generation = {r["id"]: r for r in cls.rows["generation"]}

    def run_row(self, case, text=None):
        return next(r for r in self.rows["runs"] if r["case"] == case and (text is None or r["text"] == text))

    def test_generation_compilation_and_runtime_denominators_are_separate(self):
        self.assertEqual(len(self.cases), 10)
        self.assertEqual(sum(len(c["sources"]) for c in self.cases), 13)
        self.assertEqual(sum(r["all_sources_accepted"] for r in self.rows["metadata"]), 10)
        self.assertEqual(sum(r["stage"] == "compiled" for r in self.rows["generation"]), 9)
        self.assertEqual((len(self.rows["runs"]), sum(r["accepted"] for r in self.rows["runs"])), (13, 7))

    def test_original_sources_and_complete_metadata_are_not_replaced_by_controls(self):
        earlier = {r["id"]: r for r in json.loads((FIXTURE.parent / "inference-fragments/observations.json").read_text(encoding="utf-8"))}
        originals = [c for c in self.cases if "origin_case" in c]
        self.assertEqual(len(originals), 4)
        for case in originals:
            self.assertEqual(self.metadata[case["id"]], earlier[case["origin_case"]])
        failure = self.generation["wildcard_override"]
        self.assertEqual(failure["stage"], "generation_failed")
        self.assertIn("TypeReference.package_name:inference.fragments.WildcardOverride.services.ParentGrammarAccess", failure["error_keys"])
        self.assertEqual(self.generation["wildcard_override_lowercase"]["stage"], "compiled")

    def test_compatible_wildcard_typed_call_needs_features_not_nominal_inheritance(self):
        row = self.run_row("wildcard_typed_compatible")
        self.assertTrue(row["accepted"])
        self.assertEqual(row["model"][0]["values"], {"name": "beta"})
        declarations = self.metadata["wildcard_typed_compatible"]["sources"][0]["metamodels"]
        classifiers = {c["name"]: c for d in declarations if d["generated"] for c in d["package"]["classifiers"]}
        self.assertEqual(classifiers["Model"]["super_types"], [])
        self.assertEqual([f["name"] for f in classifiers["Model"]["features"]], ["name"])
        self.assertEqual([f["name"] for f in classifiers["Common"]["features"]], ["name"])
        self.assertNotIn("Fields", classifiers)

    def test_missing_feature_is_a_runtime_failure_after_successful_generation(self):
        self.assertEqual(self.generation["wildcard_typed_call"]["stage"], "compiled")
        row = self.run_row("wildcard_typed_call")
        self.assertFalse(row["accepted"])
        self.assertEqual(row["resource_contents"], 0)
        self.assertIsNone(row["model"])
        self.assertEqual(row["exception"][-1], {"class": "java.lang.IllegalArgumentException", "message": "Model.name does not exist"})

    def test_emitted_entry_closure_and_reachable_override_preserve_parent_schema(self):
        entries = self.generation["wildcard_override_lowercase"]["parser_entries"]
        self.assertEqual(entries["inference.fragments.wildcardoverride.Child"], ["Model"])
        self.assertEqual(entries["inference.fragments.wildcardoverride.Parent"], ["Root"])
        entries = self.generation["wildcard_override_reachable"]["parser_entries"]
        self.assertEqual(entries["inference.fragments.wildcardoverride.Reachable"], ["Holder", "Root"])
        accepted = self.run_row("wildcard_override_reachable", "r alpha")
        self.assertTrue(accepted["accepted"])
        self.assertEqual(accepted["model"][0]["values"]["node"]["object"], 1)
        self.assertEqual(accepted["model"][1]["values"], {"name": "alpha"})
        rejected = self.run_row("wildcard_override_reachable", "r alpha tag beta")
        self.assertFalse(rejected["accepted"])
        self.assertEqual(rejected["exception"][-1]["message"], "Root.tag does not exist")
        unavailable = self.run_row("wildcard_override_lowercase", "r alpha")
        self.assertEqual(unavailable["exception"][-1]["class"], "java.lang.NoSuchMethodException")

    def test_integer_features_do_not_convert_wildcard_strings_and_partial_models_are_explicit(self):
        for name, expected in (("wildcard_typed_incompatible", {"name": 42}), ("wildcard_numeric_string", {"number": 0})):
            with self.subTest(case=name):
                row = self.run_row(name)
                self.assertFalse(row["accepted"])
                self.assertIsNone(row["exception"])
                self.assertEqual(row["resource_contents"], 1)
                self.assertEqual(row["model"][0]["values"], expected)
                self.assertIn("java.lang.String cannot be cast to class java.lang.Integer", row["errors"][0]["message"])

    def test_unused_fragment_body_is_retained_without_a_phantom_entry(self):
        self.assertTrue(self.metadata["unused_assigned_fragment"]["all_sources_accepted"])
        self.assertEqual(self.generation["unused_assigned_fragment"]["parser_entries"], {"inference.fragments.runtime.UnusedAssigned": ["Model"]})
        self.assertEqual([r["name"] for r in self.metadata["unused_assigned_fragment"]["sources"][0]["rules"]][:3], ["Model", "Fields", "Common"])
        self.assertEqual(self.run_row("unused_assigned_fragment")["model"][0]["values"], {"name": "alpha"})

    def test_verifier_rejects_corrupt_sources_stages_requests_models_and_inventories(self):
        def reference(rows):
            row = next(r for r in rows["runs"] if r["case"] == "wildcard_override_reachable" and r["accepted"])
            row["model"][0]["values"]["node"]["object"] = 999
        changes = [
            lambda rows: rows["generation"][0].update(source_unchanged=False),
            lambda rows: rows["generation"][3].update(generation_exit=0),
            lambda rows: rows["runs"][0].update(text="different"),
            lambda rows: rows["runs"][0].update(exception=[{"class": "failure"}]),
            lambda rows: rows["generation"][4]["parser_entries"].clear(),
            reference,
        ]
        for index, change in enumerate(changes):
            with self.subTest(change=index):
                rows = copy.deepcopy(self.rows)
                change(rows)
                with self.assertRaises(ValueError):
                    ORACLE.validate(FIXTURE, self.cases, rows)
        cases = copy.deepcopy(self.cases)
        cases[0]["sources"][0]["source"] += "// altered original"
        with self.assertRaises(ValueError):
            ORACLE.validate(FIXTURE, cases, self.rows)


if __name__ == "__main__":
    unittest.main()
