"""Pinned IParser syntax and inherited declaration selection, not full validation/execution."""
import argparse, hashlib, json, os, subprocess
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--jdk',type=Path,required=True);p.add_argument('--artifact-dir',type=Path,required=True);p.add_argument('--record',action='store_true');a=p.parse_args()
f=Path(__file__).resolve().parent
pins=json.loads((f.parent/'syntax-ast/provenance.json').read_text(encoding='utf-8'))['jars']
for item in pins:
 path=Path.home()/'.m2/repository'/item['path'];assert hashlib.sha256(path.read_bytes()).hexdigest()==item['sha256'],path
if not a.record:
 for name,digest in json.loads((f/'provenance.json').read_text(encoding='utf-8'))['files'].items():
  assert hashlib.sha256((f/name).read_bytes()).hexdigest()==digest,name
out=a.artifact_dir.resolve();out.mkdir(parents=True,exist_ok=True)
cp=os.pathsep.join(str(Path.home()/'.m2/repository'/i['path']) for i in pins)
suffix='.exe' if os.name=='nt' else ''
subprocess.run([str(a.jdk/'bin'/('javac'+suffix)),'-encoding','UTF-8','-cp',cp,'-d',str(out),str(f/'LibraryOracle.java')],check=True,timeout=60)
r=subprocess.run([str(a.jdk/'bin'/('java'+suffix)),'-cp',str(out)+os.pathsep+cp,'intersect.oracle.LibraryOracle',str(f)],check=True,capture_output=True,text=True,encoding='utf-8',timeout=60)
rows=[json.loads(line) for line in r.stdout.splitlines() if line.startswith('{')]
assert len(rows)==3
if a.record:
 (f/'observations.json').write_text(json.dumps(rows,indent=2)+'\n',encoding='utf-8',newline='\n')
 record={'scope':'Pinned Xtext 2.38.0 IParser syntax and GrammarUtil.findRuleForName inherited selection on authored sources, with explicit parent linking; not full validation, Ecore linking, generated runtime or semantic parity','jars':pins,'files':{n:hashlib.sha256((f/n).read_bytes()).hexdigest() for n in ['LibraryOracle.java','library_oracle.py','enum.xtext','datatype.xtext','fragment.xtext','model.xtext','observations.json']}}
 (f/'provenance.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8',newline='\n')
else: assert rows==json.loads((f/'observations.json').read_text(encoding='utf-8'))
print('Verified three independent declaration-library syntax/selection observations.')
