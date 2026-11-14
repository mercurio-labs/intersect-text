"""Verification-only full generated Xtext languages from unchanged source graphs."""
import argparse,base64,hashlib,json,os,re,subprocess,shutil
from pathlib import Path
HERE=Path(__file__).resolve().parent

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
def main():
 a=argparse.ArgumentParser();a.add_argument('--output',type=Path,required=True);a.add_argument('--only',nargs='*');a.add_argument('--jdk',type=Path,required=True);a.add_argument('--maven-cache',type=Path,required=True);args=a.parse_args();out=args.output.resolve();protected=HERE;assert out!=protected and protected not in out.parents,'Output must stay outside fixtures';out.mkdir(exist_ok=False)
 pins=json.loads((HERE/'inputs/pins.json').read_text(encoding='utf-8'))
 for name,h in pins.items():assert sha(HERE/'inputs'/name)==h,name
 stack=json.loads((HERE/'inputs/stack.json').read_text(encoding='utf-8'));jars=[]
 for j in stack['jars']:
  p=args.maven_cache.resolve()/j['path'];assert sha(p)==j['sha256'];jars.append(str(p))
 source_archive=args.maven_cache.resolve()/stack['source_archive']['path'];assert sha(source_archive)==stack['source_archive']['sha256']
 cp=os.pathsep.join(jars);suffix='.exe'if os.name=='nt'else '';java=str(args.jdk.resolve()/'bin'/('java'+suffix));javac=str(args.jdk.resolve()/'bin'/('javac'+suffix));commands=[];rows=[]
 def run(cmd,timeout=180,cwd=None):
  n=len(commands)+1
  try:v=subprocess.run(cmd,cwd=cwd or out,capture_output=True,text=True,encoding='utf-8',errors='strict',timeout=timeout)
  except subprocess.TimeoutExpired as ex:v=subprocess.CompletedProcess(cmd,124,ex.stdout.decode('utf-8')if isinstance(ex.stdout,bytes)else ex.stdout or '',ex.stderr.decode('utf-8')if isinstance(ex.stderr,bytes)else ex.stderr or '')
  (out/f'{n:03d}.stdout.log').write_text(v.stdout,encoding='utf-8',newline='\n');(out/f'{n:03d}.stderr.log').write_text(v.stderr,encoding='utf-8',newline='\n');commands.append({'command':cmd,'exit_code':v.returncode,'timeout_seconds':timeout,'stdout':f'{n:03d}.stdout.log','stderr':f'{n:03d}.stderr.log'});save(out/'commands.json',commands);return v
 def compile(files,classpath,dest):
  dest.mkdir(exist_ok=True);arg=out/f'javac-{len(commands)}.args';parts=['-encoding','UTF-8','--release','17','-cp',classpath,'-d',str(dest),*map(str,files)];arg.write_text('\n'.join('"'+s.replace('\\','/').replace('"','\\"')+'"'for s in parts)+'\n',encoding='utf-8',newline='\n');return run([javac,'@'+str(arg)])
 version=run([java,'-version']);save(out/'toolchain.json',{'jdk':version.stderr,'java_sha256':sha(Path(java)),'javac_sha256':sha(Path(javac)),'jars':stack['jars'],'producer_sha256':sha(Path(__file__)),'BuildProvider_sha256':sha(HERE/'BuildProvider.java'),'FullRuntime_sha256':sha(HERE/'FullRuntime.java'),'input_pins_sha256':sha(HERE/'inputs/pins.json')})
 adapter=out/'adapter';v=compile([HERE/'BuildProvider.java',HERE/'FullRuntime.java',*[HERE/'inputs'/n for n in ['RuntimeOracle.java','MetamodelOracle.java','TerminalNegationOracle.java']]],cp,adapter)
 if v.returncode:print(v.stderr,flush=True);raise SystemExit(v.returncode)
 providers={}
 for c in json.loads((HERE/'inputs/cases.json').read_text(encoding='utf-8')):
  if args.only and c['id']not in args.only:continue
  print('START '+c['id'],flush=True);project=out/c['id'];project.mkdir();source=HERE/'inputs'/c['id'];row={'id':c['id'],'entry':c['entry'],'original_inputs':c['inputs'],'source_pins':{p.name:sha(p)for p in source.iterdir()if p.is_file()},'provider':None,'metadata':None,'generation':None,'compilation':None,'runtime':[]};rows.append(row);save(out/'observations.json',rows)
  metadata=project/'metadata';metadata.mkdir();shutil.copyfile(source/'imported.ecore',metadata/'imported.ecore')
  for s in c['sources']:shutil.copyfile(source/s['file'],metadata/s['file'])
  (metadata/'order.txt').write_text(''.join(s['grammar']+'\n'for s in c['sources']),encoding='utf-8');(metadata/'entry.txt').write_text(c['entry']+'\n',encoding='utf-8');v=run([java,'-cp',str(adapter)+os.pathsep+cp,'intersect.oracle.TerminalNegationOracle','metadata',str(metadata)]);data=[json.loads(x)for x in v.stdout.splitlines()if x.startswith('{')];row['metadata']={'exit_code':v.returncode,'observation':data[0]if len(data)==1 else None}
  h=sha(source/'imported.ecore')
  if h not in providers:
   provider=out/('provider-'+h[:12]);v=run([java,'-cp',str(adapter)+os.pathsep+cp,'BuildProvider',str(source/'imported.ecore'),str(provider)]);info={'ecore_sha256':h,'generation_exit':v.returncode,'package_class':None,'compile_exit':None,'genmodel':str(provider/'provider.genmodel')};providers[h]=(provider,info)
   if v.returncode==0:
    name=re.search(r'PACKAGE_CLASS=(.+)',v.stdout);info['package_class']=name.group(1).strip()if name else None;v=compile(sorted(provider.glob('src-gen/**/*.java')),cp,provider/'classes');info['compile_exit']=v.returncode
  provider,info=providers[h];row['provider']=info
  if info['generation_exit']or info['compile_exit']!=0:save(out/'observations.json',rows);continue
  src=project/'src';src.mkdir()
  for s in c['sources']:
   p=src/(s['grammar'].replace('.','/')+'.xtext');p.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source/s['file'],p);assert sha(p)==sha(source/s['file'])
  gm=provider/'provider.genmodel';template=(HERE/'inputs/Generate.mwe2.template').read_text(encoding='utf-8');template=template.replace('Workflow {','Workflow {\n  bean = org.eclipse.emf.mwe.utils.StandaloneSetup { registerGenModelFile = "'+gm.as_uri()+'" }');languages=''.join(('    language = XtextGeneratorLanguage {\n      name = "'+s['grammar']+'"\n      referencedResource = "'+gm.as_uri()+'"\n      fragment = org.eclipse.xtext.xtext.generator.grammarAccess.GrammarAccessFragment2 {}\n    }\n') if s['grammar'] != c['entry'] else ('    language = StandardLanguage {\n      name = "'+s['grammar']+'"\n      fileExtensions = "lexicaltest"\n      referencedResource = "'+gm.as_uri()+'"\n      serializer = { generateStub = false }\n      validator = { generateDeprecationValidation = false }\n    }\n') for s in c['sources']);workflow=src/'Generate.mwe2';workflow.write_text(template.replace('@LANGUAGES@',languages),encoding='utf-8',newline='\n');pcp=os.pathsep.join([str(provider/'classes'),str(provider/'src-gen'),str(src),cp]);v=run([java,'-cp',pcp,'org.eclipse.emf.mwe2.launch.runtime.Mwe2Launcher',workflow.as_uri(),'-p','rootPath='+project.as_posix()],cwd=project);row['generation']={'exit_code':v.returncode,'source_unchanged':all(sha(src/(s['grammar'].replace('.','/')+'.xtext'))==sha(source/s['file'])for s in c['sources'])};save(out/'observations.json',rows)
  if v.returncode:print('GENERATION_FAILED '+c['id'],flush=True);continue
  v=compile(sorted(project.glob('src/**/*.java'))+sorted(project.glob('src-gen/**/*.java')),pcp,project/'classes');row['compilation']={'exit_code':v.returncode};save(out/'observations.json',rows)
  if v.returncode:print('COMPILATION_FAILED '+c['id'],flush=True);continue
  runtime=os.pathsep.join([str(adapter),str(project/'classes'),str(project/'src-gen'),str(src),pcp])
  for text in c['inputs']:
   v=run([java,'-cp',runtime,'intersect.oracle.FullRuntime',info['package_class'],c['entry'],base64.b64encode(text.encode('utf-8')).decode('ascii')],timeout=15);values=[json.loads(x)for x in v.stdout.splitlines()if x.startswith('{')];row['runtime'].append({'text':text,'exit_code':v.returncode,'timeout_seconds':15 if v.returncode==124 else None,'observation':values[0]if len(values)==1 else None});save(out/'observations.json',rows)
  print('COMPLETE '+c['id']+' '+str(len(row['runtime'])),flush=True)
 save(out/'artifacts-sha256.json',{p.relative_to(out).as_posix():sha(p)for p in sorted(out.rglob('*'))if p.is_file()})
 print('COMPLETE_ALL '+str(len(rows)),flush=True)
if __name__=='__main__':main()
