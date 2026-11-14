"""Independent portable integrity and stage controls for the upstream oracle."""
import hashlib
import json
from pathlib import Path
import unittest
from tools.xtext_terminal_negation_oracle import compare_observations
ROOT=Path(__file__).resolve().parent.parent
FIXTURE=ROOT/'fixtures/xtext/terminal-negation'
def load(name):return json.loads((FIXTURE/name).read_text(encoding='utf-8'))
class TerminalNegationOracle(unittest.TestCase):
 def test_exact_producer_inputs_outputs_dependencies_and_emitted_bytes(self):
  provenance=load('provenance.json')
  for name,pin in provenance['files'].items():self.assertEqual(hashlib.sha256((ROOT/name).read_bytes()).hexdigest(),pin,name)
  self.assertEqual(provenance['jars'],json.loads((ROOT/'fixtures/xtext/syntax-ast/provenance.json').read_text(encoding='utf-8'))['jars'])
  emitted=load('emitted-provenance.json')
  self.assertEqual(emitted['observations_sha256'],hashlib.sha256((FIXTURE/'observations.json').read_bytes()).hexdigest())
  for name,pin in emitted['files'].items():self.assertEqual(hashlib.sha256((ROOT/name).read_bytes()).hexdigest(),pin,name)
 def test_each_original_graph_retains_distinct_stage_outcomes(self):
  cases=load('cases.json');rows=load('observations.json')
  self.assertEqual([c['id'] for c in cases],[r['id'] for r in rows]);self.assertEqual(len(rows),22)
  for case,row in zip(cases,rows):
   self.assertIn(case['entry'],[s['name'] for s in case['sources']]);self.assertEqual([s['name'] for s in case['sources']],[s['source'] for s in row['syntax']]);self.assertEqual(len(row['linking']),len(case['sources']));self.assertEqual(len(row['validation']),len(case['sources']))
   if row['translation_failure'] is not None:self.assertIsNone(row['generation'])
   if row['generation'] is not None and row['generation']['exit_code']==0 and row['generation']['compilation_exit_code']==0:self.assertEqual([v['text'] for v in row['runtime']],case['inputs'])
   else:self.assertEqual(row['runtime'],[])
 def test_larger_operand_success_is_preserved(self):
  rows={r['id']:r for r in load('observations.json')}
  for name in ['multichar','astral_literal']:
   row=rows[name];self.assertTrue(all(not s['errors'] for s in row['syntax']));self.assertTrue(all(not s['errors'] for s in row['linking']));self.assertTrue(all(not s['issues'] and s['failure'] is None for s in row['validation']));self.assertEqual(row['generation']['exit_code'],0);self.assertEqual(row['generation']['compilation_exit_code'],0)
  multichar={v['text']:v for v in rows['multichar']['runtime']}
  for text in ['a','b','c']:self.assertTrue(multichar[text]['complete_single_value'])
  self.assertEqual([t['text'] for t in multichar['ab']['tokens'][:-1]],['a','b'])
  astral={v['text']:v for v in rows['astral_literal']['runtime']};self.assertEqual([t['text'] for t in astral['🌍']['tokens'][:-1]],['\ud83c','\udf0d']);self.assertEqual(astral['🌍']['lexer_errors'],0)
 def test_generator_obstructions_are_separate_from_source_acceptance(self):
  rows={r['id']:r for r in load('observations.json')}
  for name in ['nullable','sequence','eof','wildcard','cyclic_fragment','astral_range']:
   row=rows[name];self.assertTrue(all(not s['errors'] for s in row['syntax']));self.assertIsNone(row['translation_failure']);self.assertNotEqual(row['generation']['exit_code'],0);self.assertIsNone(row['generation']['compilation_exit_code']);self.assertEqual(row['runtime'],[])
  self.assertTrue(any(s['errors'] for s in rows['unknown_fragment']['linking']));self.assertIsNotNone(rows['unknown_fragment']['translation_failure'])
 def test_exact_inherited_binding_and_committed_failure(self):
  rows={r['id']:r for r in load('observations.json')};self.assertEqual(rows['inherited_program_override']['linking'][0]['calls'][0]['target'],'audit.NegationProgramChild::LETTER');explicit=rows['explicit_parent'];self.assertEqual(explicit['linking'][1]['calls'][1]['target'],'audit.NegationBaseExplicit::LETTER');self.assertTrue(explicit['linking'][1]['calls'][1]['explicit']);self.assertEqual(explicit['translated_rules'][0]['body'],'~(SUPER_LETTER)')
  values={v['text']:v for v in rows['committed_path']['runtime']}
  for text in ['ad','ac']:self.assertEqual(values[text]['lexer_errors'],1);self.assertEqual([t['name'] for t in values[text]['tokens']],['EOF'])
  self.assertTrue(values['adc']['complete_single_value']);self.assertEqual([t['name'] for t in values['abc']['tokens']],['RULE_SHORT','RULE_REST','RULE_REST','EOF'])
 def test_traceback_exception_cannot_hide_changed_semantics(self):
  rows=load('observations.json');changed=json.loads(json.dumps(rows));index=next(i for i,r in enumerate(rows) if r['id']=='cyclic_fragment');changed[index]['generation']['diagnostic']+='\nJVM stack cutoff variation\n';self.assertEqual(len(compare_observations(changed,rows)),1)
  changed[index]['generation']['diagnostic']='StackOverflowError without required recursive frames'
  with self.assertRaises(AssertionError):compare_observations(changed,rows)
  changed=json.loads(json.dumps(rows));changed[0]['runtime'][0]['lexer_errors']+=1
  with self.assertRaises(AssertionError):compare_observations(changed,rows)
 def test_separate_nested_companion_pins_and_exact_unit_results(self):
  provenance=load('nested-provenance.json')
  for name,pin in provenance['files'].items():self.assertEqual(hashlib.sha256((ROOT/name).read_bytes()).hexdigest(),pin,name)
  cases=load('nested-cases.json');rows=load('nested-observations.json');self.assertEqual(len(cases),2);self.assertEqual([c['id']for c in cases],[r['id']for r in rows])
  for case,row in zip(cases,rows):
   self.assertTrue(all(not s['errors']for s in row['syntax']));self.assertTrue(all(not s['errors']for s in row['linking']));self.assertTrue(all(not s['issues']and s['failure']is None for s in row['validation']));self.assertIsNone(row['translation_failure']);self.assertEqual(row['generation_exit'],0);self.assertEqual([v['text']for v in row['runtime']],case['inputs'])
   self.assertEqual([(v['text'],v['complete_single_value'],v['lexer_errors'])for v in row['runtime']],[('a',True,0),('b',False,1),('c',False,1)])
if __name__=='__main__':unittest.main()
