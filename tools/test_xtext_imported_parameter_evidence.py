"""Independent imported-language evidence integrity; no native support award."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures/xtext/imported-parameters"
spec = importlib.util.spec_from_file_location("imported_parameter_evidence", FIXTURE / "verify.py")
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class ImportedParameterEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.provenance, cls.cases, cls.rows = verify.frozen(FIXTURE)

    def test_complete_independent_stages_remain_separate_from_native_support(self):
        self.assertFalse(self.provenance["native_support"])
        self.assertEqual(verify.counts(self.cases, self.rows), {
            "graphs": 48, "grammar_sources": 53, "metamodel_sources": 53,
            "metadata_accepted": 31, "metadata_rejected": 17,
            "compiled_graphs": 31, "registered_requests": 91,
            "rooted_successes": 40, "error_free_rootless": 1,
            "registered_rejections": 50, "unregistered_context_requests": 31,
        })

    def test_changed_source_cannot_retain_original_producer_identity(self):
        cases = copy.deepcopy(self.cases)
        cases[0]["sources"][0]["source"] += " "
        with self.assertRaisesRegex(ValueError, "source digest"):
            verify.validate(cases, self.rows)

    def test_dropped_request_cannot_be_relabelled_as_complete_evidence(self):
        rows = copy.deepcopy(self.rows)
        rows["runs"].pop()
        with self.assertRaisesRegex(ValueError, "request inventory"):
            verify.validate(self.cases, rows)

    def test_parameter_references_keep_declaration_object_positions(self):
        rows = copy.deepcopy(self.rows)
        meta = next(m for m in rows["metadata"] if m["id"] == "true_call")
        rule = meta["sources"][0]["parameter_bindings"][0]
        rule["calls"][0]["arguments"][0]["parameter"]["position"] = 1
        with self.assertRaisesRegex(ValueError, "declaration position"):
            verify.validate(self.cases, rows)

    def test_utf16_ranges_cannot_split_surrogates_or_be_treated_as_bytes(self):
        rows = copy.deepcopy(self.rows)
        case = next(c for c in self.cases if c["id"] == "unicode_arguments")
        boundaries = verify.utf16_bytes(case["sources"][0]["source"])
        invalid = next(i for i in range(max(boundaries)) if i not in boundaries)
        meta = next(m for m in rows["metadata"] if m["id"] == case["id"])
        meta["sources"][0]["parameter_bindings"][0]["span"]["start"] = invalid
        with self.assertRaisesRegex(ValueError, "UTF-16 range"):
            verify.validate(self.cases, rows)
        self.assertNotEqual(max(boundaries), max(boundaries.values()))

    def test_models_retain_exact_imported_classifier_identities(self):
        rows = copy.deepcopy(self.rows)
        model = next(r["model"] for r in rows["runs"] if r["accepted"])
        model[0]["class"] = "urn:wrong-model#//Model"
        with self.assertRaisesRegex(ValueError, "classifier identity"):
            verify.validate(self.cases, rows)

    def test_partial_error_model_is_not_a_rooted_success(self):
        rows = copy.deepcopy(self.rows)
        row = next(r for r in rows["runs"] if r["model"] is not None and r.get("errors"))
        row["accepted"] = True
        with self.assertRaisesRegex(ValueError, "acceptance/root/error"):
            verify.validate(self.cases, rows)

    def test_unregistered_context_is_not_guessed_into_an_unknown_policy(self):
        rows = copy.deepcopy(self.rows)
        rows["unregistered_context"][0]["context"] = "guessed"
        with self.assertRaisesRegex(ValueError, "provider context"):
            verify.validate(self.cases, rows)

    def test_named_entries_follow_observed_reachable_contexts(self):
        generated = {g["id"]:g for g in self.rows["generation"]}
        cases = {c["id"]:c for c in self.cases}
        def entries(case_id):
            case = cases[case_id]
            grammar = next(s["grammar"] for s in case["sources"] if s["id"] == case["entry"])
            return generated[case_id]["parser_entries"][grammar]
        self.assertNotIn("Node", entries("true_call"))
        self.assertIn("Node", entries("false_call"))
        self.assertIn("Node", entries("inherited_parameters"))
        self.assertNotIn("Root", entries("inherited_parameters"))
        self.assertIn("Model", entries("parameterized_entry"))
        self.assertNotIn("Fields", entries("typed_fragment"))


if __name__ == "__main__":
    unittest.main()
