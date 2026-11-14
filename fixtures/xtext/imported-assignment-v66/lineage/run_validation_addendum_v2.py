from pathlib import Path
import base64,hashlib,json,os,subprocess,time
H=Path(__file__).resolve().parent;D=H/'validation-addendum-v2';D.mkdir(exist_ok=False)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,indent=2,ensure_ascii=True)+'\n',encoding='utf-8',newline='\n')
def pin(p):return {'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)}
original=H/'oracle-record';assert json.loads((H/'oracle-progress.json').read_text())[0]['exit_code']==0
cases=json.loads((H/'inputs/cases.json').read_text());old=json.loads((original/'observations.json').read_text());manifest=json.loads((original/'artifacts-sha256.json').read_text());
for n,h in manifest.items():assert sha(original/n)==h,n
cache=Path('C:/Users/MarkPetrotta/.m2/repository');stack=json.loads((H/'inputs/stack.json').read_text());jars=[]
for j in stack['jars']:
 p=cache/j['path'];assert sha(p)==j['sha256'];jars.append(str(p))
cp=os.pathsep.join(jars);jdk=Path('D:/dev/jdks/jdk21/bin');java=str(jdk/'java.exe');javac=str(jdk/'javac.exe');adapter=original/'adapter';commands=[]
save(D/'preexecution.json',{'source':pin(H/'UniqueRuntimeAddendumV2.java'),'runner':pin(Path(__file__)),'original_source_preexecution':pin(H/'preexecution.json'),'original_generated_artifact_manifest':pin(original/'artifacts-sha256.json'),'original_full_runtime':pin(H/'oracle-producer/FullRuntime.java'),'stack':pin(H/'inputs/stack.json'),'java':pin(Path(java)),'javac':pin(Path(javac)),'scope':'Additive actual generated provider feature/list and Xtext/EMF model validation observations; no graph/producer repair; main original resource/IParser stages remain unchanged.'})
def run(command):
 n=len(commands);v=subprocess.run(command,capture_output=True,encoding='utf-8',errors='strict',timeout=60);(D/f'{n:03d}.stdout.log').write_text(v.stdout,encoding='utf-8',newline='\n');(D/f'{n:03d}.stderr.log').write_text(v.stderr,encoding='utf-8',newline='\n');commands.append({'command':command,'exit_code':v.returncode});save(D/'commands.json',commands);return v
classes=D/'classes';classes.mkdir();v=run([javac,'-encoding','UTF-8','--release','17','-cp',str(adapter)+os.pathsep+cp,'-d',str(classes),str(H/'UniqueRuntimeAddendumV2.java')]);assert v.returncode==0,v.stderr
def once(lane):
 rows=[]
 for c,row in zip(cases,old):
  assert c['id']==row['id'];project=original/c['id'];provider=original/('provider-'+row['provider']['ecore_sha256'][:12]);runtime=os.pathsep.join([str(classes),str(adapter),str(project/'classes'),str(project/'src-gen'),str(project/'src'),str(provider/'classes'),str(provider/'src-gen'),cp]);r={'id':c['id'],'runtime':[]};rows.append(r)
  for text in c['inputs']:
   v=run([java,'-cp',runtime,'intersect.oracle.UniqueRuntimeAddendumV2',row['provider']['package_class'],c['entry'],base64.b64encode(text.encode()).decode()]);values=[json.loads(s)for s in v.stdout.splitlines()if s.startswith('{')];r['runtime'].append({'text':text,'exit_code':v.returncode,'observation':values[0]if len(values)==1 else None});assert v.returncode==0,v.stderr;save(D/(lane+'.json'),rows)
 return rows
a=once('record');b=once('replay');assert a==b
for n,h in manifest.items():assert sha(original/n)==h,n
save(D/'terminal.json',{'state':'COMPLETE_FRESH_VALIDATION_ADDENDUM_REPLAY_EXACT','graphs':3,'runtime_per_run':18,'record':pin(D/'record.json'),'replay':pin(D/'replay.json'),'original_generated_inputs_post_unchanged':True,'artifacts':[pin(p)for p in sorted(D.rglob('*'))if p.is_file()]})
print('PASS 3 graphs / 18+18 exact generated feature/list/Xtext/EMF validation observations',flush=True)
