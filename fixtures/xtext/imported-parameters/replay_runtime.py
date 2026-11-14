"""Fresh independent providers and ordinary imported-language parsers/models."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import subprocess
import sys
sys.dont_write_bytecode = True
import traceback
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[3]
EVIDENCE = Path(__file__).resolve().parent
plan_path, metadata_path, output = map(lambda s:Path(s).resolve(), sys.argv[1:4])
plan = json.loads(plan_path.read_text(encoding='utf-8'))
metadata = json.loads(metadata_path.read_text(encoding='utf-8'))
assert [c['id'] for c in plan['cases']] == [r['id'] for r in metadata]
protected = ROOT/'fixtures'
if output == protected or protected in output.parents:
    raise ValueError('Replay output must stay outside fixtures')
output.mkdir(parents=True, exist_ok=False)
fixture = ROOT/'fixtures/xtext/inference-fragment-runtime'
spec = importlib.util.spec_from_file_location('ordinary_provider_oracle', fixture/'verify.py')
oracle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(oracle)
provenance, _, _ = oracle.frozen(fixture)
jdk, cache = Path(sys.argv[4]).resolve(), Path(sys.argv[5]).resolve()
suffix = '.exe' if os.name == 'nt' else ''
jars = []
for item in provenance['jars']:
    path = cache/item['path']
    oracle.checked(path,item['sha256'])
    jars.append(str(path))
cp = os.pathsep.join(jars)


def run(command, log, cwd=None):
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True,
                            encoding='utf-8', timeout=180)
    log.write_text(result.stdout+result.stderr, encoding='utf-8', newline='\n')
    return result


def compile_java(files, classpath, classes, directory):
    classes.mkdir()
    args = ['-encoding','UTF-8','--release','17','-cp',classpath,'-d',str(classes),*map(str,files)]
    argfile = directory/'javac.args'
    argfile.write_text('\n'.join('"'+a.replace('\\','/').replace('"','\\"')+'"' for a in args)+'\n', encoding='utf-8', newline='\n')
    result = run([str(jdk/'bin'/('javac'+suffix)),'@'+str(argfile)],directory/'compilation.log')
    result.check_returncode()


adapter = output/'adapter'
adapter.mkdir()
compile_java([ROOT/'fixtures/xtext/metamodel-inference/MetamodelOracle.java',
              fixture/'RuntimeOracle.java',
              EVIDENCE/'ImportedRuntimeOracle.java'],
             cp, adapter/'classes', adapter)
ledger = {'version':1, 'native_support':False, 'state':'exploratory_independent_capture',
          'plan_sha256':hashlib.sha256(plan_path.read_bytes()).hexdigest(),
          'metadata_sha256':hashlib.sha256(metadata_path.read_bytes()).hexdigest(),
          'scope':'Fresh pinned JVM model providers, ordinary import-only grammar generation and complete runtime observations; production never needs Java.',
          'results':[]}


def save():
    (output/'capture-ledger.json').write_text(json.dumps(ledger,indent=2)+'\n', encoding='utf-8', newline='\n')


for case, observed in zip(plan['cases'],metadata):
    print('START '+case['id'],flush=True)
    row = {'id':case['id'], 'state':'pending', 'planned_runs':case['runs'], 'runs':[]}
    ledger['results'].append(row)
    save()
    if not observed['all_sources_accepted']:
        row.update(state='metadata_rejected',generation=None)
        save()
        print('METADATA_REJECTED '+case['id'],flush=True)
        continue
    try:
        original = copy.deepcopy(case['oracle_model_provider']['scaffold_case'])
        original['runs'] = []
        provider_root = output/'providers'/case['id']
        captured = oracle.replay(fixture,provenance,[original],jdk,cache,provider_root)
        assert captured['generation'][0]['stage']=='compiled',captured['generation'][0]
        assert captured['generation'][0]['ecore_sha256']==case['oracle_model_provider']['scaffold_ecore_hashes']
        provider = provider_root/original['id']
        model_paths = []
        package_classes = []
        for genmodel in sorted((provider/'model/generated').glob('*.genmodel')):
            model_paths.append(genmodel)
            for package in ET.parse(genmodel).getroot().findall('genPackages'):
                ecore = genmodel.parent/package.attrib['ecorePackage'].split('#')[0]
                namespace = ET.parse(ecore).getroot().attrib['nsURI']
                imported = next(m for m in case['metamodels'] if m['namespace']==namespace)
                assert hashlib.sha256(ecore.read_bytes()).hexdigest()==imported['sha256']
                name = ET.parse(ecore).getroot().attrib['name']
                classname = package.attrib['basePackage']+'.'+name+'.'+package.attrib['prefix']+'Package'
                assert (provider/'src-gen'/Path(classname.replace('.','/')+'.java')).exists(),classname
                package_classes.append(classname)
        assert len(package_classes)==len(case['metamodels'])
        row['provider']={'generation':captured['generation'][0], 'registered_packages':package_classes,
                         'ecore_hashes_match_exact_imported_inputs':True}
        print('PROVIDER_READY '+case['id'],flush=True)
        save()
        project = output/'languages'/case['id']
        project.mkdir(parents=True)
        source_files = []
        for source in case['sources']:
            path = project/'src'/Path(source['grammar'].replace('.','/')+'.xtext')
            path.parent.mkdir(parents=True,exist_ok=True)
            path.write_text(source['source'],encoding='utf-8',newline='\n')
            source_files.append((path,source['sha256']))
        template = (fixture/'Generate.mwe2.template').read_text(encoding='utf-8')
        registrations = ''.join(' registerGenModelFile = "'+p.as_uri()+'"' for p in model_paths)
        template = template.replace('Workflow {','Workflow {\n  bean = org.eclipse.emf.mwe.utils.StandaloneSetup {'+registrations+' }')
        references = ''.join('      referencedResource = "'+p.as_uri()+'"\n' for p in model_paths)
        languages = ''.join('    language = StandardLanguage {\n      name = "'+s['grammar']+'"\n      fileExtensions = "fragmenttest"\n'+references+'      serializer = { generateStub = false }\n      validator = { generateDeprecationValidation = false }\n    }\n' for s in case['sources'])
        workflow = project/'src/Generate.mwe2'
        workflow.write_text(template.replace('@LANGUAGES@',languages),encoding='utf-8',newline='\n')
        provider_cp = os.pathsep.join([str(provider/'classes'),str(provider/'src-gen'),str(provider/'src'),str(project/'src'),cp])
        result = run([str(jdk/'bin'/('java'+suffix)),'-cp',provider_cp,'org.eclipse.emf.mwe2.launch.runtime.Mwe2Launcher',workflow.as_uri(),'-p','rootPath='+project.as_posix()],project/'generation.log',project)
        row['generation']={'exit_code':result.returncode}
        save()
        result.check_returncode()
        for path,h in source_files:
            assert hashlib.sha256(path.read_bytes()).hexdigest()==h,path
        assert not list((project/'model/generated').glob('*.ecore'))
        compile_java(sorted(project.glob('src/**/*.java'))+sorted(project.glob('src-gen/**/*.java')),
                     provider_cp,project/'classes',project)
        row['generation'].update(compilation_exit=0,source_unchanged=True,generated_ecore_files=[])
        row['state']='compiled'
        save()
        runtime_cp = os.pathsep.join([str(adapter/'classes'),str(project/'classes'),str(project/'src-gen'),str(project/'src'),provider_cp])
        entry = next(s['grammar'] for s in case['sources'] if s['id']==case['entry'])
        for index,request in enumerate(case['runs']):
            command = [str(jdk/'bin'/('java'+suffix)),'-cp',runtime_cp,'intersect.oracle.ImportedRuntimeOracle',','.join(package_classes),entry,request['entry'],request['text']]
            result = run(command,project/('runtime-'+str(index)+'.log'))
            result.check_returncode()
            observations = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
            assert len(observations)==1
            row['runs'].append({'context':'registered',**observations[0]})
            save()
        first = next((r for r in case['runs'] if r['entry']=='resource'),None)
        if first:
            command = [str(jdk/'bin'/('java'+suffix)),'-cp',runtime_cp,'intersect.oracle.RuntimeOracle',entry,'resource',first['text']]
            result = run(command,project/'runtime-unregistered.log')
            result.check_returncode()
            observations = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
            assert len(observations)==1
            row['unregistered_context_probe']=observations[0]
        row['state']='captured'
        save()
        print('CAPTURED '+case['id']+' runs='+str(len(row['runs'])),flush=True)
    except Exception as error:
        row.update(state='capture_failed_unqualified',error_type=type(error).__name__,error=str(error),traceback=traceback.format_exc())
        save()
        print('CAPTURE_FAILED '+case['id']+' '+type(error).__name__+': '+str(error),flush=True)
print('COMPLETE '+json.dumps({state:sum(r['state']==state for r in ledger['results']) for state in sorted({r['state'] for r in ledger['results']})}),flush=True)
if any(r['state']=='capture_failed_unqualified' for r in ledger['results']):
    raise SystemExit(1)
