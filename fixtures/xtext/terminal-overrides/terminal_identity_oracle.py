"""Pinned Xtext token naming plus authored ANTLR lexer probes; not full Xtext generation."""
import argparse, hashlib, json, os, subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--jdk',type=Path,required=True);p.add_argument('--artifact-dir',type=Path,required=True);p.add_argument('--record',action='store_true');a=p.parse_args()
f=Path(__file__).resolve().parent
pins=json.loads((f.parent/'syntax-ast/provenance.json').read_text(encoding='utf8'))['jars']
for pin in pins:
 path=Path.home()/'.m2/repository'/pin['path'];assert hashlib.sha256(path.read_bytes()).hexdigest()==pin['sha256'],path
out=a.artifact_dir.resolve();out.mkdir(parents=True,exist_ok=True)
cp=os.pathsep.join(str(Path.home()/'.m2/repository'/pin['path']) for pin in pins)
java=str(a.jdk/'bin/java.exe');javac=str(a.jdk/'bin/javac.exe')
def run(command,check=True):
 r=subprocess.run(command,cwd=out,capture_output=True,text=True,encoding='utf8',timeout=60)
 if check:r.check_returncode()
 return r
run([javac,'-encoding','UTF-8','-cp',cp,'-d',str(out),str(f/'BindingOracle.java'),str(f/'TerminalIdentityOracle.java')])
runtime=str(out)+os.pathsep+cp
names=json.loads(run([java,'-cp',runtime,'intersect.oracle.TerminalIdentityOracle','names']).stdout)
assert names=={'child':'RULE_CHOICE','parent':'SUPER_Choice','order':['child','parent']},names
rows=[]
for index,(name,child,parent,inputs) in enumerate([
 ('identical',"'red'","'red'",['red']),
 ('disjoint',"'blue'","'red'",['red','blue']),
 ('partial-overlap',"'red'","'red' | 'blue'",['red','blue']),
]):
 grammar='TerminalIdentity'+str(index)
 text='lexer grammar '+grammar+';\n'+names['child']+': '+child+';\n'+names['parent']+': '+parent+';\n'
 path=out/(grammar+'.g');path.write_text(text,encoding='utf8',newline='\n')
 generated=run([java,'-cp',cp,'org.antlr.Tool',str(path)],check=False)
 row={'case':name,'grammar':text,'generation_exit_code':generated.returncode,'diagnostic':generated.stderr.replace(str(out),'<artifact-dir>').replace('\\','/'),'tokens':[]}
 if generated.returncode==0:
  run([javac,'-encoding','UTF-8','-cp',cp,'-d',str(out),str(out/(grammar+'.java'))])
  for text in inputs:
   observed=json.loads(run([java,'-cp',runtime,'intersect.oracle.TerminalIdentityOracle','lex',grammar,names['child'],names['parent'],text]).stdout)
   row['tokens'].append({'text':text,**observed})
 rows.append(row)
result={'names':names,'cases':rows}
files=['BindingOracle.java','TerminalIdentityOracle.java','terminal_identity_oracle.py','terminal-identity-observations.json']
if a.record:
 (f/'terminal-identity-observations.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf8',newline='\n')
 provenance={'scope':__doc__,'jdk':run([java,'-version']).stderr.splitlines()[0],'jars':pins,'files':{name:hashlib.sha256((f/name).read_bytes()).hexdigest() for name in files}}
 (f/'terminal-identity-provenance.json').write_text(json.dumps(provenance,indent=2)+'\n',encoding='utf8',newline='\n')
else:
 for name,digest in json.loads((f/'terminal-identity-provenance.json').read_text(encoding='utf8'))['files'].items():assert hashlib.sha256((f/name).read_bytes()).hexdigest()==digest,name
 assert result==json.loads((f/'terminal-identity-observations.json').read_text(encoding='utf8')),result
print(json.dumps(result,indent=2))
