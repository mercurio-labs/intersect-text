"""Integrity/stage/return checks for independent entry observations; no native claim."""
import copy,importlib.util,json,shutil,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
ROOT=Path(__file__).resolve().parents[1]
FIXTURE=ROOT/"fixtures/xtext/parameter-entry-contexts"
spec=importlib.util.spec_from_file_location("parameter_entry_evidence",FIXTURE/"verify.py")
proof=importlib.util.module_from_spec(spec);spec.loader.exec_module(proof)
class EntryEvidenceTests(unittest.TestCase):
 def setUp(self):
  self.cases=json.loads((FIXTURE/"cases.json").read_text(encoding="utf-8"))
  self.observed=json.loads((FIXTURE/"observations.json").read_text(encoding="utf-8"))
 def rejected(self):
  with self.assertRaises((AssertionError,ValueError,KeyError)):
   proof.validate(self.cases,self.observed)
 def test_frozen_complete_independent_stages(self):
  _,_,_,report=proof.frozen(ROOT)
  self.assertEqual(report["compiled_parsers"],7)
  self.assertEqual(report["direct_entries"],25)
  self.assertFalse(report["native_support"])
 def test_exact_grammar_bytes_are_required(self):
  self.cases["cases"][0]["sources"][0]["source"]+=" "
  self.rejected()
 def test_producer_identity_is_separate_from_content_hash(self):
  self.cases["cases"][0]["sources"][0]["uri"]="memory:/different/entry.xtext"
  self.rejected()
 def test_rootless_parse_is_not_a_syntax_rejection(self):
  row=next(r for r in self.observed["runs"] if r["case"]=="scalar_false" and r["entry"]=="Word" and r["text"]=="no")
  row["errors"]=[{"message":"invented error"}]
  self.rejected()
 def test_antlr_zero_counter_does_not_erase_real_node_errors(self):
  row=next(r for r in self.observed["direct_entries"] if r.get("errors") and r.get("antlr_syntax_error_counter")==0)
  row["errors"]=[]
  self.rejected()
 def test_successful_scalar_return_cannot_disappear(self):
  row=next(r for r in self.observed["direct_entries"] if r.get("value_kind")=="string" and not r.get("errors"))
  row.update(value_kind="null",value=None)
  self.rejected()
 def test_inventory_must_not_admit_false_seeds_under_dead_guards(self):
  forged={"method":"entryRuleWord","returns":"java.lang.String"}
  row=next(r for r in self.observed["generation"] if r["id"]=="scalar_false_dead")
  row["entry_methods"].append(forged)
  for raw in self.observed["direct_entries"]:
   if raw["case"]=="scalar_false_dead":raw["entry_methods"].append(copy.deepcopy(forged))
  self.rejected()
 def test_generator_obstruction_is_not_metadata_rejection(self):
  row=next(r for r in self.observed["generation"] if r["id"]=="parameterized_scalar_entry")
  row["stage"]="metadata_rejected"
  self.rejected()
 def test_unchanged_flags_do_not_override_changed_imported_structure(self):
  self.observed["metadata"][0]["imported_after"][0]["name"]="changed"
  self.rejected()
 def test_component_pin_rejects_changed_scalar_value(self):
  with tempfile.TemporaryDirectory() as directory:
   destination=Path(directory)
   for source in FIXTURE.iterdir():
    if source.is_file():shutil.copyfile(source,destination/source.name)
   changed=copy.deepcopy(self.observed)
   row=next(r for r in changed["direct_entries"] if r.get("value_kind")=="string" and not r.get("errors"))
   row["value"]="changed"
   (destination/"observations.json").write_text(json.dumps(changed),encoding="utf-8")
   with patch.object(proof,"HERE",destination),self.assertRaises(ValueError):proof.frozen(ROOT)
if __name__=="__main__":unittest.main()
