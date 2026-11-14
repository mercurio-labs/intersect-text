"""Portable evidence integrity and strict comparison, not a second language oracle."""
from pathlib import Path
import copy,importlib.util,json,os,unittest
CORPUS=Path(os.environ.get('INTERSECT_PROOF_GAP_CORPUS',Path(__file__).resolve().parents[1]/'fixtures/xtext/contextual-first-rule-proof-gaps'))
spec=importlib.util.spec_from_file_location('proof_gap_verify',CORPUS/'verify.py');verify=importlib.util.module_from_spec(spec);spec.loader.exec_module(verify)
class Evidence(unittest.TestCase):
 def test_complete_pinned_evidence(self):
  self.assertEqual(verify.verify(),{'graphs':5,'planned':25,'helper_admit':3,'helper_reject':2,'native_before_calls':17,'native_projects':5,'production_change':False})
 def test_unrelated_absolute_genmodel_is_not_normalized(self):
  rows=verify.load(verify.HERE/'oracle/record/observations.json');p=verify.load(verify.HERE/'provenance.json');rows[0]['provider']['genmodel']='C:/unrelated/provider.genmodel'
  with self.assertRaises(AssertionError):verify.canonical(rows,'record',p)
 def test_matching_basename_outside_owned_run_is_not_normalized(self):
  rows=verify.load(verify.HERE/'oracle/record/observations.json');p=verify.load(verify.HERE/'provenance.json');h=rows[0]['provider']['ecore_sha256'];rows[0]['provider']['genmodel']='C:/unrelated/provider-'+h[:12]+'/provider.genmodel'
  with self.assertRaises(AssertionError):verify.canonical(rows,'record',p)
 def test_runtime_field_changes_are_retained(self):
  rows=verify.load(verify.HERE/'oracle/record/observations.json');p=verify.load(verify.HERE/'provenance.json');forged=copy.deepcopy(rows);forged[0]['runtime'][0]['observation']['resource']['accepted']=not forged[0]['runtime'][0]['observation']['resource']['accepted']
  self.assertNotEqual(verify.canonical(rows,'record',p),verify.canonical(forged,'record',p))
if __name__=='__main__':unittest.main()
