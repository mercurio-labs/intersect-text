"""Native consumer views must retain exact original graphs and complete outcomes."""
import hashlib
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
NEW=ROOT/'fixtures/xtext/full-generated-until'
OLD=ROOT/'fixtures/xtext/full-generated-lexical'
def read(path):return json.loads(path.read_text(encoding='utf-8'))
class NativeUntilEvidence(unittest.TestCase):
 def test_original_source_bytes_uris_and_migration_adapter_are_unchanged(self):
  native=read(NEW/'native-cases.json');first=read(OLD/'native-cases.json');cases=read(NEW/'inputs/cases.json')
  canonical=next(s for s in first['cases'][0]['compilation']['bundle']['sources']if s['kind']=='rcore')
  self.assertEqual(len(native['cases']),22)
  for row in native['cases']:
   compilation=row['compilation'];self.assertEqual(next(s for s in compilation['bundle']['sources']if s['kind']=='rcore'),canonical)
   for source in compilation['bundle']['sources']:self.assertEqual(hashlib.sha256(source['text'].encode('utf-8')).hexdigest(),source['id']['sha256'])
   if row['cohort']=='first-seven':
    original=next(c for c in first['cases']if c['id']==row['id']);self.assertEqual(compilation,original['compilation']);self.assertEqual(row['inputs'],[r['text']for r in next(c for c in read(OLD/'observations.json')if c['id']==row['id'])['runtime']]);continue
   original=next(c for c in cases if c['id']==row['id']);self.assertEqual(row['inputs'],original['inputs'])
   for source in original['sources']:
    selected=next(s for s in compilation['bundle']['sources']if s['id']==source['source_id'])
    self.assertEqual(selected['text'].encode('utf-8'),(NEW/'inputs'/row['id']/source['file']).read_bytes())
   self.assertEqual(compilation['xtext'],next(s['source_id']for s in original['sources']if s['grammar']==original['entry']))
 def test_derived_native_view_preserves_every_original_runtime_and_partial_boundary(self):
  original=read(NEW/'observations.json')+[c for c in read(OLD/'observations.json')if c['id']in['alternatives_companion','fragment_companion','abc_or_b','ab_or_abc']]
  native=read(NEW/'native-observations.json')['cases'];self.assertEqual(len(original),22);self.assertEqual(len(native),22)
  total=accepted=partials=first_value=0
  for row in original:
   view=next(c for c in native if c['id']==row['id']);self.assertEqual(view['upstream_generated'],row['generation']['exit_code']==0 and row['compilation']['exit_code']==0)
   self.assertEqual(len(view['runtime']),len(row['runtime']))
   for a,b in zip(view['runtime'],row['runtime']):
    total+=1;resource=b['observation']['resource'];token=b['observation']['lexer']['tokens'][0]
    self.assertEqual(a['text'],b['text']);self.assertEqual(a['resource_accepted'],resource['accepted']);self.assertEqual(a['parser_accepted'],b['observation']['parser']['accepted'])
    self.assertEqual(a['resource_model_count'],len(resource.get('model')or[]));self.assertEqual(a['first_token_name'],token['name']);self.assertEqual(a['first_token_utf16_end'],token['utf16_stop']+1)
    accepted+=resource['accepted'];partials+=not resource['accepted']and bool(resource.get('model'));first_value+=token['name']=='RULE_VALUE'
    if resource['accepted']:self.assertEqual(a['resource_model'],resource['model']);self.assertEqual(a['leaves'],resource['leaves'])
    else:self.assertNotIn('resource_model',a)
  self.assertEqual((total,accepted,partials,first_value),(268,69,85,151))
  for id in ['inherited_operand_override','explicit_parent_operand']:
   view=next(c for c in native if c['id']==id);self.assertFalse(view['upstream_generated']);self.assertEqual(view['runtime'],[])
 def test_literal_companion_is_separate_exact_source_and_complete_seven_observations(self):
  path=ROOT/'fixtures/xtext/full-generated-until-literal-suffix';c=read(path/'native-case.json');original=read(path/'inputs/cases.json')[0];view=read(path/'native-observations.json');full=read(path/'observations.json')[0]
  self.assertEqual(c['inputs'],original['inputs']);self.assertEqual(c['id'],original['id']);self.assertEqual(len(view['runtime']),7)
  s=original['sources'][0];source=next(s for s in c['compilation']['bundle']['sources']if s['kind']=='xtext');self.assertEqual(source['id'],s['source_id']);self.assertEqual(source['text'].encode('utf-8'),(path/'inputs'/c['id']/s['file']).read_bytes())
  for a,b in zip(view['runtime'],full['runtime']):self.assertEqual(a['text'],b['text']);self.assertEqual(a['resource'],b['observation']['resource']);self.assertEqual(a['first_token'],b['observation']['lexer']['tokens'][0])
  self.assertEqual(sum(r['resource']['accepted']for r in view['runtime']),4);self.assertEqual(sum(not r['resource']['accepted']and bool(r['resource']['model'])for r in view['runtime']),2)
if __name__=='__main__':unittest.main()
