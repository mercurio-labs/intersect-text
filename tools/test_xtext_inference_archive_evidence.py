"""Read-only integrity checks for the frozen pre-change ordinary archive baseline."""
import hashlib
import json
from pathlib import Path
import unittest
import tempfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "fixtures/xtext/inference-archive"

def verify(folder):
    provenance = json.loads((folder / "provenance.json").read_text(encoding="utf-8"))
    if provenance["source_revision"] != "a7ea7155575214029d47905157ea55c43cb4c80a" or provenance["expectation_regeneration"]:
        raise ValueError("Frozen source revision/classification changed")
    for name, expected in provenance["artifacts"].items():
        if hashlib.sha256((folder / name).read_bytes()).hexdigest() != expected:
            raise ValueError("Frozen artifact hash changed: " + name)
    baseline = json.loads((folder / "ordinary-baseline.json").read_text(encoding="utf-8"))
    if baseline["manifest"]["files"] != baseline["files"]:
        raise ValueError("Complete original manifest/payload inventory differs")
    if set(baseline["zip"]) != {"plain", "python"}:
        raise ValueError("Missing ZIP observation")
    return provenance, baseline

def verify_recovered_input(folder):
    provenance = json.loads((folder / "ordinary-input-provenance.json").read_text(encoding="utf-8"))
    _, baseline = verify(FIXTURE)
    if provenance["source_revision"] != "a7ea7155575214029d47905157ea55c43cb4c80a" or provenance["expectation_regeneration"]:
        raise ValueError("Recovered input producer contract changed")
    record = provenance["generated_input"]
    data = (folder / record["path"]).read_bytes()
    if len(data) != record["bytes"] or hashlib.sha256(data).hexdigest() != record["sha256"]:
        raise ValueError("Recovered generated input changed")
    if record["archive_source_id"] != {"uri": "memory:model.rcore", "sha256": record["sha256"]}:
        raise ValueError("Recovered archive source identity changed")
    if provenance["payload_observations_matched"] != baseline["files"] or provenance["manifest_observation_matched"] is not True:
        raise ValueError("Historical producer did not reproduce frozen observations")
    if provenance["original_observation_sha256"] != hashlib.sha256((FIXTURE / "ordinary-baseline.json").read_bytes()).hexdigest():
        raise ValueError("Historical observation changed")
    return provenance

class ArchiveEvidence(unittest.TestCase):
    def test_frozen_artifacts_and_prechange_commit(self):
        provenance, baseline = verify(FIXTURE)
        self.assertIn("compatibility observation", provenance["classification"])
        self.assertIn("no new semantic support", baseline["classification"])
    def test_complete_manifest_and_original_compiler_contract(self):
        _, baseline = verify(FIXTURE)
        manifest = baseline["manifest"]
        self.assertEqual(manifest["compiler"]["name"], "intersect-model")
        self.assertEqual(manifest["definitions"], {"bundle": "definitions/bundle.json"})
        self.assertEqual(manifest["format_version"], 2)
        self.assertEqual(manifest["runtime"], {"package_format": 2, "profile": "intersect.workbench.v1", "rcore_behavior_format": 1})
        self.assertEqual(set(baseline["files"]), {"package.json", "definitions/bundle.json"})
    def test_both_real_zip_observations_are_distinct_and_complete(self):
        _, baseline = verify(FIXTURE)
        self.assertEqual(baseline["zip"]["plain"]["bytes"], 5080)
        self.assertEqual(baseline["zip"]["python"]["bytes"], 9958)
        for observation in baseline["zip"].values():
            self.assertEqual(len(observation["sha256"]), 64)
        self.assertNotEqual(baseline["zip"]["plain"]["sha256"], baseline["zip"]["python"]["sha256"])
    def test_each_payload_and_zip_producer_is_pinned(self):
        provenance, _ = verify(FIXTURE)
        self.assertTrue({"toolkit/src/package-export.js", "toolkit/src/package-import.js", "toolkit/src/python-api.js", "toolkit/package-lock.json", "Cargo.lock", "tests/acceptance/examples/compatibility_corpus.rs"} <= set(provenance["source_hashes_before_changes"]))
        self.assertTrue(all(len(h) == 64 for h in provenance["source_hashes_before_changes"].values()))

    def test_recovered_historical_input_has_exact_source_and_producer_identity(self):
        p = verify_recovered_input(FIXTURE)
        self.assertEqual(p["generated_input"]["bytes"], 1044)
        self.assertEqual(p["generated_input"]["sha256"], "1829936c163c964bee941a362d72007914b0fae010525074f266efd3b2412194")
        self.assertIn("crates/intersect-model/src/definitions/ecore_migration.rs", p["producer_sources"])
        self.assertIn("tests/acceptance/src/lib.rs", p["producer_sources"])
        self.assertTrue(all(len(value) == 64 for value in p["producer_sources"].values()))

    def test_recovered_input_tampering_is_rejected_without_changing_frozen_expectations(self):
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name)
            for source in ["ordinary-input-provenance.json", "ordinary-baseline-input.rcore"]:
                (folder / source).write_bytes((FIXTURE / source).read_bytes())
            verify_recovered_input(folder)
            with (folder / "ordinary-baseline-input.rcore").open("ab") as changed:
                changed.write(b" ")
            with self.assertRaisesRegex(ValueError, "Recovered generated input changed"):
                verify_recovered_input(folder)

    def test_source_encoding_independent_artifacts_and_full_outcomes(self):
        p = json.loads((FIXTURE / "source-encoding-provenance.json").read_text(encoding="utf-8"))
        for name, expected in p["artifacts"].items():
            self.assertEqual(hashlib.sha256((FIXTURE / name).read_bytes()).hexdigest(), expected)
        cases = json.loads((FIXTURE / "source-encoding-cases.json").read_text(encoding="utf-8"))
        rows = json.loads((FIXTURE / "source-encoding-observations.json").read_text(encoding="utf-8"))
        self.assertEqual([r["id"] for r in rows], [c["id"] for c in cases])
        self.assertEqual([r["all_sources_accepted"] for r in rows], [True, False, True])
        self.assertEqual(cases[1]["sources"][0]["source"], "\ufeff" + cases[0]["sources"][0]["source"])
        self.assertIn("\u03b1", cases[2]["sources"][0]["source"])
    def test_oracle_diagnostics_preserve_bom_and_utf8_stdio_contract(self):
        rows = json.loads((FIXTURE / "source-encoding-observations.json").read_text(encoding="utf-8"))
        diagnostic = rows[1]["sources"][0]["diagnostics"][0]
        self.assertIn("\ufeff", diagnostic["message"])
        self.assertEqual((diagnostic["start"], diagnostic["length"]), (0, 1))
        p = json.loads((FIXTURE / "source-encoding-provenance.json").read_text(encoding="utf-8"))
        self.assertIn("-Dstdout.encoding=UTF-8", p["java_options"])
        self.assertEqual(hashlib.sha256((ROOT / p["oracle_provenance"]).read_bytes()).hexdigest(), p["oracle_provenance_sha256"])

if __name__ == "__main__":
    unittest.main()
