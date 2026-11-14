"""Pinned parameter declarations, scoped execution, and generation boundaries."""
import copy
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent
FIXTURE = ROOT / "fixtures/xtext/inference-parameters"
SPEC = importlib.util.spec_from_file_location("parameter_evidence", FIXTURE / "verify.py")
ORACLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORACLE)


class ParameterEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.provenance, cls.cases, cls.rows = ORACLE.frozen(FIXTURE)

    def run_row(self, case, text="alpha", entry="resource", variant="lowercase_counterparts"):
        return next(r for r in self.rows[variant]["runs"] if r["case"] == case and r["text"] == text and r["entry"] == entry)

    def test_separate_inference_generation_and_runtime_denominators(self):
        self.assertEqual(self.provenance["counts"], {
            "graphs": 74, "sources": 81, "inference_accepted": 48,
            "inference_rejected": 26, "compiled_graphs": 26,
            "generation_failed": 22, "runtime_requests": 67,
            "runtime_accepted": 30, "runtime_rejected": 37,
            "planned_unexecuted_requests": 58,
        })

    def test_original_obstructions_keep_every_unexecuted_request(self):
        original = self.rows["original_obstructions"]
        self.assertEqual(original["runs"], [])
        self.assertEqual(sum(len(c["planned_runs"]) for c in self.cases["original_obstructions"]), 58)
        failures = [r for r in original["generation"] if r["stage"] == "generation_failed"]
        self.assertEqual(len(failures), 22)
        self.assertTrue(all(r["generation_exit"] != 0 and r["compilation_exit"] is None for r in failures))
        self.assertTrue(all(any(key.startswith("TypeReference.package_name:") for key in r["error_keys"]) for r in failures))

    def test_guarded_structure_retains_both_features_even_with_constant_false(self):
        for name in ("true_call", "false_call", "unreachable_guard_structure"):
            row = next(r for r in self.rows["lowercase_counterparts"]["metadata"] if r["id"] == name)
            classifiers = [c for d in row["sources"][-1]["metamodels"] if d["generated"] for c in d["package"]["classifiers"]]
            node = next(c for c in classifiers if c["name"] == "Node")
            self.assertEqual([(f["name"], f["lower"], f["upper"]) for f in node["features"]], [("a", 0, 1), ("b", 0, 1)])
        self.assertEqual(self.run_row("true_call")["model"][1]["values"], {"a": "alpha", "b": None})
        self.assertEqual(self.run_row("false_call")["model"][1]["values"], {"a": None, "b": "alpha"})

    def test_duplicate_declaration_identity_is_not_a_name_map(self):
        row = self.run_row("duplicate_used", variant="additional_controls")
        self.assertTrue(row["accepted"])
        self.assertEqual(row["model"][1]["values"], {"a": None, "b": "alpha"})
        metadata = next(r for r in self.rows["additional_controls"]["metadata"] if r["id"] == "duplicate_named_declarations")
        self.assertFalse(metadata["all_sources_accepted"])
        messages = [d["message"] for s in metadata["sources"] for d in s["diagnostics"]]
        self.assertIn("Duplicate value for parameter Flag", messages)

    def test_recursion_pushes_and_restores_boolean_environments(self):
        row = self.run_row("recursive_context", "yes alpha(no beta(yes gamma))")
        self.assertTrue(row["accepted"])
        nodes = row["model"][1:]
        self.assertEqual([(n["values"]["a"], n["values"]["b"]) for n in nodes], [("alpha", None), (None, "beta"), ("gamma", None)])
        self.assertFalse(self.run_row("recursive_context", "yes alpha(yes beta)")["accepted"])

    def test_resource_default_and_named_parameter_entry_are_distinct(self):
        self.assertEqual(self.run_row("parameterized_entry")["model"][0]["values"], {"a": None, "b": "alpha"})
        resource = self.run_row("named_entry", variant="additional_controls")
        named = self.run_row("named_entry", entry="Node", variant="additional_controls")
        self.assertTrue(resource["accepted"])
        self.assertFalse(named["accepted"])
        self.assertIsNone(named["model"])
        self.assertEqual(named["exception"][-1]["class"], "java.lang.NoSuchMethodException")
        self.assertIn("entryRuleNode()", named["exception"][-1]["message"])

    def test_boolean_precedence_and_argument_ordering_have_independent_results(self):
        for name in ("and_or_precedence", "reordered_named", "mixed_named_positional"):
            self.assertEqual(self.run_row(name)["model"][1]["values"], {"yes": "alpha", "no": None})
        self.assertEqual(self.run_row("grouped_boolean")["model"][1]["values"], {"yes": None, "no": "alpha"})
        row = next(r for r in self.rows["additional_controls"]["metadata"] if r["id"] == "mixed_out_of_order")
        messages = [d["message"] for s in row["sources"] for d in s["diagnostics"] if d["severity"] == "ERROR"]
        self.assertEqual(messages, ["Out of sequence named argument. Expected value for B", "Out of sequence named argument. Expected value for C"])

    def test_verifier_rejects_omitted_requests_and_changed_source_variants(self):
        cases = copy.deepcopy(self.cases)
        cases["lowercase_counterparts"][0]["planned_runs"].pop()
        with self.assertRaises(ValueError):
            ORACLE.validate(FIXTURE, cases, self.rows)
        cases = copy.deepcopy(self.cases)
        cases["lowercase_counterparts"][0]["sources"][0]["source"] += "// changed semantics"
        with self.assertRaises(ValueError):
            ORACLE.validate(FIXTURE, cases, self.rows)


if __name__ == "__main__":
    unittest.main()
