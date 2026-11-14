"""Portable integrity controls for frozen multi-source inference observations."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures/xtext/inference-composition"
SPEC = importlib.util.spec_from_file_location("inference_composition_oracle", FIXTURE / "verify.py")
ORACLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORACLE)


class XtextInferenceCompositionEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.provenance, cls.cases, cls.rows = ORACLE.frozen(FIXTURE)
        cls.by_id = {row["id"]: row for row in cls.rows}
        cls.imported = json.loads((FIXTURE.parent / "metamodel-inference/observations.json").read_text(encoding="utf-8"))[0]["imported_package"]

    def generated(self, case, name):
        return next(classifier for declaration in self.by_id[case]["sources"][-1]["metamodels"]
                    if declaration["generated"] and declaration["package"]
                    for classifier in declaration["package"]["classifiers"] if classifier["name"] == name)

    def test_denominators_preserve_structural_and_origin_changes_separately(self):
        self.assertEqual((len(self.rows), sum(row["accepted"] for row in self.rows)), (31, 20))
        self.assertEqual(sum(len(row["sources"]) for row in self.rows), 66)
        parents = [parent for row in self.rows for parent in row["parent_snapshots"]]
        self.assertEqual(len(parents), 35)
        self.assertTrue(all(parent["structure_unchanged"] for parent in parents))
        self.assertEqual(sum(not parent["metadata_unchanged"] for parent in parents), 4)
        self.assertEqual(sum(not row["imported_unchanged"] for row in self.rows), 1)
        self.assertTrue(all(ORACLE.structural(row["imported_package"]) == ORACLE.structural(self.imported) for row in self.rows))
        changed = self.by_id["child_subclasses_original_import"]["imported_package"]
        self.assertEqual(changed["classifiers"][0]["features"][0]["origins"][0]["kind"], "Assignment")
        self.assertEqual(self.provenance["implementation_baseline"], "a05e271779a277dc0ad49767b2e9d17183355c4e")

    def test_exact_inherited_overrides_super_calls_and_import_aliases(self):
        name = "override_implicit_child_type"
        node = self.generated(name, "Node")
        self.assertEqual(node["super_types"], [f"urn:intersect:inference:composition:{name}:base#//Node"])
        self.assertEqual(node["features"], [])
        rules = self.by_id["same_short_classifier_names"]["sources"][-1]["rules"]
        calls = next(rule for rule in rules if rule["name"] == "Entry")["calls"]
        self.assertEqual([call["explicit"] for call in calls], [True, False])
        self.assertEqual([call["target"]["grammar"].rsplit(".", 1)[-1] for call in calls], ["Base", "Child"])
        self.assertNotEqual(calls[0]["target"]["type"], calls[1]["target"]["type"])
        fields = self.generated("same_name_imports_explicit_aliases", "Entry")["features"]
        self.assertEqual(fields[0]["type"], "urn:intersect:inference:composition:same_name_imports_explicit_aliases:base#//External")
        self.assertEqual(fields[1]["type"], "urn:intersect:inference:imported#//External")
        self.assertTrue(self.by_id["shared_import_alias"]["accepted"])
        child = self.by_id["import_only_child"]["sources"][-1]
        self.assertFalse(any(declaration["generated"] for declaration in child["metamodels"]))
        self.assertTrue(child["accepted"])

    def test_rejected_partial_outputs_do_not_award_composition_support(self):
        for name in ("two_parents", "ambiguous_parent_rule", "unaliased_import_collision"):
            row = self.by_id[name]
            self.assertFalse(row["accepted"])
            self.assertEqual(row["sources"][-1]["error_count"], 1)
            self.assertTrue(any("more than one" in item["message"] for item in row["sources"][-1]["diagnostics"]))
            self.assertTrue(self.generated(name, "Entry")["features"])
        self.assertFalse(self.by_id["sealed_parent_new_feature"]["accepted"])
        self.assertFalse(self.by_id["sealed_parent_new_supertype"]["accepted"])
        self.assertFalse(self.by_id["same_name_imports_unqualified"]["accepted"])
        self.assertFalse(self.by_id["override_terminal_type_change"]["accepted"])
        self.assertFalse(self.by_id["override_imported_enum_new_literal"]["accepted"])

    def test_verifier_rejects_corrupt_graph_outcomes_origins_and_mutation_claims(self):
        changes = [
            lambda rows: rows[0].update(accepted=False),
            lambda rows: rows[0]["sources"][-1]["rules"][0]["calls"][0]["target"].update(grammar="foreign.Grammar"),
            lambda rows: rows[0]["parent_snapshots"][0].update(metadata_unchanged=False),
            lambda rows: rows[0]["sources"][-1]["rules"][0].update(end=10**9),
            lambda rows: rows[0]["imported_package"]["classifiers"][0]["features"][0].update(unique=False),
            lambda rows: rows[0]["sources"][-1]["metamodels"][0]["package"]["classifiers"][0]["features"][0]["origins"][0].update(grammar="foreign.Grammar"),
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
