"""Capture exact imported-only contexts with the pinned independent JVM oracle."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = Path(__file__).resolve().parent
plan_path = Path(sys.argv[1]).resolve()
output = Path(sys.argv[2]).resolve()
plan = json.loads(plan_path.read_text(encoding='utf-8'))
protected = ROOT/'fixtures'
if output == protected or protected in output.parents:
    raise ValueError('Replay output must stay outside fixtures')
output.mkdir(parents=True, exist_ok=False)
jdk, cache = Path(sys.argv[3]).resolve(), Path(sys.argv[4]).resolve()
suffix = '.exe' if os.name == 'nt' else ''
p = json.loads((ROOT/'fixtures/xtext/inference-fragment-runtime/provenance.json').read_text(encoding='utf-8'))
jars = []
for item in p['jars']:
    jar = cache/item['path']
    assert hashlib.sha256(jar.read_bytes()).hexdigest() == item['sha256'], jar
    jars.append(str(jar))
cp = os.pathsep.join(jars)
classes = output/'classes'
classes.mkdir()
sources = [ROOT/'fixtures/xtext/metamodel-inference/MetamodelOracle.java',
           ROOT/'fixtures/xtext/inference-fragments/FragmentOracle.java',
           EVIDENCE/'OrdinaryParameterOracle.java']
command = [str(jdk/'bin'/('javac'+suffix)), '-encoding', 'UTF-8', '--release', '17',
           '-cp', cp, '-d', str(classes), *map(str, sources)]
result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8', timeout=180)
(output/'compilation.log').write_text(result.stdout+result.stderr, encoding='utf-8', newline='\n')
result.check_returncode()
inputs = {}
for case in plan['cases']:
    directory = output/case['id']
    directory.mkdir()
    for source in case['sources']:
        f = directory/(source['id']+'.xtext')
        f.write_text(source['source'], encoding='utf-8', newline='\n')
        assert hashlib.sha256(f.read_bytes()).hexdigest() == source['sha256']
        inputs[f.relative_to(output).as_posix()] = source['sha256']
    for model in case['metamodels']:
        f = directory/model['file']
        f.write_text(model['source'], encoding='utf-8', newline='\n')
        assert hashlib.sha256(f.read_bytes()).hexdigest() == model['sha256']
        inputs[f.relative_to(output).as_posix()] = model['sha256']
    (directory/'models.txt').write_text(''.join(m['file']+'\n' for m in case['metamodels']), encoding='utf-8', newline='\n')
    (directory/'order.txt').write_text(''.join(s['id']+'\n' for s in case['sources']), encoding='utf-8', newline='\n')
command = [str(jdk/'bin'/('java'+suffix)), '-cp', os.pathsep.join([str(classes), cp]),
           'intersect.oracle.OrdinaryParameterOracle', str(output), *[c['id'] for c in plan['cases']]]
result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8', timeout=180)
(output/'metadata.log').write_text(result.stdout+result.stderr, encoding='utf-8', newline='\n')
result.check_returncode()
rows = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
assert [r['id'] for r in rows] == [c['id'] for c in plan['cases']]
for path, digest in inputs.items():
    assert hashlib.sha256((output/path).read_bytes()).hexdigest() == digest, path
(output/'observations.json').write_text(json.dumps(rows, indent=2)+'\n', encoding='utf-8', newline='\n')
receipt = {'version':1, 'native_support':False, 'state':'exploratory_independent_capture',
           'plan_sha256':hashlib.sha256(plan_path.read_bytes()).hexdigest(),
           'range_units':'UTF-16 code units; source digests cover exact UTF-8 bytes',
           'input_hashes_unchanged':inputs,
           'adapter_hashes':{str(s):hashlib.sha256(s.read_bytes()).hexdigest() for s in sources},
           'graphs':len(rows), 'accepted':sum(r['all_sources_accepted'] for r in rows),
           'rejected':sum(not r['all_sources_accepted'] for r in rows),
           'structural_mutations':[r['id'] for r in rows if not r['imported_structure_unchanged']],
           'author_expectation_mismatches':[c['id'] for c,r in zip(plan['cases'],rows) if c['author_expected_metadata_acceptance'] != r['all_sources_accepted']]}
(output/'receipt.json').write_text(json.dumps(receipt, indent=2)+'\n', encoding='utf-8', newline='\n')
print({k:v for k,v in receipt.items() if k not in ['input_hashes_unchanged','adapter_hashes']})
