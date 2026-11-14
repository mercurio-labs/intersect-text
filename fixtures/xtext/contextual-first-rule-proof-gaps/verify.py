"""Pure proposed corpus integrity; executes no recorded native/Java paths."""
from pathlib import Path, PureWindowsPath
import copy,hashlib,json,os,argparse,subprocess,sys
HERE=Path(__file__).resolve().parent
if os.name=='nt'and not str(HERE).startswith('\\\\?\\'):HERE=Path('\\\\?\\'+str(HERE))

def load(p):return json.loads(p.read_text(encoding='utf-8'))
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def canonical(rows,lane,provenance):
 result=copy.deepcopy(rows)
 root=PureWindowsPath(provenance['original_output_roots'][lane])
 for row in result:
  provider=row['provider'];expected=root/('provider-'+provider['ecore_sha256'][:12])/'provider.genmodel'
  actual=PureWindowsPath(provider['genmodel'])
  assert actual.is_absolute()and actual==expected,'Unowned original GenModel path'
  provider['genmodel']='OWNED_OUTPUT/'+expected.parent.name+'/provider.genmodel'
 return result

def verify():
 provenance=load(HERE/'provenance.json')
 actual={p.relative_to(HERE).as_posix()for p in HERE.rglob('*')if p.is_file()and p.name!='provenance.json'}
 assert actual==set(provenance['files']),'Extra or missing corpus files'
 for name,pin in provenance['files'].items():
  p=HERE/name;assert len(p.read_bytes())==pin['bytes']and digest(p)==pin['sha256'],name
 cases=load(HERE/'inputs/cases.json');record=load(HERE/'oracle/record/observations.json');replay=load(HERE/'oracle/replay/observations.json')
 assert len(cases)==5 and sum(len(c['inputs'])for c in cases)==25
 assert canonical(record,'record',provenance)==canonical(replay,'replay',provenance)
 for lane in ['record','replay']:
  root=HERE/'oracle'/lane;manifest=load(root/'artifacts-sha256.json')
  assert set(manifest)=={p.relative_to(root).as_posix()for p in root.rglob('*')if p.is_file()and p.name!='artifacts-sha256.json'}
  for name,h in manifest.items():assert digest(root/name)==h,name
 native=load(HERE/'native/native-cases.json');helper=load(HERE/'native/helper/helper-record.json');helper_replay=load(HERE/'native/helper/helper-replay.json');before=load(HERE/'native/before/record.json');before_replay=load(HERE/'native/before/replay.json')
 assert helper==helper_replay and helper['input']==native
 assert before['transcript']==before_replay['transcript']and len(before['transcript'])==17
 for c,n,h,o,b in zip(cases,native['cases'],helper['cases'],record,before['cases']):
  assert c['id']==n['id']==h['id']==o['id']==b['id'];assert h['request']['bundle']==n['compilation']['bundle'];assert h['request']['entry']==n['compilation']['xtext']
  for s in n['compilation']['bundle']['sources']:assert hashlib.sha256(s['text'].encode('utf-8')).hexdigest()==s['id']['sha256']
  for source in c['sources']:
   path=HERE/'inputs'/c['id']/source['file'];assert digest(path)==source['source_id']['sha256']
   item=next(s for s in n['compilation']['bundle']['sources']if s['id']==source['source_id']);assert path.read_bytes()==item['text'].encode('utf-8')
  errors=[(v['source'],i)for v in o['metadata']['observation']['validation']for i in v['issues']if i['severity']=='ERROR']
  assert h['result']['ok']==(not errors)
  if not h['result']['ok']:
   d=h['result']['diagnostic'];source=next(s for s in c['sources']if s['source_id']['uri']==d['source']);assert any(g==source['grammar']and i['message']==d['message']for g,i in errors)
   origin=h['result']['origin'];assert origin['source']==source['source_id']and origin['span']['source']==source['source_id'];assert origin['span']['start']==d['start']and origin['span']['end']==d['end']
 assert [h['result']['ok']for h in helper['cases']]==[True,False,True,True,False]
 return {'graphs':5,'planned':25,'helper_admit':3,'helper_reject':2,'native_before_calls':17,'native_projects':5,'production_change':False}
def oracle_replay(output,jdk,maven):
 output=output.resolve();assert not output.exists(),'Never overwrite oracle evidence';output.mkdir(parents=True)
 stage=output/'producer';stage.mkdir();(stage/'inputs').mkdir()
 # Relocate byte-exact pinned original producer/inputs to a short NEW runtime root.
 # No original source/scaffold/producer repair or corpus writes.
 for name in ['produce.py','BuildProvider.java','FullRuntime.java','FirstRuleMetadata.java']:(stage/name).write_bytes((HERE/name).read_bytes())
 for p in (HERE/'inputs').rglob('*'):
  if not p.is_file():continue
  dest=stage/'inputs'/p.relative_to(HERE/'inputs');dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(p.read_bytes())
 commands=[]
 for lane in ['record','replay']:
  cmd=[sys.executable,'-B',str(stage/'produce.py'),'--output',str(output/lane),'--jdk',str(jdk.resolve()),'--maven-cache',str(maven.resolve())]
  with(output/(lane+'.log')).open('w',encoding='utf-8',newline='\n')as f:r=subprocess.run(cmd,stdout=f,stderr=subprocess.STDOUT,text=True,encoding='utf-8')
  commands.append({'command':cmd,'exit_code':r.returncode});assert r.returncode==0,lane
 frozen=load(HERE/'provenance.json');receipts=[]
 for lane in ['record','replay']:
  rows=load(output/lane/'observations.json');temporary={'original_output_roots':{lane:str(output/lane)}}
  got=canonical(rows,lane,temporary)
  for original in ['record','replay']:assert got==canonical(load(HERE/'oracle'/original/'observations.json'),original,frozen)
  root=output/lane;manifest=load(root/'artifacts-sha256.json')
  assert set(manifest)=={p.relative_to(root).as_posix()for p in root.rglob('*')if p.is_file()and p.name!='artifacts-sha256.json'}
  for name,h in manifest.items():assert digest(root/name)==h
  tool=load(root/'toolchain.json');old=load(HERE/'oracle/record/toolchain.json')
  for field in ['jdk','java_sha256','javac_sha256','jars','producer_sha256','BuildProvider_sha256','FullRuntime_sha256','FirstRuleMetadata_sha256','input_pins_sha256']:assert tool[field]==old[field],field
  receipts.append({'lane':lane,'observations_sha256':digest(root/'observations.json'),'raw_artifact_pins':len(manifest)})
 receipt={'state':'All five complete fresh original oracle record/replay observations equal BOTH frozen runs; only strict owned GenModel path differs.','commands':commands,'runs':receipts,'production_change':False}
 (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8');return receipt
if __name__=='__main__':
 a=argparse.ArgumentParser();a.add_argument('--oracle-replay',type=Path);a.add_argument('--jdk',type=Path);a.add_argument('--maven-cache',type=Path);x=a.parse_args();result=verify()
 if x.oracle_replay:
  assert x.jdk and x.maven_cache,'Explicit pinned JDK and Maven cache required';result['oracle_replay']=oracle_replay(x.oracle_replay,x.jdk,x.maven_cache)
 print(json.dumps(result))
