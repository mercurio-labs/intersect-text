"""Optional pinned ANTLR oracle, never used by native builds or production."""
import argparse, base64, hashlib, json, os, subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--jdk',type=Path,required=True);p.add_argument('--artifact-dir',type=Path,required=True);p.add_argument('--verify',action='store_true');a=p.parse_args()
f=Path(__file__).resolve().parent;out=a.artifact_dir.resolve();out.mkdir(parents=True,exist_ok=True)
if a.verify:
 for name,digest in json.loads((f/'provenance.json').read_text())['files'].items():
  assert hashlib.sha256((f/name).read_bytes()).hexdigest()==digest, name
parent=f.parent/'first-token-predicates/oracle-provenance.json'
pins=json.loads(parent.read_text())['jars'];jars=[]
for item in pins:
 if item['path'].startswith(('org/xtext/antlr-generator/','org/antlr/antlr-runtime/')):
  path=Path.home()/'.m2/repository'/item['path'];assert hashlib.sha256(path.read_bytes()).hexdigest()==item['sha256'];jars.append(str(path))
cp=os.pathsep.join(jars);suffix='.exe' if os.name=='nt' else ''
def run(cmd):
 r=subprocess.run(cmd,cwd=out,capture_output=True,text=True,encoding='utf-8',check=False,timeout=60)
 with (out/'diagnostics.txt').open('a',encoding='utf-8',newline='\n') as log:log.write(r.stderr)
 r.check_returncode()
 return r.stdout
rows=[]
rejected=[]
for index,case in enumerate(json.loads((f/'cases.json').read_text())):
 name='TerminalAudit'+str(index)
 grammar='lexer grammar '+name+';\n@members { public int errors=0; @Override public void reportError(RecognitionException e) { errors++; } }\nVALUE: '+case['body']+';\n'
 (out/(name+'.g')).write_text(grammar,encoding='utf-8',newline='\n')
 try:
  run([str(a.jdk/'bin'/('java'+suffix)),'-cp',cp,'org.antlr.Tool',str(out/(name+'.g'))])
 except subprocess.CalledProcessError as error:
  rejected.append(dict(body=case['body'],exit_code=error.returncode,diagnostic=error.stderr.replace(str(out),'<artifact-dir>').replace(chr(92),'/')))
  continue
 run([str(a.jdk/'bin'/('javac'+suffix)),'-encoding','UTF-8','-cp',cp,'-d',str(out),str(out/(name+'.java')),str(f/'TerminalOracle.java')])
 result=run([str(a.jdk/'bin'/('java'+suffix)),'-cp',str(out)+os.pathsep+cp,'TerminalOracle',name,*[base64.b64encode(t.encode()).decode() for t in case['inputs']]])
 for line in result.splitlines():
  i,accepted,errors=line.split('\t');rows.append(dict(body=case['body'],text=case['inputs'][int(i)],accepted=accepted=='true',errors=int(errors)))
if a.verify:
 assert rows==json.loads((f/'observations.json').read_text())
 assert rejected==json.loads((f/'generation-rejections.json').read_text())
else:
 (f/'generation-rejections.json').write_text(json.dumps(rejected,indent=2)+'\n',encoding='utf-8',newline='\n')
 (f/'observations.json').write_text(json.dumps(rows,indent=2)+'\n',encoding='utf-8',newline='\n')
 provenance=dict(scope='Authored ANTLR lexer probes, not full Xtext generation or pinned KerML execution',jars=[i for i in pins if i['path'].startswith(('org/xtext/antlr-generator/','org/antlr/antlr-runtime/'))],files={n:hashlib.sha256((f/n).read_bytes()).hexdigest() for n in ['cases.json','TerminalOracle.java','oracle.py','observations.json','generation-rejections.json','model.xtext','example.model','README.md']})
 (f/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n',encoding='utf-8',newline='\n')
print(f'{len(rows)} independent lexer observations verified')
