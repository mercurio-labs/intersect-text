"""Replay complete attribute/containment/reference-spelling AST observations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--jdk',type=Path,required=True)
p.add_argument('--artifact-dir',type=Path,required=True)
p.add_argument('--maven-cache',type=Path,default=Path.home()/'.m2/repository')
p.add_argument('--record',action='store_true')
a=p.parse_args();f=Path(__file__).resolve().parent
pins=json.loads((f/'provenance.json').read_text(encoding='utf-8'))
def checked(path,digest):
    assert hashlib.sha256(path.read_bytes()).hexdigest()==digest,path
for item in pins['jars']:checked(a.maven_cache/item['path'],item['sha256'])
for name,digest in pins['files'].items():checked(f/name,digest)
if not a.record:
    for name,digest in json.loads((f/'tree-provenance.json').read_text())['files'].items():checked(f/name,digest)
out=a.artifact_dir.resolve();out.mkdir(parents=True,exist_ok=True)
suffix='.exe' if os.name=='nt' else ''
cp=os.pathsep.join(str(a.maven_cache/item['path']) for item in pins['jars'])
subprocess.run([str(a.jdk/'bin'/('javac'+suffix)),'-encoding','UTF-8','-cp',cp,'-d',str(out),str(f/'SyntaxOracle.java'),str(f/'TreeOracle.java')],check=True,timeout=60)
cases=json.loads((f/'cases.json').read_text())+json.loads((f/'coverage-cases.json').read_text())+[{'id':'structure','source':(f/'structure.xtext').read_text()}]
inputs=[]
for case in cases:
    path=out/(case['id']+'.xtext');path.write_text(case['source'],encoding='utf-8',newline='\n');inputs.append(str(path))
r=subprocess.run([str(a.jdk/'bin'/('java'+suffix)),'-cp',str(out)+os.pathsep+cp,'intersect.oracle.TreeOracle',*inputs],check=True,capture_output=True,text=True,encoding='utf-8',timeout=60)
rows=[json.loads(line) for line in r.stdout.splitlines() if line.startswith('{')]
assert len(rows)==len(cases)
if a.record:
    (f/'tree-observations.json').write_text(json.dumps(rows,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
    record={'scope':'Independent syntax attributes, containment and converted reference spellings; no reference resolution or runtime execution','files':{name:hashlib.sha256((f/name).read_bytes()).hexdigest() for name in ['TreeOracle.java','tree_oracle.py','tree-observations.json','SyntaxOracle.java','provenance.json','cases.json','coverage-cases.json','structure.xtext']}}
    (f/'tree-provenance.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8',newline='\n')
else:assert rows==json.loads((f/'tree-observations.json').read_text())
print('Verified',len(rows),'syntax tree observations;',sum(not r['syntax_errors'] for r in rows),'valid trees.')
