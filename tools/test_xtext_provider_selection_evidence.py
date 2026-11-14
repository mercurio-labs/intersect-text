"""Frozen authored provider-selector graphs and actual old-release responses."""
import hashlib
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
FIXTURE=ROOT/"fixtures/xtext/inferred-provider-selection"
FROZEN={'plan.json': '3abe0f4cb034c4958a76c5fced3a58127206d53865b885424e360f9cd3c67e08', 'before.json': '2198ddc38102886b162bf8bd18002dd6c223003762783147e5d68629a37d7067', 'catalog-before.json': '8d7f7176413d932ab847211c2c225047b0744af9592725bf6a5845f21b1c8cef'}
class ProviderSelectionEvidence(unittest.TestCase):
    def test_frozen_bytes(self):
        for name,digest in FROZEN.items():
            self.assertEqual(hashlib.sha256((FIXTURE/name).read_bytes()).hexdigest(),digest)
    def test_original_source_graphs_and_independent_expected_contexts(self):
        plan=json.loads((FIXTURE/"plan.json").read_text(encoding="utf-8"))
        originals=[]
        for family,name in [("scalar","inferred-computed-host-contexts"),("collection","inferred-computed-collection-contexts")]:
            parent=ROOT/"fixtures/xtext"/name
            rows=json.loads((parent/"plan.json").read_text(encoding="utf-8"))["cases"]
            contexts=json.loads((parent/"contexts.json").read_text(encoding="utf-8"))
            originals.extend((family+"_"+row["id"],row,next(c["inference_context"] for c in contexts if c["id"]==row["id"])) for row in rows)
        self.assertEqual(len(plan["cases"]),4)
        for row,(name,original,expected) in zip(plan["cases"],originals):
            self.assertEqual(row["id"],name)
            self.assertEqual(row["inference"],original["inference"])
            self.assertEqual(row["expected_context"],expected)
            selection=row["provider_selection"]
            self.assertEqual(set(selection),{"version","behavior","computed_settings"})
            self.assertEqual(selection["version"],1)
            self.assertEqual(selection["behavior"],original["behavior"])
            for selector in selection["computed_settings"]:
                self.assertEqual(set(selector),{"source","namespace","class","feature","provider","operations"})
                self.assertEqual(selector["provider"],{"name":"rcore.projection","version":"1"})
                self.assertEqual(selector["operations"],expected["computed_settings"][0]["operations"])
                self.assertIn(selector["source"],expected["computed_settings"][0]["sources"])
        for row in plan["cases"]+plan["negative_cases"]:
            self.assertEqual(set(row["inference"]),{"version","bundle","grammar"})
            for source in row["inference"]["bundle"]["sources"]:
                self.assertEqual(source["id"]["sha256"],hashlib.sha256(source["text"].encode()).hexdigest())
    def test_actual_prior_helper_preview_and_original_execution_requests(self):
        plan=json.loads((FIXTURE/"plan.json").read_text(encoding="utf-8"))
        before=json.loads((FIXTURE/"before.json").read_text(encoding="utf-8"))
        self.assertEqual(before["baseline"],plan["baseline"])
        self.assertEqual(before["wasm_sha256"],"3d4594f531f7825eb7871319a130a2c2f8632291cddbde0030044bc1dccb8cc0")
        for row,old in zip(plan["cases"],before["cases"]):
            self.assertEqual(row["id"],old["id"])
            observations=old["observations"]
            self.assertEqual(len(observations),6)
            self.assertEqual(observations[1]["request"],{"method":"inferred/selectProviders","inference":row["inference"],"provider_selection":row["provider_selection"]})
            for observation in observations[:3]:self.assertEqual(observation["response"],{"error":"Unknown method"})
            self.assertEqual(observations[3]["response"],{"error":"Inferred compilation requires the explicit context method"})
            self.assertFalse(observations[4]["response"]["ok"])
            self.assertEqual(observations[5]["response"]["diagnostics"],[])
            self.assertEqual(len(observations[5]["response"]["model"]["elements"]),2)
        self.assertEqual(len(before["negative_cases"]),4)
        for row,old in zip(plan["negative_cases"],before["negative_cases"]):
            self.assertEqual(old["request"],{"method":"inferred/selectProviders","inference":row["inference"],"provider_selection":row["provider_selection"]})
            self.assertEqual(old["response"],{"error":"Unknown method"})
    def test_original_catalog_only_graphs_and_preview_boundaries(self):
        before=json.loads((FIXTURE/"catalog-before.json").read_text(encoding="utf-8"))
        parent=ROOT/"fixtures/xtext/inferred-host-contexts"
        plan=json.loads((parent/"plan.json").read_text(encoding="utf-8"))
        contexts=json.loads((parent/"contexts.json").read_text(encoding="utf-8"))
        for row,old in zip(plan["cases"],before["cases"]):
            self.assertEqual(row["inference"],old["request"]["inference"])
            context=next(c["inference_context"] for c in contexts if c["id"]==row["id"])
            self.assertEqual(old["request"]["inference_context"],context)
            self.assertEqual(context["version"],1)
            self.assertEqual(old["response"],{"error":"Unknown method"})
            self.assertEqual(old["legacy_response"],{"error":"Inferred compilation requires the explicit context method"})
if __name__=="__main__":unittest.main()
