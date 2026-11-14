"""Independent full-source Xtext oracle; preserves failed and unexecuted stages."""
import base64, copy, hashlib, json, ntpath, os, subprocess, sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
INPUT=HERE/'oracle-inputs-v1'
OUTPUT=HERE.parent/'grouped-negation-full-oracle-v1'
def raw(p):
 p=Path(p).resolve()
 if os.name=='nt'and not str(p).startswith('\\\\?\\'):p=Path('\\\\?\\'+str(p))
 return p.read_bytes()
def pin(p):
 b=raw(p);return {'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
def read(p):return json.loads(raw(p).decode('utf-8'))
def save(p,v):p.write_text(json.dumps(v,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
def check(root,cases):
 manifest=read(root/'artifacts-sha256.json')
 for n,h in manifest.items():assert pin(root/n)['sha256']==h,(root,n)
 actual={p.relative_to(root).as_posix()for p in root.rglob('*')if p.is_file()and p.name!='artifacts-sha256.json'};assert set(manifest)==actual
 rows=read(root/'observations.json');assert [r['id']for r in rows]==[c['id']for c in cases]
 runtime_commands=[c for c in read(root/'commands.json')if 'intersect.oracle.FullRuntime'in c['command']];expected=[]
 for c,r in zip(cases,rows):
  assert r['entry']==c['entry']and r['original_inputs']==c['inputs']
  assert r['source_pins']=={p.name:pin(p)['sha256']for p in (INPUT/'inputs'/c['id']).iterdir()if p.is_file()}
  if r['generation']:assert r['generation']['source_unchanged']
  if r['compilation']and r['compilation']['exit_code']==0:assert [v['text']for v in r['runtime']]==c['inputs']
  else:assert r['runtime']==[]
  for v in r['runtime']:expected.append((c['entry'],base64.b64encode(v['text'].encode('utf-8')).decode('ascii'),v['exit_code']))
 assert len(runtime_commands)==len(expected)
 for command,(entry,text,code)in zip(runtime_commands,expected):assert command['command'][-2:]==[entry,text]and command['exit_code']==code
 assert '21.0.6+7'in read(root/'toolchain.json')['jdk']
 normalized=copy.deepcopy(rows)
 for row in normalized:
  provider=row['provider'];assert provider['genmodel']==ntpath.join(str(root),'provider-'+provider['ecore_sha256'][:12],'provider.genmodel')
  assert (root/('provider-'+provider['ecore_sha256'][:12])/'provider.genmodel').is_file()
  provider['genmodel']='<owned-output>/provider.genmodel'
 return rows,normalized,{'artifact_pins':len(manifest),'observations':pin(root/'observations.json'),'manifest':pin(root/'artifacts-sha256.json'),'toolchain':pin(root/'toolchain.json')}
def main():
 assert not OUTPUT.exists();OUTPUT.mkdir();freeze=read(INPUT/'preexecution.json')
 for n,p in freeze['files'].items():assert pin(INPUT/n)==p,n
 cases=read(INPUT/'inputs/cases.json');commands=[];receipt={'state':'independent_full_source_record_replay_started','input_freeze':pin(INPUT/'preexecution.json'),'producer':pin(INPUT/'produce.py'),'verifier':pin(Path(__file__)),'commands':commands,'native_support_award':False,'gate_closures':[]};save(OUTPUT/'preexecution.json',receipt)
 try:
  runs={};normalized=[]
  for key in ['record','replay']:
   command=[sys.executable,'-B',str(INPUT/'produce.py'),'--output',str(OUTPUT/key),'--jdk','D:/dev/jdks/jdk21','--maven-cache','C:/Users/MarkPetrotta/.m2/repository']
   with(OUTPUT/(key+'.stdout.log')).open('wb')as stdout,(OUTPUT/(key+'.stderr.log')).open('wb')as stderr:r=subprocess.run(command,stdout=stdout,stderr=stderr)
   commands.append({'command':command,'exit_code':r.returncode});save(OUTPUT/'receipt.json',receipt);assert r.returncode==0,(key,r.returncode)
   rows,norm,info=check(OUTPUT/key,cases);normalized.append(norm);runs[key]=info
  assert normalized[0]==normalized[1]
  for n,p in freeze['files'].items():assert pin(INPUT/n)==p,n
  receipt.update(state='independent_full_source_record_and_fresh_replay_complete_observations_equal',runs=runs,comparison_exception='Only exact absolute owned provider-<EcoreSHA12>/provider.genmodel output path. No other normalization.',graphs=len(cases),planned_inputs=sum(len(c['inputs'])for c in cases),actual_runtime_inputs=sum(len(r['runtime'])for r in rows),generation_stages=[{'id':r['id'],'generation':r['generation'],'compilation':r['compilation'],'actual_runtime_inputs':len(r['runtime']),'unexecuted_inputs':r['original_inputs']if not r['runtime']else []}for r in rows])
 except BaseException as failure:
  receipt.update(state='failed_or_incomplete_original_stage_preserved',failure={'class':type(failure).__name__,'message':str(failure)});save(OUTPUT/'receipt.json',receipt);raise
 save(OUTPUT/'receipt.json',receipt);print(json.dumps({'path':str(OUTPUT/'receipt.json'),**pin(OUTPUT/'receipt.json')}))
if __name__=='__main__':main()
