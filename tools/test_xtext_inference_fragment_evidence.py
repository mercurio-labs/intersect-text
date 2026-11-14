"""Integrity and semantic controls for frozen parser-fragment inference evidence."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures/xtext/inference-fragments"
SPEC = importlib.util.spec_from_file_location("fragment_inference_oracle", FIXTURE / "verify.py")
ORACLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORACLE)


class FragmentInferenceEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.provenance, cls.cases, cls.rows = ORACLE.frozen(FIXTURE)
        cls.by_id = {r["id"]: r for r in cls.rows}
        cls.imported = json.loads((FIXTURE.parent / "metamodel-inference/observations.json").read_text(encoding="utf-8"))[0]["imported_package"]

    def classifiers(self, name):
        return {c["name"]: c for m in self.by_id[name]["sources"][-1]["metamodels"] if m["generated"] for c in m["package"]["classifiers"]}

    def test_complete_graphs_preserve_invalid_unused_ancestors(self):
        self.assertEqual((len(self.rows), sum(r["all_sources_accepted"] for r in self.rows), sum(r["accepted"] for r in self.rows)), (45, 28, 30))
        self.assertEqual(sum(len(r["sources"]) for r in self.rows), 60)
        for name in ("wildcard_library_parent", "invalid_unused_parent"):
            row = self.by_id[name]
            self.assertTrue(row["accepted"])
            self.assertFalse(row["all_sources_accepted"])
            self.assertGreater(row["sources"][0]["error_count"], 0)

    def test_typed_fragments_define_return_classes_and_inheritance(self):
        classes = self.classifiers("typed_basic")
        self.assertEqual(classes["Model"]["super_types"], ["urn:inference:fragments:typed_basic#//Fields"])
        self.assertEqual([f["name"] for f in classes["Fields"]["features"]], ["name"])
        nested = self.classifiers("typed_nested")
        self.assertEqual([s.rsplit("/", 1)[-1] for s in nested["Model"]["super_types"]], ["Outer", "Inner"])
        shared = self.classifiers("typed_shared_return")
        self.assertEqual([f["name"] for f in shared["Common"]["features"]], ["name", "tag"])
        scalar = self.by_id["typed_datatype_unassigned"]["sources"][-1]["rules"][1]
        self.assertTrue(scalar["fragment"])
        self.assertEqual(scalar["type"], "http://www.eclipse.org/emf/2002/Ecore#//EString")

    def test_wildcard_fragments_have_no_classifier_and_keep_caller_projection(self):
        for name in ("wildcard_basic", "wildcard_nested"):
            classes = self.classifiers(name)
            self.assertEqual(list(classes), ["Model"])
            self.assertEqual([f["name"] for f in classes["Model"]["features"]], ["name"])
            for rule in self.by_id[name]["sources"][-1]["rules"]:
                if rule["wildcard"]:
                    self.assertTrue(rule["fragment"])
                    self.assertIsNone(rule["type"])
        classes = self.classifiers("wildcard_typed_call")
        self.assertEqual(classes["Model"]["super_types"], [])
        self.assertEqual(classes["Model"]["features"], [])
        self.assertEqual([f["name"] for f in classes["Common"]["features"]], ["name"])

    def test_synthetic_default_reference_calls_keep_exact_target_and_no_span(self):
        calls = [c for r in self.rows for s in r["sources"] for rule in s["rules"] for c in rule["calls"] if c["synthetic"]]
        self.assertEqual(len(calls), 2)
        for call in calls:
            self.assertIsNone(call["start"])
            self.assertIsNone(call["end"])
            self.assertFalse(call["proxy"])
            self.assertEqual(call["target"]["name"], "ID")
            self.assertEqual(call["target"]["type"], "http://www.eclipse.org/emf/2002/Ecore#//EString")
            self.assertFalse(call["target"]["fragment"])

    def test_parent_imported_metadata_and_rejection_categories_are_preserved(self):
        parents = [p for r in self.rows for p in r["parent_snapshots"]]
        self.assertEqual(len(parents), 15)
        self.assertTrue(all(p["metadata_unchanged"] and p["structure_unchanged"] for p in parents))
        for row in self.rows:
            self.assertTrue(row["imported_unchanged"])
            self.assertEqual(row["imported_package"], self.imported)
        for name in ("typed_assigned", "wildcard_assigned", "typed_override_new_type", "typed_to_wildcard_override", "wildcard_to_typed_override", "production_to_fragment", "fragment_to_production", "wildcard_conflicting_types"):
            self.assertFalse(self.by_id[name]["all_sources_accepted"])

    def test_verifier_rejects_corrupt_outcomes_targets_flags_spans_and_snapshots(self):
        changes = [
            lambda rows: rows[0].update(all_sources_accepted=False),
            lambda rows: rows[0]["sources"][0]["rules"][0]["calls"][0]["target"].update(fragment=False),
            lambda rows: rows[0]["sources"][0]["rules"][0].update(end=10**9),
            lambda rows: rows[0]["sources"][0]["rules"][0]["calls"][0].update(synthetic=True),
            lambda rows: rows[16]["sources"][0]["rules"][1].update(type="fabricated#//Fields"),
            lambda rows: rows[28]["parent_snapshots"][0].update(metadata_unchanged=False),
        ]
        for index, change in enumerate(changes):
            with self.subTest(change=index):
                rows = copy.deepcopy(self.rows)
                change(rows)
                with self.assertRaises(ValueError):
                    ORACLE.validate_rows(self.cases, rows, self.imported)
        with self.assertRaisesRegex(ValueError, "order/identity"):
            ORACLE.validate_rows(self.cases, list(reversed(self.rows)), self.imported)


if __name__ == "__main__":
    unittest.main()
