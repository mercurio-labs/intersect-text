"""Read-only output verification; creates a NEW final receipt, never rewrites an attempt."""
from pathlib import Path
import copy,hashlib,json
R=Path(__file__).resolve().parent
B=R.parent
OLD=B/'terminal-return-identity-after-v62-v1/frozen-inputs-v1'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def load(p):return json.loads(p.read_text(encoding='utf-8'))
def save(p,v):
 assert not p.exists(),p
 p.write_text(json.dumps(v,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
def verify_pins(root,pins):
 for n,h in pins.items():
  p=root/n
  assert p.is_file() and sha(p)==h,(p,h)
 return len(pins)
def comparable(row,output):
 v=copy.deepcopy(row)
 p=Path(v['provider']['genmodel'])
 expected=output/('provider-'+v['provider']['ecore_sha256'][:12])/'provider.genmodel'
 assert p.is_absolute() and p.resolve()==expected.resolve(),(p,expected)
 assert p.name=='provider.genmodel' and p.parent.name=='provider-'+v['provider']['ecore_sha256'][:12]
 assert p.is_file()
 v['provider']['genmodel']='<verified-owned-provider.genmodel>'
 return v
def main():
 cases=load(R/'inputs/cases.json')
 original=[c for c in load(OLD/'inputs/cases.json') if c['id'] in {r['id'] for r in cases}]
 assert cases==original and len(cases)==2
 for c in cases:
  a=R/'inputs'/c['id'];b=OLD/'inputs'/c['id']
  assert {p.name:sha(p) for p in a.iterdir() if p.is_file()}=={p.name:sha(p) for p in b.iterdir() if p.is_file()}
  for s in c['sources']:assert sha(a/s['file'])==s['source_id']['sha256']
 count=verify_pins(R/'inputs',load(R/'inputs/pins.json'))
 for n in ['BuildProvider.java','FullRuntime.java']:assert sha(R/n)==sha(OLD/n)
 assert sha(R/'original/produce.py')==sha(OLD/'produce.py')
 pinchecks=[]
 for n in range(1,5):
  pre=load(R/f'preexecution-attempt{n}.json')
  if n==1:verify_pins(R,pre['files'])
  else:assert sha(R/f'produce-attempt{n}.py')==pre['producer_sha256']
  out=B/f'tilw-a{n}-record'
  k=verify_pins(out,load(out/'artifacts-sha256.json'))
  rows=load(out/'observations.json')
  assert [x['id'] for x in rows]==[x['id'] for x in cases]
  for c,row in zip(cases,rows):
   assert row['original_inputs']==c['inputs']
   assert all(row['source_pins'][s['file']]==s['source_id']['sha256'] for s in c['sources'])
  pinchecks.append({'attempt':n,'artifact_pins_verified':k,'observations_sha256':sha(out/'observations.json'),'commands_sha256':sha(out/'commands.json'),'commands':len(load(out/'commands.json')),'row_stage_summary':[{'id':x['id'],'generation':x['generation'],'compilation':x['compilation'],'runtime_exit_codes':[v['exit_code'] for v in x['runtime']]} for x in rows]})
 record=B/'tilw-a4-record';replay=B/'tilw-a4-replay'
 rc=load(record/'observations.json');rp=load(replay/'observations.json')
 assert [comparable(v,record) for v in rc]==[comparable(v,replay) for v in rp]
 # The sole exception must never normalize a forged unrelated path.
 forged=copy.deepcopy(rc[0]);forged['provider']['genmodel']=str(B/'unrelated/provider.genmodel')
 rejected=False
 try:comparable(forged,record)
 except AssertionError:rejected=True
 assert rejected
 replay_pin_count=verify_pins(replay,load(replay/'artifacts-sha256.json'))
 oldrows={v['id']:v for v in load(B/'trm-oracle-v1/record/observations.json')}
 for row in rc:
  assert row['metadata']==oldrows[row['id']]['metadata']
  assert row['source_pins']==oldrows[row['id']]['source_pins']
  assert row['generation']=={'exit_code':0,'source_unchanged':True}
  assert row['compilation']=={'exit_code':0}
  assert len(row['runtime'])==5 and all(x['exit_code']==0 and x['observation'] is not None for x in row['runtime'])
 assert load(record/'toolchain.json')==load(replay/'toolchain.json')
 runtime=[];generated=[]
 for row in rc:
  for x in row['runtime']:
   o=x['observation'];r=o['resource'];p=o['parser']
   runtime.append({'id':row['id'],'text':x['text'],'resource_accepted':r['accepted'],'parser_accepted':p['accepted'],'resource_partial_model':not r['accepted'] and r['model'] is not None,'resource':r,'parser':p,'lexer':o['lexer']})
  project=record/row['id']
  validator=project/'src-gen/audit/validation'/('Abstract'+row['entry'].split('.')[-1]+'Validator.java')
  text=validator.read_text(encoding='utf-8')
  assert 'extends AbstractDeclarativeValidator' in text and 'ReturnIdentityBaseValidator' not in text
  assert not (project/'src-gen/audit/validation/AbstractReturnIdentityBaseValidator.java').exists()
  module=project/'src-gen/audit/AbstractReturnIdentityBaseRuntimeModule.java'
  assert 'ReturnIdentityBaseValidator' not in module.read_text(encoding='utf-8')
  for p in sorted(project.rglob('*.java')):
   if 'parser' not in p.parts and ('ReturnIdentityBase' in p.name or 'Validator' in p.name or 'StandaloneSetup' in p.name):generated.append({'path':str(p),'sha256':sha(p),'bytes':p.stat().st_size})
 accepted=sum(x['resource_accepted'] for x in runtime);partial=sum(x['resource_partial_model'] for x in runtime)
 assert accepted==2 and partial==1
 receipt={'scope':'Verification-only SAME-SOURCE library-appropriate generated workflow addendum; not a source repair, implementation change, profile qualification or old workflow reclassification.','cases':cases,'input_pins_verified':count,'input_pins_sha256':sha(R/'inputs/pins.json'),'original_case_subset_exact':True,'original_metadata_complete_equal':True,'original_workflow_failures_retained':{'root':str(B/'trm-oracle-v1'),'observation_sha256':sha(B/'trm-oracle-v1/record/observations.json'),'meaning':'Both clean source graphs failed all-StandardLanguage generation. This new workflow does not change those failures.'},'prior_attempts':pinchecks,'final_workflow':{'producer_sha256':sha(R/'produce-attempt4.py'),'base_fragments':['GrammarAccessFragment2','ImportNamespacesScopingFragment2','SerializerFragment2(generateStub=false)'],'child':'StandardLanguage unchanged except validator.inheritImplementation=false; generateDeprecationValidation=false retained.','validator_consequence':'Terminal-only Base has no model productions or authored validation checks. Its unused generated eager empty-EPackage validator is omitted; Child generated validator extends AbstractDeclarativeValidator directly and still registers actual imported model EPackage. Full grammar source syntax/link/validation metadata is unchanged including ExplicitOverrideMissing WARNING. No generated Java edited or handwritten parser/model oracle.','generated_dependencies':generated},'record':{'path':str(record),'observations_sha256':sha(record/'observations.json')},'replay':{'path':str(replay),'observations_sha256':sha(replay/'observations.json'),'artifact_pins_verified':replay_pin_count},'equality':'Complete row equality with sole strict absolute owned provider-<EcoreSHA12>/provider.genmodel path exception; forged unrelated path rejected. No token/error/model/span normalization. Toolchain complete equal.','per_run':{'graphs':2,'planned':10,'actual_runtime':10,'all_runtime_exit_zero':True,'resource_accepted':accepted,'resource_rejected':10-accepted,'rejected_partial_models':partial,'timeouts':0},'runtime':runtime,'native_boundary':'Separate prior native-context addendum ordinary compile/export positives remain distinct; default inference declaration-library failures remain. This is upstream generated-language evidence only, no native Ecore/support gate award.','source_pins':{p.relative_to(R).as_posix():sha(p) for p in sorted(R.rglob('*')) if p.is_file()}}
 save(R/'final-receipt.json',receipt)
 print(json.dumps({'receipt':str(R/'final-receipt.json'),'sha256':sha(R/'final-receipt.json'),'per_run':receipt['per_run'],'replay_pins':replay_pin_count}))
if __name__=='__main__':main()
