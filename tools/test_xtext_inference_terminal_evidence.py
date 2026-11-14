"""Integrity controls for whole-graph terminal-library inference evidence."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parent.parent
FIXTURE=ROOT/"fixtures/xtext/inference-terminal-parents"
SPEC=importlib.util.spec_from_file_location("terminal_inference_oracle",FIXTURE/"verify.py")
ORACLE=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(ORACLE)


class TerminalInferenceEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.provenance,cls.cases,cls.rows=ORACLE.frozen(FIXTURE)
        cls.by_id={r["id"]:r for r in cls.rows}
        cls.imported=json.loads((FIXTURE.parent/"metamodel-inference/observations.json").read_text(encoding="utf-8"))[0]["imported_package"]

    def test_complete_graph_and_entry_only_outcomes_are_not_interchangeable(self):
        self.assertEqual((len(self.rows),sum(r["all_sources_accepted"] for r in self.rows),sum(r["accepted"] for r in self.rows)),(21,16,19))
        self.assertEqual(sum(len(r["sources"]) for r in self.rows),46)
        self.assertEqual(sum(len(r["parent_snapshots"]) for r in self.rows),25)
        for name in ("invalid_parent_return","malformed_parent_body","undeclared_parent_alias"):
            row=self.by_id[name]
            self.assertTrue(row["accepted"]);self.assertFalse(row["all_sources_accepted"])
            self.assertGreater(row["sources"][0]["error_count"],0)

    def test_terminal_ancestors_keep_no_generated_package_and_exact_rule_origins(self):
        row=self.by_id["common_scalars"]
        self.assertTrue(row["all_sources_accepted"])
        self.assertFalse(any(d["generated"] for d in row["sources"][0]["metamodels"]))
        calls=row["sources"][-1]["rules"][0]["calls"]
        self.assertEqual([c["target"]["name"] for c in calls],["ID","INT","STRING"])
        self.assertTrue(all(c["target"]["grammar"]=="org.eclipse.xtext.common.Terminals" for c in calls))
        fragment=self.by_id["terminal_fragments"]["sources"][0]["rules"][0]
        self.assertEqual(fragment["name"],"LETTER");self.assertEqual(fragment["type"],"http://www.eclipse.org/emf/2002/Ecore#//EString")
        self.assertTrue(self.by_id["three_level_scalar"]["all_sources_accepted"])
        self.assertTrue(self.by_id["three_level_generated"]["all_sources_accepted"])

    def test_pinned_source_bytes_and_immutable_parent_metadata_are_retained(self):
        common=(FIXTURE.parent/"terminal-parents/Terminals.xtext").read_bytes()
        self.assertTrue(common.startswith(b"/*****"))
        for case,row in zip(self.cases,self.rows):
            if case["sources"][0]["grammar"]=="org.eclipse.xtext.common.Terminals":
                self.assertEqual(case["sources"][0]["source"].encode(),common)
            self.assertTrue(all(s["metadata_unchanged"] and s["structure_unchanged"] for s in row["parent_snapshots"]))
            self.assertEqual(row["imported_package"],self.imported)

    def test_verifier_rejects_corrupt_outcomes_scopes_spans_and_source_snapshots(self):
        changes=[
            lambda rows: rows[0].update(all_sources_accepted=True),
            lambda rows: rows[1]["sources"][-1]["rules"][0]["calls"][0]["target"].update(grammar="foreign.Grammar"),
            lambda rows: rows[1]["sources"][0]["rules"][0].update(end=10**9),
            lambda rows: rows[1]["parent_snapshots"][0].update(metadata_unchanged=False),
        ]
        for index,change in enumerate(changes):
            with self.subTest(change=index):
                rows=copy.deepcopy(self.rows);change(rows)
                with self.assertRaises(ValueError):ORACLE.validate_rows(self.cases,rows,self.imported)
        with self.assertRaisesRegex(ValueError,"order/identity"):
            ORACLE.validate_rows(self.cases,list(reversed(self.rows)),self.imported)


if __name__=="__main__":unittest.main()
