"""Integrity of frozen native provider-extension graphs and old-release failures.
These controls do not award independent JVM or browser provider qualification.
"""
import hashlib
import json
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1] / "fixtures/xtext/inferred-contexts"
FROZEN = {'before.json': '701f603e0b42b6ca8da4ff13f8c629efc71822de5ee55d29c21bd7f8a86bd13f', 'behavior-before.json': '161ea628876a0b2d26803a29efd1dc20fa3ec8b338fd9e1e8bd19cf0aa011294', 'behavior-cases.json': '08963c251a1267388a87af6e822b9725691bccf430f74432578f3b8bcf0c1e5b', 'behavior-entry-before.json': 'fc90aa51f9ced30e46990c619d448034af85e434727a160d0d7abd27ac297970', 'behavior-entry-cases.json': 'e405f39a0c49c1b336d1136576c4bdb0279e34738f984b139969e494c0728047', 'behavior-entry-explicit-before.json': '70d5fc37fcf2866d6f80918b4a6f5c462f805be8475e5d3c4d2f2511f88e4e1b', 'behavior-entry-explicit-cases.json': '646486faa39359e70c86fe09b5dad2fe6dbc88169a488b9f5d48c3ba88cf9581', 'cases.json': '5626f83c09fd93596b50d5bb110f6bdf9d8104577e501f8c0c697247f28c2ddb', 'construction-before.json': '40e3d3f5625b1372c8e373159bde2a24e264352fff376a0a63496cdd25cad3a6', 'construction-cases.json': '1214ad115bd15ba2623103da93206251615716e1958f81a141918396e5a7a53c', 'construction-group-before.json': '645c550bbd5061fc0739920b920321f52eac4ba5e81ceb55bc267511e53fdd14', 'construction-group-cases.json': '46380adff1919fb4a1752ca114ab276621898e57e11a61630cfd14e3804c72b5', 'conversion-before.json': '9c222a1b09e00fe97cd69f9cc06b14098937ac90ecba8e7a253fe1f9e9c9f8e8', 'conversion-cases.json': 'defac3512ba458314f9572694637b1a733cd81382ed21aa5f9b71319b28ac4e6'}
class InferredContextEvidence(unittest.TestCase):
    def test_frozen_plans_and_old_release_captures(self):
        for name, digest in FROZEN.items():
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), digest, name)
    def test_exact_source_revisions_and_dependency_selections(self):
        for path in [ROOT / "cases.json", *ROOT.glob("*-cases.json")]:
            for row in json.loads(path.read_text(encoding="utf-8"))["cases"]:
                request = row["inference"]
                sources = request["bundle"]["sources"]
                ids = [s["id"] for s in sources]
                self.assertIn(request["grammar"], ids)
                for source in sources:
                    self.assertEqual(source["id"]["sha256"], hashlib.sha256(source["text"].encode()).hexdigest())
                    for dependency in source["dependencies"]:
                        self.assertIn(dependency, ids)
                for binding in request["bundle"]["namespaces"]:
                    self.assertIn(binding["source"], ids)
    def test_original_failures_remain_separate_from_positive_counterparts(self):
        for name, count in [("", 4), ("behavior-", 2), ("behavior-entry-", 2), ("behavior-entry-explicit-", 2), ("conversion-", 2), ("construction-", 2), ("construction-group-", 2)]:
            plan = json.loads((ROOT / (name + "cases.json")).read_text(encoding="utf-8"))["cases"]
            before = json.loads((ROOT / (name + "before.json")).read_text(encoding="utf-8"))
            self.assertEqual(len(plan), count)
            self.assertEqual(len(before), count)
            for case, captured in zip(plan, before):
                self.assertEqual(case["inference"], captured["inference"])
                self.assertEqual(case["id"], captured["id"])
                self.assertFalse(captured["before"]["ok"])
        initial = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))
        self.assertEqual(len(initial["cases"]), 4)
if __name__ == "__main__":
    unittest.main()
