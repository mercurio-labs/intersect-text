"""Frozen authored range controls, not an independent JVM qualification award."""
import hashlib
import json
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[1] / "fixtures/xtext/parameter-origins"
class ParameterOriginEvidence(unittest.TestCase):
    def test_frozen_preimplementation_plan_and_observations(self):
        for name, expected in [("expectations.json", "4226b3a083ba24c61b9cc65fa20ded752fd4c53f961899dc81ce8d4741942769"), ("cases.json", "3666921608c7d1932ea20959201a1befb049166c82668c0619aa7b1f183ef892"), ("before.json", "aa5af23b892e2f328e5ee26cef4f413d256e66c926dc1999b6a18778e22020a6")]:
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), expected)
    def test_exact_source_revisions_and_utf8_ranges_match_the_old_real_requests(self):
        plan = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))
        before = json.loads((ROOT / "before.json").read_text(encoding="utf-8"))
        self.assertEqual(len(plan["cases"]), 4)
        self.assertEqual(len(before), 4)
        for case, row in zip(plan["cases"], before):
            self.assertEqual(case["id"], row["id"])
            ids = {}
            for authored, actual in zip(case["sources"], row["inference"]["bundle"]["sources"]):
                identity = {"uri": authored["uri"], "sha256": hashlib.sha256(authored["source"].encode()).hexdigest()}
                self.assertEqual(identity["sha256"], authored["sha256"])
                self.assertEqual(actual, {"id": identity, "kind": "xtext", "text": authored["source"], "dependencies": [ids[name] for name in authored["dependencies"]]})
                ids[authored["id"]] = identity
            self.assertEqual(row["inference"]["grammar"], ids[case["entry"]])
            source = next(s for s in case["sources"] if s["id"] == case["expected_source"])
            start, end = case["expected_original"]["start"], case["expected_original"]["end"]
            self.assertEqual(source["source"].encode()[start:end].decode(), case["expected_original_text"])
            self.assertFalse(row["result"]["ok"])
            self.assertEqual(row["result"]["diagnostics"][0]["message"], case["message"])
            old = row["result"]["diagnostic_origins"][0]["originals"][0]
            self.assertEqual(old["source"], ids[case["expected_source"]])
            self.assertLessEqual(old["start"], start)
            self.assertGreaterEqual(old["end"], end)
            self.assertGreater(old["end"] - old["start"], end - start)
    def test_endpoint_and_predicate_scope_corrections_retain_initial_proposals(self):
        cases = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))["cases"]
        expected = json.loads((ROOT / "expectations.json").read_text(encoding="utf-8"))["scopes"]
        for case in cases:
            source = next(s for s in case["sources"] if s["id"] == case["expected_source"])
            scope = expected[case["id"]]
            self.assertEqual(source["source"].encode()[scope["start"]:scope["end"]].decode(), scope["text"])
            self.assertEqual(scope["text"], "=> b=ID" if case["id"] == "predicate" else "'a'")
            self.assertNotEqual({"start": scope["start"], "end": scope["end"]}, case["expected_original"])
    def test_unicode_crlf_and_same_uri_revisions_are_real_source_controls(self):
        cases = json.loads((ROOT / "cases.json").read_text(encoding="utf-8"))["cases"]
        text = cases[1]["sources"][0]["source"]
        self.assertIn("café 😀", text)
        self.assertIn("\r\n", text)
        self.assertNotIn("\n", text.replace("\r\n", ""))
        first, selected = cases[3]["sources"][:2]
        self.assertEqual(first["uri"], selected["uri"])
        self.assertNotEqual(first["sha256"], selected["sha256"])
        self.assertEqual(cases[3]["sources"][2]["dependencies"], [selected["id"]])
if __name__ == "__main__":
    unittest.main()
