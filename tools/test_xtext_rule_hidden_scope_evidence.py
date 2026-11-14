"""Pure pinned source/oracle/native storage integrity; no Java or native build."""
from pathlib import Path
import importlib.util,unittest

class HiddenScopeEvidence(unittest.TestCase):
 def test_original_stages_complete_rows_and_storage_negatives(self):
  root=Path(__file__).resolve().parents[1]/'fixtures/xtext/rule-hidden-scope'
  spec=importlib.util.spec_from_file_location('rule_hidden_scope_verify',root/'verify.py')
  module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
  result=module.verify(True)
  self.assertEqual(result['storage_negative_controls'],4)
  self.assertEqual(result['counts']['named_parser_executed'],155)
  self.assertEqual(result['counts']['named_entry_availability_failed'],23)
  self.assertEqual(result['final_attempt4_evidence']['complete_native_record_rows'],2147)

if __name__=='__main__':unittest.main()
