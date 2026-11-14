from pathlib import Path
import hashlib,json,subprocess,sys,time
H=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8',newline='\n')
pins=json.loads((H/'inputs/pins.json').read_text());before=sha(H/'preexecution.json');producer=sha(H/'oracle-producer/produce.py');runs=[]
def check():
 assert sha(H/'preexecution.json')==before;assert sha(H/'oracle-producer/produce.py')==producer
 for n,h in pins.items():assert sha(H/'inputs'/n)==h,n
check();save(H/'oracle-run-preexecution.json',{'runner_sha256':sha(Path(__file__)),'preexecution_sha256':before,'producer_sha256':producer,'scope':'Unchanged full source-loaded producer; stages/failures retained for all6graphs. Only exact owned provider GenModel absolute output location may differ between fresh runs.'})
for lane in ['record','replay']:
 out=H/('oracle-'+lane);assert not out.exists();command=[sys.executable,str(H/'oracle-producer/produce.py'),'--output',str(out),'--jdk','D:/dev/jdks/jdk21','--maven-cache','C:/Users/MarkPetrotta/.m2/repository'];start=time.time()
 with (H/('oracle-'+lane+'.log')).open('wb')as log:r=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
 runs.append({'lane':lane,'command':command,'exit_code':r.returncode,'seconds':time.time()-start,'log_sha256':sha(H/('oracle-'+lane+'.log'))});save(H/'oracle-progress.json',runs);check();assert r.returncode==0
def normalize(root):
 rows=json.loads((root/'observations.json').read_text())
 for r in rows:
  if r['provider']is None:continue
  p=Path(r['provider']['genmodel']);h=r['provider']['ecore_sha256'];assert p.is_absolute()and p.name=='provider.genmodel'and p.parent.name=='provider-'+h[:12]and p.parent.parent==root
  r['provider']['genmodel']='<verified-owned-provider>/provider-'+h[:12]+'/provider.genmodel'
 return rows
a=normalize(H/'oracle-record');b=normalize(H/'oracle-replay');assert a==b
save(H/'oracle-terminal.json',{'state':'COMPLETE_SIX_RECORD_FRESH_REPLAY_EXACT','runs':runs,'graphs':len(a),'runtime_attempts':sum(len(r['runtime'])for r in a),'stages':[{'id':r['id'],'provider':r['provider'],'generation':r['generation'],'compilation':r['compilation'],'runtime':len(r['runtime']),'metadata':r['metadata']}for r in a],'input_pins_unchanged':True})
print('PASS six negative graphs complete record/replay',flush=True)
