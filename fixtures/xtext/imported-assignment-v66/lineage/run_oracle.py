from pathlib import Path
import hashlib,json,subprocess,sys,time
H=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2,ensure_ascii=True)+'\n',encoding='utf-8',newline='\n')
pre=H/'preexecution.json';assert sha(pre)=='14f0054aed8b76a7b28eb98b00670661ae474d2869eab00b5f9d4e58a4767431'
pins=json.loads((H/'inputs/pins.json').read_text(encoding='utf-8'))
def check():
 for n,h in pins.items():assert sha(H/'inputs'/n)==h,n
 for x in json.loads(pre.read_text(encoding='utf-8'))['producer']:assert sha(Path(x['copy']['path']))==x['copy']['sha256']
check();runs=[]
for lane in ['record','replay']:
 out=H/('oracle-'+lane);assert not out.exists();start=time.time();command=[sys.executable,str(H/'oracle-producer/produce.py'),'--output',str(out),'--jdk','D:/dev/jdks/jdk21','--maven-cache','C:/Users/MarkPetrotta/.m2/repository']
 with (H/('oracle-'+lane+'.log')).open('wb')as log:r=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
 runs.append({'lane':lane,'command':command,'exit_code':r.returncode,'seconds':time.time()-start,'log_sha256':sha(H/('oracle-'+lane+'.log'))});save(H/'oracle-progress.json',runs);check();assert r.returncode==0,lane
def normalized(path):
 rows=json.loads((path/'observations.json').read_text(encoding='utf-8'))
 for row in rows:
  p=Path(row['provider']['genmodel']);h=row['provider']['ecore_sha256'];assert p.is_absolute()and p.name=='provider.genmodel'and p.parent.name=='provider-'+h[:12]and p.parent.parent==path
  row['provider']['genmodel']='<verified-owned-provider-output>/provider-'+h[:12]+'/provider.genmodel'
 return rows
a=normalized(H/'oracle-record');b=normalized(H/'oracle-replay');assert a==b
stages=[{'id':r['id'],'metadata_exit':r['metadata']['exit_code'],'generation':r['generation'],'compilation':r['compilation'],'runtime':len(r['runtime']),'accepted':sum(bool(x.get('observation',{}).get('resource',{}).get('accepted'))for x in r['runtime'])}for r in a]
save(H/'oracle-terminal.json',{'state':'COMPLETE_RECORD_AND_FRESH_REPLAY_EQUAL','runs':runs,'sole_exception':'Strictly verified absolute owned provider.genmodel location; no semantic, diagnostic, generated, source, token, model or partial result exclusions.','stages':stages,'observations':[{'path':str(H/('oracle-'+lane)/'observations.json'),'sha256':sha(H/('oracle-'+lane)/'observations.json')}for lane in ['record','replay']],'preexecution_sha256':sha(pre),'post_input_pins_equal':True})
print(json.dumps(stages),flush=True)
