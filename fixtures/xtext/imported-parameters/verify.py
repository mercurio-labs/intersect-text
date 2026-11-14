"""Check/replay immutable imported-only Xtext evidence; Java is oracle-only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

sys.dont_write_bytecode = True
COMPONENTS = {'cases.json','observations.json','README.md','verify.py',
              'replay_metadata.py','replay_runtime.py',
              'OrdinaryParameterOracle.java','ImportedRuntimeOracle.java'}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def checked(path, digest):
    data = path.read_bytes()
    require(hashlib.sha256(data).hexdigest() == digest, 'SHA-256 mismatch: '+str(path))
    return data


def utf16_bytes(text):
    result = {0:0}
    units = size = 0
    for char in text:
        units += len(char.encode('utf-16-le'))//2
        size += len(char.encode('utf-8'))
        result[units] = size
    return result


def structural(value):
    if isinstance(value, dict):
        return {k:structural(v) for k,v in value.items() if k != 'origins'}
    if isinstance(value, list):
        return [structural(v) for v in value]
    return value


def counts(cases, rows):
    runs = rows['runs']
    return {'graphs':len(cases),'grammar_sources':sum(len(c['sources']) for c in cases),
            'metamodel_sources':sum(len(c['metamodels']) for c in cases),
            'metadata_accepted':sum(r['all_sources_accepted'] for r in rows['metadata']),
            'metadata_rejected':sum(not r['all_sources_accepted'] for r in rows['metadata']),
            'compiled_graphs':sum(r['stage']=='compiled' for r in rows['generation']),
            'registered_requests':len(runs),'rooted_successes':sum(r['accepted'] for r in runs),
            'error_free_rootless':sum(not r['accepted'] and not r.get('errors') and not r.get('exception') and r['model'] is None for r in runs),
            'registered_rejections':sum(not r['accepted'] and (bool(r.get('errors')) or bool(r.get('exception'))) for r in runs),
            'unregistered_context_requests':len(rows['unregistered_context'])}


def validate(cases, rows):
    ids = [c['id'] for c in cases]
    require(len(ids)==len(set(ids)) and all(re.fullmatch('[a-z][a-z0-9_]*',s) for s in ids), 'Invalid graph identities')
    for key in ['metadata','generation']:
        require([r['id'] for r in rows[key]] == ids, key+' graph inventory differs')
    expected_runs = [(c['id'],r['entry'],r['text']) for c in cases for r in c['runs']]
    require([(r['case'],r['entry'],r['text']) for r in rows['runs']] == expected_runs, 'Runtime request inventory differs')
    accepted = []
    classes_by_case = {}
    for case,meta,generated in zip(cases,rows['metadata'],rows['generation']):
        source_ids = [s['id'] for s in case['sources']]
        require(len(source_ids)==len(set(source_ids)) and case['entry'] in source_ids, 'Source entry/identity differs')
        require([r['id'] for r in meta['sources']] == source_ids, 'Metadata source inventory differs')
        by_grammar = {s['grammar']:s for s in case['sources']}
        require(len(by_grammar)==len(source_ids), 'Duplicate grammar identities')
        boundaries = {name:utf16_bytes(s['source']) for name,s in by_grammar.items()}
        declarations = {}
        for s in case['sources']:
            require(hashlib.sha256(s['source'].encode('utf-8')).hexdigest()==s['sha256'], 'Grammar source digest differs')
            require(s['uri']=='memory:/ordinary-parameters/'+case['id']+'/'+s['id']+'.xtext', 'Grammar producer URI differs')
            require(not re.search(r'\bgenerate\s+',s['source']), 'Ordinary grammar acquired generated metamodel authority')
            require(set(s['dependencies']) <= set(source_ids), 'Undeclared source dependency')
        namespaces = set()
        for model in case['metamodels']:
            require(re.fullmatch('[A-Za-z0-9_.-]+\.ecore',model['file']) is not None, 'Invalid metamodel file')
            require(hashlib.sha256(model['source'].encode('utf-8')).hexdigest()==model['sha256'], 'Metamodel source digest differs')
            require(model['uri']=='memory:/ordinary-parameters/'+case['id']+'/'+model['file'], 'Metamodel producer URI differs')
            require(ET.fromstring(model['source']).attrib['nsURI']==model['namespace'], 'Metamodel namespace differs')
            require(model['namespace'] not in namespaces, 'Ambiguous imported namespace')
            namespaces.add(model['namespace'])
        for source in meta['sources']:
            for rule in source['parameter_bindings']:
                for index,p in enumerate(rule['parameters']):
                    require(p['position']==index and p['rule']==rule['rule'] and p['grammar']==source['grammar'], 'Parameter declaration identity differs')
                    declarations[(p['grammar'],p['rule'],index)] = p

        def visit(value, grammar):
            if isinstance(value,list):
                for child in value: visit(child,grammar)
                return
            if not isinstance(value,dict): return
            if value.get('grammar') in by_grammar:
                grammar = value['grammar']
            if {'position','rule','uri','span'} <= set(value) and not value['proxy']:
                key = (value['grammar'],value['rule'],value['position'])
                require(declarations.get(key)==value, 'Resolved parameter binding differs from exact declaration position')
            if value.get('start') is not None:
                end = value.get('end')
                if end is None and value.get('length') is not None:
                    end = value['start']+value['length']
                if end is not None:
                    require(grammar in boundaries, 'Source range has no producer')
                    b = boundaries[grammar]
                    require(value['start'] in b and end in b and value['start']<=end, 'Invalid UTF-16 range or surrogate boundary')
            for child in value.values(): visit(child,grammar)

        for source in meta['sources']:
            visit(source,source['grammar'])
            errors = sum(d['severity']=='ERROR' for d in source['diagnostics'])
            require(source['error_count']==errors and source['accepted']==(errors==0), 'Incomplete metadata diagnostics')
        require(meta['all_sources_accepted']==all(s['accepted'] for s in meta['sources']), 'Graph metadata acceptance differs')
        require(structural(meta['imported_before'])==structural(meta['imported_after']) and meta['imported_structure_unchanged'], 'Imported structure mutated')
        require(meta['imported_metadata_unchanged']==(meta['imported_before']==meta['imported_after']), 'Imported metadata classification differs')
        classes_by_case[case['id']] = {t['identity']:t for p in meta['imported_before'] for t in p['classifiers'] if t['kind']=='EClass'}
        if not meta['all_sources_accepted']:
            require(generated['stage']=='metadata_rejected' and not case['runs'], 'Rejected metadata acquired parser/runtime qualification')
            continue
        accepted.append(case['id'])
        require(generated['stage']=='compiled' and generated['generation_exit']==0 and generated['compilation_exit']==0, 'Accepted graph lacks compiled parser')
        require(generated['source_unchanged'] and generated['generated_ecore_files']==[], 'Ordinary source/model generation boundary differs')
        require(set(generated['parser_entries'])==set(by_grammar), 'Parser entry inventory omits a source language')
        require(len(generated['registered_packages'])==len(case['metamodels']), 'Installed provider inventory differs')
        provider_hashes = {Path(k).name:v for k,v in generated['provider_ecore_sha256'].items()}
        require(provider_hashes=={m['file']:m['sha256'] for m in case['metamodels']}, 'Provider model differs from exact imported bytes')
    require([r['case'] for r in rows['unregistered_context']]==accepted, 'Missing-context probe inventory differs')
    by_case = {c['id']:c for c in cases}
    for row in rows['runs']+rows['unregistered_context']:
        case = by_case[row['case']]
        require(row['context'] in ['registered','unregistered'], 'Unknown provider context')
        entry_source = next(s for s in case['sources'] if s['id']==case['entry'])
        require(row['grammar']==entry_source['grammar'], 'Runtime grammar identity differs')
        success = row['model'] is not None and not row.get('errors') and not row.get('exception')
        require(row['accepted']==success, 'Runtime acceptance/root/error classification differs')
        if row['accepted'] and row['entry']=='resource':
            require(row['resource_contents']==1, 'Rooted resource count differs')
        if row['model'] is not None:
            require(isinstance(row['model'],list) and row['model'], 'Incomplete model projection')
            classes = classes_by_case[case['id']]
            def features(identity, active=None):
                active = set() if active is None else active
                require(identity in classes and identity not in active, 'Unknown or cyclic imported classifier identity')
                current = classes[identity]
                names = {f['name'] for f in current['features']}
                for parent in current['super_types']:
                    names |= features(parent,active|{identity})
                return names
            for obj in row['model']:
                require(set(obj['values'])==features(obj['class']), 'Incomplete model classifier/feature projection')
            def refs(value):
                if isinstance(value,list):
                    for child in value: refs(child)
                elif isinstance(value,dict):
                    if 'object' in value:
                        index = value['object']
                        require(isinstance(index,int) and 0<=index<len(row['model']), 'Invalid model object index')
                        require(row['model'][index]['class']==value['class'], 'Reference classifier identity differs')
                    for child in value.values(): refs(child)
            refs(row['model'])


def frozen(fixture):
    p = json.loads((fixture/'provenance.json').read_text(encoding='utf-8'))
    require(set(p['files'])==COMPONENTS, 'Fixture component inventory differs')
    for name,digest in p['files'].items(): checked(fixture/name,digest)
    for name,digest in p['dependencies'].items(): checked(fixture.parents[2]/name,digest)
    require(p['native_support'] is False, 'Independent observations acquired native support authority')
    cases = json.loads((fixture/'cases.json').read_text(encoding='utf-8'))['cases']
    rows = json.loads((fixture/'observations.json').read_text(encoding='utf-8'))
    validate(cases,rows)
    require(counts(cases,rows)==p['counts'], 'Evidence denominators differ')
    return p,cases,rows


def run(command, log):
    result = subprocess.run(command,capture_output=True,text=True,encoding='utf-8')
    log.write_text(result.stdout+result.stderr,encoding='utf-8',newline='\n')
    result.check_returncode()


def replay(fixture,p,cases,expected,jdk,cache,output):
    output = output.resolve()
    protected = fixture.parents[1]
    require(output!=protected and protected not in output.parents, 'Replay output must stay outside fixtures')
    output.mkdir(parents=True,exist_ok=False)
    for jar in p['jars']: checked(cache/jar['path'],jar['sha256'])
    checked(cache/p['source_archive']['path'],p['source_archive']['sha256'])
    run([sys.executable,'-B',str(fixture/'replay_metadata.py'),str(fixture/'cases.json'),str(output/'metadata'),str(jdk),str(cache)],output/'metadata-driver.log')
    metadata_path = output/'metadata/observations.json'
    metadata = json.loads(metadata_path.read_text(encoding='utf-8'))
    require(metadata==expected['metadata'], 'Independent metadata replay differs; expectations are read-only')
    run([sys.executable,'-B',str(fixture/'replay_runtime.py'),str(fixture/'cases.json'),str(metadata_path),str(output/'runtime'),str(jdk),str(cache)],output/'runtime-driver.log')
    ledger = json.loads((output/'runtime/capture-ledger.json').read_text(encoding='utf-8'))
    generation,runs,missing = [],[],[]
    for case,row in zip(cases,ledger['results']):
        require(row['id']==case['id'], 'Replayed graph order differs')
        if row['state']=='metadata_rejected':
            generation.append({'id':case['id'],'stage':'metadata_rejected'})
            continue
        require(row['state']=='captured', 'Unqualified replay failure: '+case['id'])
        project = output/'runtime/languages'/case['id']
        entries = {}
        for source in case['sources']:
            package,language = source['grammar'].rsplit('.',1)
            checked(project/'src'/Path(source['grammar'].replace('.','/')+'.xtext'),source['sha256'])
            parser = project/'src-gen'/package.replace('.','/')/'parser/antlr/internal'/('Internal'+language+'Parser.java')
            entries[source['grammar']] = sorted(set(re.findall(r'public final [^\n]+ entryRule([A-Za-z_0-9]+)\(',parser.read_text(encoding='utf-8'))))
        generation.append({'id':case['id'],'stage':'compiled','generation_exit':0,'compilation_exit':0,
                           'source_unchanged':True,'generated_ecore_files':[],'parser_entries':entries,
                           'provider_ecore_sha256':row['provider']['generation']['ecore_sha256'],
                           'registered_packages':row['provider']['registered_packages']})
        runs += [{'case':case['id'],**r} for r in row['runs']]
        missing.append({'case':case['id'],'context':'unregistered',**row['unregistered_context_probe']})
    actual = {'version':1,'metadata':metadata,'generation':generation,'runs':runs,'unregistered_context':missing}
    validate(cases,actual)
    (output/'observations-replay.json').write_text(json.dumps(actual,indent=2)+'\n',encoding='utf-8',newline='\n')
    require(actual==expected, 'Independent runtime replay differs; expectations are read-only')
    frozen(fixture)
    print(json.dumps({'state':'read_only_replay_passed','counts':counts(cases,actual),'native_support':False}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--replay',type=Path)
    parser.add_argument('--jdk',type=Path)
    parser.add_argument('--maven-cache',type=Path)
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    p,cases,rows = frozen(fixture)
    if args.replay:
        require(args.jdk is not None and args.maven_cache is not None, 'Replay needs explicit --jdk and --maven-cache')
        replay(fixture,p,cases,rows,args.jdk.resolve(),args.maven_cache.resolve(),args.replay)
    else:
        print(json.dumps({'state':'frozen_evidence_verified','counts':counts(cases,rows),'native_support':False}))


if __name__=='__main__':
    main()
