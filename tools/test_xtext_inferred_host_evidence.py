"""Frozen native catalog consumer plans and real old-release boundary failures."""
import hashlib
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]/"fixtures/xtext/inferred-host-contexts"
FROZEN={'before.json': '6a4da03f880d5fb585bc340358ff28884cd04d62c974b3889b26abd57eaa9712', 'contexts.json': '0757141205a5ad5f49baa316231ad19feefa465542657f00f136b4d045d1b90f', 'plan.json': '25f822ebd2db49c4f4fd71d8985a5b46308f14de3333e5729c4de88d1d8ecf8f', 'selected-before.json': 'c4f765a2a48c38e693835b47a5e3d7ab1746e843469a1e1c84c36aef0a26bfde'}
class InferredHostEvidence(unittest.TestCase):
    def test_exact_frozen_bytes(self):
        for name,digest in FROZEN.items(): self.assertEqual(hashlib.sha256((ROOT/name).read_bytes()).hexdigest(),digest)
    def test_original_source_revisions_and_actual_preimplementation_requests(self):
        plan=json.loads((ROOT/"plan.json").read_text(encoding="utf-8"))
        before=json.loads((ROOT/"selected-before.json").read_text(encoding="utf-8"))
        contexts=json.loads((ROOT/"contexts.json").read_text(encoding="utf-8"))
        self.assertEqual(len(plan["cases"]),2)
        for case,previous,context in zip(plan["cases"],before,contexts):
            self.assertEqual(case["id"],previous["id"])
            self.assertEqual(case["inference"],previous["inference"])
            self.assertEqual(previous["inference_context"],context["inference_context"])
            self.assertIn("does not accept conversions",previous["legacy"]["error"])
            self.assertEqual(previous["explicit"]["error"],"Unknown method")
            sources=case["inference"]["bundle"]["sources"]
            for source in sources:
                self.assertEqual(source["id"]["sha256"],hashlib.sha256(source["text"].encode()).hexdigest())
                for dep in source["dependencies"]: self.assertIn(dep,[s["id"] for s in sources])
            self.assertEqual(context["inference_context"]["conversions"][0]["datatype"]["source"],sources[0]["id"])
if __name__=="__main__": unittest.main()
