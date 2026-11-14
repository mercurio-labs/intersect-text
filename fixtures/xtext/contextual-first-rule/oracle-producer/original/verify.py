"""Independent replay comparison; preserves raw files and all semantic observations."""
from pathlib import Path
import argparse,copy,hashlib,json
HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8'))
def pin(p):return {'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)}
def normalized(root,rows):
 result=copy.deepcopy(rows)
 for row in result:
  provider=row['provider'];expected=(root/('provider-'+provider['ecore_sha256'][:12])/'provider.genmodel').resolve()
  actual=Path(provider['genmodel'])
  assert actual.is_absolute() and actual.resolve()==expected,(row['id'],actual,expected)
  assert expected.is_file() and expected.parent.parent==root.resolve()
  provider['genmodel']='OWNED_OUTPUT_ROOT/'+expected.parent.name+'/provider.genmodel'
 return result
def main():
 a=argparse.ArgumentParser();a.add_argument('--record',type=Path,required=True);a.add_argument('--replay',type=Path,required=True);a.add_argument('--receipt',type=Path,required=True);x=a.parse_args()
 for name,digest in read(HERE/'inputs/pins.json').items():assert sha(HERE/'inputs'/name)==digest,name
 cases=read(HERE/'inputs/cases.json');runs={}
 for key,root in [('record',x.record.resolve()),('replay',x.replay.resolve())]:
  files=read(root/'artifacts-sha256.json');actual={p.relative_to(root).as_posix()for p in root.rglob('*')if p.is_file()and p.name!='artifacts-sha256.json'};assert set(files)==actual
  for name,digest in files.items():assert sha(root/name)==digest,(key,name)
  rows=read(root/'observations.json');assert [c['id']for c in cases]==[r['id']for r in rows]
  commands=read(root/'commands.json');runtime=[c for c in commands if 'intersect.oracle.FullRuntime'in c['command']]
  assert len(runtime)==sum(len(r['runtime'])for r in rows)
  for c,r in zip(cases,rows):
   assert r['entry']==c['entry']and r['original_inputs']==c['inputs']
   assert r['source_pins']=={p.name:sha(p)for p in (HERE/'inputs'/c['id']).iterdir()if p.is_file()}
   assert r['metadata']['exit_code']==0 and r['metadata']['observation']is not None
   if r['generation']is not None:assert r['generation']['source_unchanged']
   if r['compilation']and r['compilation']['exit_code']==0:assert [v['text']for v in r['runtime']]==c['inputs']
   else:assert not r['runtime']
  runs[key]={'rows':rows,'observations':pin(root/'observations.json'),'artifact_manifest':pin(root/'artifacts-sha256.json'),'artifact_pins':len(files),'commands':len(commands),'runtime_processes':len(runtime),'toolchain':pin(root/'toolchain.json')}
 assert normalized(x.record.resolve(),runs['record']['rows'])==normalized(x.replay.resolve(),runs['replay']['rows'])
 result={'version':1,'state':'original_contextual_first_rule_record_and_independent_replay_equal','graphs':len(cases),'planned_inputs_per_run':sum(len(c['inputs'])for c in cases),'comparison_exception':'Only row.provider.genmodel after validating exact absolute owned provider-<EcoreSHA12>/provider.genmodel in its run root. No other output normalization.','preexecution':pin(HERE/'preexecution.json'),'cases':[],'runs':{k:{a:b for a,b in v.items()if a!='rows'}for k,v in runs.items()},'native_support_award':False,'gate_closures':[]}
 for c,r in zip(cases,runs['record']['rows']):
  result['cases'].append({'id':c['id'],'syntax':r['metadata']['observation']['syntax'],'validation':r['metadata']['observation']['validation'],'first_rule_metadata':r['first_rule_metadata'],'provider':{'generation_exit':r['provider']['generation_exit'],'compile_exit':r['provider']['compile_exit']},'generation':r['generation'],'compilation':r['compilation'],'planned_inputs':len(c['inputs']),'actual_runtime_inputs_per_run':len(r['runtime']),'unexecuted_inputs':c['inputs']if not r['runtime']else [],'accepted_per_run':sum(bool(v['observation']and v['observation']['resource']and v['observation']['resource'].get('accepted'))for v in r['runtime'])})
 result['producer']=pin(Path(__file__));x.receipt.write_text(json.dumps(result,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n');print(json.dumps(pin(x.receipt)))
if __name__=='__main__':main()
