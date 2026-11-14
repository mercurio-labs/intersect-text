#!/usr/bin/env python3
"""Portable contextual-entry corpus integrity and explicit optional replays."""
import argparse, hashlib, json, os, pathlib, subprocess, sys
ROOT=pathlib.Path(__file__).resolve().parent

def extended(p):
    p=pathlib.Path(p)
    return pathlib.Path('\\\\?\\'+str(p)) if os.name=='nt' and not str(p).startswith('\\\\?\\') else p

def raw(rel):return extended(ROOT/rel).read_bytes()
def load(rel):return json.loads(raw(rel).decode('utf8'))
def sha(b):return hashlib.sha256(b).hexdigest()
def equal(a,b,message):
    if a!=b:raise AssertionError(message)
def owned(rel):
    p=pathlib.PurePosixPath(rel)
    if p.is_absolute()or'..'in p.parts or ':'in rel or '\\'in rel:raise AssertionError('Unsafe portable path '+rel)
    return ROOT.joinpath(*p.parts)
def source(s):equal(sha(s['text'].encode('utf8')),s['id']['sha256'],'Source digest '+s['id']['uri'])
def paired(v):
    if isinstance(v,dict):
        if 'request'in v and'result'in v:return[(v['request'],v['result'])]
        return[p for child in v.values()for p in paired(child)]
    if isinstance(v,list):return[p for child in v for p in paired(child)]
    return[]
def verify_native():
    provenance=load('native-provenance.json')
    for rel,pin in provenance['files'].items():
        data=extended(owned(rel)).read_bytes();equal(len(data),pin['bytes'],'Size '+rel);equal(sha(data),pin['sha256'],'SHA '+rel)
    original=load('native/raw/native-cases.original.json');cases=load('native/native-cases.json')['cases'];inputs=load('inputs/cases.json')
    equal(len(cases),20,'Twenty unchanged source graphs');equal(sum(len(c['inputs'])for c in cases),128,'128 planned inputs')
    for c in cases:
        old=next(x for x in original['cases']if x['id']==c['id']);equal(c['compilation'],old['compilation'],'Metadata-only adaptation '+c['id'])
        oracle=next(x for x in inputs if x['id']==c['id']);equal(c['inputs'],oracle['inputs'],'Original probes '+c['id'])
        for s in c['compilation']['bundle']['sources']:source(s)
        for s in oracle['sources']:
            data=raw('inputs/'+c['id']+'/'+s['file']);equal(sha(data),s['source_id']['sha256'],'Original Xtext bytes '+c['id'])
            native=next(x for x in c['compilation']['bundle']['sources']if x['id']==s['source_id']);equal(native['text'].encode('utf8'),data,'Original source text '+c['id'])
        data=extended(owned(c['original_ecore']['path'])).read_bytes();equal(sha(data),c['original_ecore']['sha256'],'Original provider '+c['id'])
    before=load('native/before-v60.json');after=load('native/after-v61.json');compat=load('native/inferred-before-v60.json')
    old=paired(before);equal(len(old),240,'Complete old transcript');equal([(r['request'],r['result'])for r in after['transcript'][:240]],old,'Every old V60 complete response')
    equal([(r['request'],r['result'])for r in after['transcript'][240:244]],paired(compat),'Every inferred response');equal(len(after['transcript']),394,'Complete candidate transcript')
    equal(sum(c['compile']['result'].get('ok',False)for c in before['cases']),12,'Old compile successes')
    equal(sum(c['compile'].get('ok',False)for c in after['cases']),6,'New compile successes')
    for c in after['cases']:
        original_case=next(x for x in cases if x['id']==c['id']);equal([x['text']for x in c['inputs']],original_case['inputs'],'Every planned slot retained '+c['id'])
        if c['compile'].get('ok'):
            equal(c['compile']['package']['compilation'],original_case['compilation'],'Original compilation authority '+c['id']);equal(c['archive_import']['package'],c['compile']['package'],'Archive rebuild '+c['id'])
            equal(c['project']['definitionRecovery']['compilation'],original_case['compilation'],'Recovery source '+c['id'])
        else:
            if not all('not_executed'in i for i in c['inputs']):raise AssertionError('Rejected-source slots relabelled as runtime '+c['id'])
    migration=load('migration/identity-report.json');report=migration['result']['report'];equal(migration['result']['ok'],True,'Actual migration');equal(report['issues'],[],'Complete migration issues');equal(len(report['correspondence']),14,'All correspondence retained')
    equal(report['original']['text'].encode('utf8'),raw('migration/original.ecore'),'Original Ecore unchanged');equal(report['converted']['text'].encode('utf8'),raw('migration/converted-source.rcore'),'Converted Rcore unchanged');equal(report['draft'].encode('utf8'),raw('migration/original-draft.rcore'),'Original draft unchanged');source(report['original']);source(report['converted'])
    selected=load('migration/selection.json')
    for id in selected['graphs']:
        c=next(x for x in cases if x['id']==id);s=next(s for s in c['compilation']['bundle']['sources']if s['id']==c['compilation']['ecore']);equal(s,report['converted'],'Selected actual complete migration output '+id);equal(raw('inputs/'+id+'/imported.ecore'),raw('migration/original.ecore'),'New provider bytes '+id)
    parameters=load('native/parameter-cases.json')['cases'];original_parameters=load('native/raw/parameter-cases.original.json')['cases']
    for c in parameters:
        equal(c['compilation'],next(x for x in original_parameters if x['id']==c['id'])['compilation'],'Parameter metadata-only adaptation');[source(s)for s in c['compilation']['bundle']['sources']]
    gap=next(c for c in load('native/parameter-before-v60.json')['cases']if c['id']=='identity_parameter_before');equal(gap['result']['ok'],False,'Prior identity parameter failure preserved');equal(gap['result'],gap['replay'],'Original complete fresh replay')
    upstream=load('oracle/record/observations.json');overlap=0;accepted_models=0;unexecuted=0
    for c in after['cases']:
        if not c['compile'].get('ok'):continue
        oracle=next(x for x in upstream if x['id']==c['id']);package=c['compile']['package']
        for i in c['inputs']:
            observed=next((x for x in oracle['runtime']if x['text']==i['text']),None)
            if observed is None:unexecuted+=1;continue
            equal(observed['exit_code'],0,'Original runtime process '+c['id']);resource=observed['observation']['resource'];native=i['result'];accepted=not native['diagnostics']
            equal(accepted,resource['accepted'],'Actual resource acceptance '+c['id']);overlap+=1
            if accepted:
                projected=[]
                for e in native['model']['elements']:
                    d=package['schema']['declarations'][e['class']]
                    projected.append({'class':d['namespace']+'#//'+d['name'],'values':e['values']})
                    equal(e['start'],0,'Native source start');equal(e['end'],len(i['text'].encode('utf8')),'Native source byte end')
                equal(projected,resource['model'],'Complete accepted object classes/features '+c['id']);accepted_models+=1
    equal((overlap,accepted_models,unexecuted),(29,7,15),'Actual overlap is separate from unexecuted requests')
    return{'graphs':20,'planned_inputs':128,'v60_native_compiles':12,'v61_native_compiles':6,'original_complete_responses':244,'candidate_complete_requests':394,'actual_upstream_resource_overlap':29,'actual_accepted_full_object_models':7,'native_inputs_with_upstream_unexecuted':15,'native_ecore_award':False}

def main():
    p=argparse.ArgumentParser();p.add_argument('--wasm',type=pathlib.Path);p.add_argument('--replay',type=pathlib.Path);p.add_argument('--oracle-replay',type=pathlib.Path);p.add_argument('--jdk',type=pathlib.Path);p.add_argument('--maven-cache',type=pathlib.Path);args=p.parse_args()
    oracle=[sys.executable,str(ROOT/'oracle-producer/verify.py')]
    if args.oracle_replay:
        if not args.jdk or not args.maven_cache:p.error('--oracle-replay requires explicit --jdk and --maven-cache')
        oracle+=['--replay',str(args.oracle_replay),'--jdk',str(args.jdk),'--maven-cache',str(args.maven_cache)]
    subprocess.run(oracle,check=True)
    result=verify_native()
    if args.replay:
        if not args.wasm:p.error('--replay requires explicit --wasm; historical recorded artifact paths are not executable authority')
        subprocess.run(['node',str(ROOT/'native/replay.mjs'),'--wasm',str(args.wasm.resolve()),'--output',str(args.replay.resolve())],check=True)
    print(json.dumps(result))
if __name__=='__main__':main()
