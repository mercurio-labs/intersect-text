"""Exact authored source graphs and real previous-release provider requests."""
import hashlib
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
FIXTURE=ROOT/"fixtures/xtext/inferred-computed-host-contexts"
FROZEN={'plan.json': 'f56a18bf5186d1bde3cde19f7ee767a8575e674d3de145326a8ff26cbc537425', 'contexts.json': 'e0291c346d9e8be0b4e6fe2a27812e588349c6cf2a5e3f960eb5a179ff972836', 'before.json': '72d3d7e5e58ac0de432054bab8f8f355d19b3dcf534f5f75dbaec8fb8b5f29d6'}
class ComputedHostEvidence(unittest.TestCase):
    def test_exact_frozen_bytes(self):
        for name,digest in FROZEN.items():
            self.assertEqual(hashlib.sha256((FIXTURE/name).read_bytes()).hexdigest(),digest)
    def test_original_source_graphs_contracts_and_actual_old_requests(self):
        plan=json.loads((FIXTURE/"plan.json").read_text(encoding="utf-8"))
        contexts=json.loads((FIXTURE/"contexts.json").read_text(encoding="utf-8"))
        before=json.loads((FIXTURE/"before.json").read_text(encoding="utf-8"))
        original=json.loads((ROOT/"fixtures/xtext/inferred-contexts/behavior-entry-explicit-cases.json").read_text(encoding="utf-8"))
        self.assertEqual(plan["cases"],original["cases"])
        self.assertEqual(before["baseline"],plan["baseline"])
        self.assertEqual(before["wasm_sha256"],"64ee84465a4b1a095a2893682003b3020151fc2b014e51a5ad749bbb1c2c400f")
        self.assertEqual(len(plan["cases"]),2)
        for row,selected,old in zip(plan["cases"],contexts,before["cases"]):
            context=selected["inference_context"]
            self.assertEqual(old["id"],row["id"])
            self.assertEqual(old["request"],{"method":"compile_inferred_with_context","inference":row["inference"],"inference_context":context})
            self.assertIn("unknown field `behavior`",old["response"]["error"])
            self.assertEqual(context["behavior"],row["behavior"])
            contract=context["computed_settings"][0]
            self.assertEqual(contract["provider"],{"name":"rcore.projection","version":"1"})
            self.assertEqual(contract["operations"],["set"])
            self.assertIn(row["behavior"],contract["sources"])
            sources=row["inference"]["bundle"]["sources"]
            self.assertEqual(contract["sources"],[s["id"] for s in sources])
            for source in sources:
                self.assertEqual(source["id"]["sha256"],hashlib.sha256(source["text"].encode()).hexdigest())
                for dep in source["dependencies"]:self.assertIn(dep,contract["sources"])
if __name__=="__main__":unittest.main()
