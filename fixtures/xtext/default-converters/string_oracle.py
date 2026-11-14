"""Pinned default STRING converter, separate from lexer acceptance and model parsing."""
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
subprocess.run([str(a.jdk/'bin'/('javac'+suffix)),'-encoding','UTF-8','-cp',cp,'-d',str(out),str(f.parent/'syntax-ast/SyntaxOracle.java'),str(f/'StringOracle.java')],check=True,timeout=60)
cases=json.loads((f/'cases.json').read_text(encoding='utf-8'))
transport=out/'cases.utf16.txt'
transport.write_text(''.join(' '.join(f'{int.from_bytes(b[i:i+2],"little"):04x}' for i in range(0,len(b),2))+'\n' for b in [s.encode('utf-16-le') for s in cases]),encoding='ascii',newline='\n')
r=subprocess.run([str(a.jdk/'bin'/('java'+suffix)),'-cp',str(out)+os.pathsep+cp,'intersect.oracle.StringOracle',str(transport)],check=True,capture_output=True,text=True,encoding='utf-8',timeout=60)
rows=[json.loads(line) for line in r.stdout.splitlines() if line.startswith('{')]
assert len(rows)==len(cases)
assert [row['literal'] for row in rows]==cases, 'Oracle transport changed source literals'
if a.record:
 (f/'observations.json').write_text(json.dumps(rows,indent=2,ensure_ascii=True)+'\n',encoding='utf-8',newline='\n')
 record={'scope':'Xtext 2.38.0 STRINGValueConverter.toValue with null node on authored literal cases; converter values/errors only, not lexer acceptance or generated model execution','jars':pins,'files':{n:hashlib.sha256((f/n).read_bytes()).hexdigest() for n in ['../syntax-ast/SyntaxOracle.java','StringOracle.java','string_oracle.py','cases.json','observations.json']}}
 (f/'provenance.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8',newline='\n')
else: assert rows==json.loads((f/'observations.json').read_text(encoding='utf-8'))
print(f'Verified {len(rows)} independent STRING converter observations.')
