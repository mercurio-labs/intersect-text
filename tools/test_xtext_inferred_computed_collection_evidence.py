"""Frozen source-backed collection consumers and actual prior-release outcomes."""
import hashlib
import json
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/xtext/inferred-computed-collection-contexts"
FROZEN = {'plan.json': 'd07cf5cb0b2460b4adf1f9ad56bc597b726ae1070d4fb3856eeb39c1bf1b1edd', 'contexts.json': 'a3760101ff024f9edbb8f80dfd311c0dade2c350a943be31691d9c0ed3afe1ed', 'before.json': 'c0442898002871f256ca88981ee59611ed6f07ea2f98ad43f9b16b55ff0d76dc', 'shape-conflicts.json': 'b32532ea3f3d9f8afa8568ce3c59a621e2964aaad90697c1cc4dc6baac69fe33'}
class ComputedCollectionEvidence(unittest.TestCase):
    def test_exact_frozen_bytes(self):
        for name, digest in FROZEN.items():
            self.assertEqual(hashlib.sha256((FIXTURE / name).read_bytes()).hexdigest(), digest)

    def test_original_source_graphs_and_exact_contracts(self):
        plan = json.loads((FIXTURE / "plan.json").read_text(encoding="utf-8"))
        contexts = json.loads((FIXTURE / "contexts.json").read_text(encoding="utf-8"))
        self.assertEqual(len(plan["cases"]), 2)
        for row, selected in zip(plan["cases"], contexts):
            self.assertEqual(row["id"], selected["id"])
            context = selected["inference_context"]
            self.assertEqual(context["version"], 2)
            self.assertEqual(context["behavior"], row["behavior"])
            contract = context["computed_settings"][0]
            self.assertEqual(contract["operations"], ["set", "append", "unset"])
            self.assertEqual(contract["provider"], {"name":"rcore.projection", "version":"1"})
            sources = row["inference"]["bundle"]["sources"]
            self.assertEqual(contract["sources"], [source["id"] for source in sources])
            self.assertIn(row["behavior"], contract["sources"])
            self.assertIn("[0,3] nonunique", sources[0]["text"])
            for source in sources:
                self.assertEqual(source["id"]["sha256"], hashlib.sha256(source["text"].encode()).hexdigest())
                for dependency in source["dependencies"]:
                    self.assertIn(dependency, contract["sources"])

    def test_actual_old_append_failure_and_operation_bypass_are_retained(self):
        plan = json.loads((FIXTURE / "plan.json").read_text(encoding="utf-8"))
        before = json.loads((FIXTURE / "before.json").read_text(encoding="utf-8"))
        contexts = json.loads((FIXTURE / "contexts.json").read_text(encoding="utf-8"))
        self.assertEqual(before["baseline"], plan["baseline"])
        self.assertEqual(before["wasm_sha256"], "7de2953d6489f4a4094c839f4761ccc5f710b805f8bf08d2497b9e775b75cf92")
        for row, selected, old in zip(plan["cases"], contexts, before["cases"]):
            self.assertEqual(row["id"], old["id"])
            observations = old["observations"]
            self.assertEqual(len(observations), 17)
            self.assertEqual(observations[0]["request"], {"method":"compile_inferred_with_context", "inference":row["inference"], "inference_context":selected["inference_context"]})
            self.assertTrue(observations[0]["response"]["ok"])
            self.assertEqual(observations[1]["response"]["diagnostics"], [])
            if row["id"] == "plain":
                self.assertEqual(observations[2]["response"]["diagnostics"], [])
            else:
                self.assertIn("No emitted no-argument parser entry", observations[2]["response"]["diagnostics"][0]["message"])
            self.assertEqual(observations[4]["request"]["method"], "script/collection")
            self.assertEqual(observations[4]["response"]["error"], "Collection mutation requires a stored feature")
            self.assertIn("aborted", observations[5]["response"]["error"])
            self.assertEqual(observations[10]["request"]["inference_context"]["computed_settings"][0]["operations"], ["append"])
            self.assertEqual(observations[12]["request"]["method"], "script/set")
            self.assertIsNone(observations[12]["response"])
            self.assertEqual(observations[15]["request"]["method"], "script/unset")
            self.assertIsNone(observations[15]["response"])

    def test_unique_imported_shapes_remain_explicit_rejected_original_graphs(self):
        original = json.loads((FIXTURE / "shape-conflicts.json").read_text(encoding="utf-8"))
        before = json.loads((FIXTURE / "before.json").read_text(encoding="utf-8"))
        self.assertEqual(len(original["cases"]), 4)
        for row, old in zip(original["cases"], before["shape_conflicts"]):
            self.assertEqual(row["id"], old["id"])
            self.assertEqual(old["request"], {"method":"compile_inferred", "inference":row["inference"]})
            self.assertFalse(old["response"]["ok"])
            self.assertEqual(old["response"]["diagnostics"][0]["message"], "Grammar assignment conflicts with the imported feature")
            for source in row["inference"]["bundle"]["sources"]:
                self.assertEqual(source["id"]["sha256"], hashlib.sha256(source["text"].encode()).hexdigest())
if __name__ == "__main__":
    unittest.main()
