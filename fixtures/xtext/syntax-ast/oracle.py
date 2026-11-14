"""Optional exact-dependency Xtext syntax oracle; never used by production builds."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--jdk', type=Path, required=True)
parser.add_argument('--artifact-dir', type=Path, required=True)
parser.add_argument('--maven-cache', type=Path, default=Path.home()/'.m2/repository')
parser.add_argument('--record', action='store_true')
parser.add_argument('--coverage', action='store_true', help='Replay the additive node/lexical coverage matrix')
parser.add_argument('--structure', action='store_true', help='Replay the additive node-span fixture')
args = parser.parse_args()
fixture = Path(__file__).resolve().parent
if args.structure and args.coverage:
    parser.error('Choose one additive suite')
if args.structure and args.record:
    parser.error('--structure replays immutable recorded evidence; do not combine with --record')
provenance = json.loads((fixture/'provenance.json').read_text(encoding='utf-8'))
def checked(path, expected):
    raw = path.read_bytes()
    assert hashlib.sha256(raw).hexdigest() == expected, path
    return raw
jars = [str(args.maven_cache/item['path']) for item in provenance['jars']]
for item in provenance['jars']:
    checked(args.maven_cache/item['path'], item['sha256'])
if not args.record:
    for name, digest in provenance['files'].items():
        checked(fixture/name, digest)
out = args.artifact_dir.resolve()
out.mkdir(parents=True, exist_ok=True)
suffix = '.exe' if os.name == 'nt' else ''
cp = os.pathsep.join(jars)
subprocess.run([str(args.jdk/'bin'/('javac'+suffix)), '-encoding', 'UTF-8', '-cp', cp, '-d', str(out), str(fixture/'SyntaxOracle.java')], check=True, timeout=60)
inputs = []
cases = ([{'id':'structure','source':(fixture/'structure.xtext').read_text(encoding='utf-8')}] if args.structure else json.loads((fixture/('coverage-cases.json' if args.coverage else 'cases.json')).read_text(encoding='utf-8')))
if args.structure:
    for name,digest in json.loads((fixture/'structure-provenance.json').read_text(encoding='utf-8'))['files'].items():
        checked(fixture/name,digest)
if args.coverage and not args.record:
    for name,digest in json.loads((fixture/'coverage-provenance.json').read_text())['files'].items():
        checked(fixture/name,digest)
for case in cases:
    path = out/(case['id']+'.xtext')
    path.write_text(case['source'], encoding='utf-8', newline='\n')
    inputs.append(str(path))
result = subprocess.run([str(args.jdk/'bin'/('java'+suffix)), '-cp', str(out)+os.pathsep+cp, 'intersect.oracle.SyntaxOracle', *inputs], capture_output=True, text=True, encoding='utf-8', check=True, timeout=60)
rows = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
assert len(rows) == len(inputs)
if args.record and args.coverage:
    (fixture/'coverage-observations.json').write_text(json.dumps(rows,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
    record={'scope':'Independent node-class and lexical boundary observations; no runtime qualification','files':{name:hashlib.sha256((fixture/name).read_bytes()).hexdigest() for name in ['coverage-cases.json','coverage-observations.json','SyntaxOracle.java','provenance.json']}}
    (fixture/'coverage-provenance.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8',newline='\n')
elif args.record:
    (fixture/'observations.json').write_text(json.dumps(rows, ensure_ascii=True, indent=2)+'\n', encoding='utf-8', newline='\n')
    provenance['files'] = {name: hashlib.sha256((fixture/name).read_bytes()).hexdigest() for name in ['SyntaxOracle.java', 'oracle.py', 'cases.json', 'observations.json']}
    (fixture/'provenance.json').write_text(json.dumps(provenance, indent=2)+'\n', encoding='utf-8', newline='\n')
else:
    expected = ([json.loads((fixture/'structure-observation.json').read_text(encoding='utf-8'))] if args.structure else json.loads((fixture/('coverage-observations.json' if args.coverage else 'observations.json')).read_text(encoding='utf-8')))
    assert rows == expected
for row in rows:
    print(row['id'], row['syntax_errors'], json.dumps([n['attributes'].get('value') for n in row['nodes'] if n['class']=='Keyword']), json.dumps(row['errors']))
