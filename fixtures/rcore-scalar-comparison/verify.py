"""Portable raw preservation plus source-bound full transcript derivations.

Only the local object store is opened. Historical absolute origin paths are data.
No build, native/WASM execution or language algorithm is implemented here.
"""
from pathlib import Path
import gzip,hashlib,json,math,re
ROOT=Path(__file__).resolve().parent

def require(value,message):
    if not value:raise AssertionError(message)
def exact(a,b):
    if type(a)is not type(b):return False
    if isinstance(a,dict):return a.keys()==b.keys()and all(exact(a[k],b[k])for k in a)
    if isinstance(a,list):return len(a)==len(b)and all(exact(x,y)for x,y in zip(a,b))
    return a==b
def same(a,b,message):require(exact(a,b),message)
def sha(data):return hashlib.sha256(data).hexdigest()
def request_hash(value):
    # Frozen requests contain strings, booleans, null and safe integer numbers.
    # Validate that scope rather than silently approximating JS Number encoding.
    def check(v):
        if isinstance(v,dict):
            require(all(isinstance(k,str)for k in v),'non-string request key')
            for x in v.values():check(x)
        elif isinstance(v,list):
            for x in v:check(x)
        elif type(v)is int:require(abs(v)<=9007199254740991,'request integer outside exact JS range')
        else:require(v is None or type(v)in(str,bool),'unsupported request numeric encoding')
    check(value)
    # backslashreplace preserves any lone UTF16 JSON observation spelling;
    # ordinary non-ASCII/astral strings retain their exact UTF8 representation.
    return sha(json.dumps(value,ensure_ascii=False,separators=(',',':')).encode('utf-8','backslashreplace'))
def source_hashes(request):
    count=0
    for source in request.get('bundle',{}).get('sources',[]):
        require(sha(source['text'].encode('utf-8'))==source['id']['sha256'],'authored source SHA mismatch')
        require(isinstance(source['id']['uri'],str)and bool(source['id']['uri']),'missing authored source URI')
        count+=1
    return count

class Store:
    def __init__(self):
        self.manifest=json.loads((ROOT/'provenance.json').read_bytes());self.cache={};self.parsed={}
        for key,entry in self.manifest['entries'].items():
            digest=entry['sha256']
            require(re.fullmatch('[0-9a-f]{64}',digest)is not None,'invalid object digest')
            if digest not in self.cache:
                packed=(ROOT/'objects'/(digest+'.gz')).read_bytes()
                require(packed[3]==0 and packed[4:8]==bytes(4),'non-deterministic gzip metadata')
                self.cache[digest]=gzip.decompress(packed)
            raw=self.cache[digest]
            require(len(raw)==entry['bytes']and sha(raw)==digest,'raw object integrity: '+key)
    def raw(self,key):return self.cache[self.manifest['entries'][key]['sha256']]
    def read(self,key):
        if key not in self.parsed:self.parsed[key]=json.loads(self.raw(key))
        return self.parsed[key]

class Transcript:
    def __init__(self,rows):self.rows=rows;self.at=0;self.source_checks=0
    def take(self,request):
        self.source_checks+=source_hashes(request)
        require(self.at<len(self.rows),'truncated request transcript')
        row=self.rows[self.at];self.at+=1
        require(row['request_sha256']==request_hash(request),'request hash differs at row '+str(self.at-1))
        return row['response']
    def finish(self):require(self.at==len(self.rows),'unconsumed request transcript rows')

UNKNOWN={'error':'Unknown, stale or foreign function job'}

def verify_public(store):
    keys=['before-main','before-unit','before-unicode'];current=store.read('current-public/native-0.json')
    require(len(current)==3,'public suite inventory changed')
    lookup={};hashes=0;source_checks=0;outcomes=0
    for si,key in enumerate(keys):
        cases=store.read(key+'/requests.json');before=store.read(key+'/native-0.json');capture=current[si]
        require(len(cases)==[152,8,4][si],'original/supplement case inventory changed')
        require(len(cases)==len(before['outputs'])==len(capture['outputs']),'public output count')
        t=Transcript(capture['rows']);same(t.take({'method':'rcore_behavior/jobHost','namespace':'scalar-equality-before'}),before['rows'][0]['response'],'host initialization')
        sequence=0
        for i,case in enumerate(cases):
            require(case['id']not in lookup,'ambiguous original case ID');lookup[case['id']]={'request':case['request'],'outcome':before['outputs'][i]['outcome']}
            require(case['id']==before['outputs'][i]['id']==capture['outputs'][i]['id'],'source/case outcome association')
            require(request_hash(case['request'])==before['rows'][before['outputs'][i]['first_row']]['request_sha256'],'original request hash')
            first=t.at;result=t.take(case['request'])
            if result.get('pending'):
                sequence+=1;job='scalar-equality-before:'+str(sequence)
                same(result,{'job':job,'pending':True},'deterministic original namespace/handle')
                for _ in range(100000):
                    result=t.take({'method':'rcore_behavior/resumeFunction','job':job,'quantum':4096})
                    if not result.get('pending'):break
                    same(result,{'job':job,'pending':True},'public Pending envelope')
                require(result.get('pending')is False,'public completion missing')
                same(t.take({'method':'rcore_behavior/resumeFunction','job':job,'quantum':1}),UNKNOWN,'finished handle remains live')
                result=result['result']
            same(result,before['outputs'][i]['outcome'],'whole original outcome '+case['id'])
            same(capture['outputs'][i],{'index':i,'id':case['id'],'first_row':first,'row_count':t.at-first,'outcome':result},'complete public output/row association')
            outcomes+=1
        t.finish();hashes+=t.at;source_checks+=t.source_checks
    for mode in ['native','wasm']:
        for replay in [0,1]:require(store.raw(f'current-public/{mode}-{replay}.json')==store.raw('current-public/native-0.json'),'whole public fresh captures differ')
    require(hashes==525 and outcomes==164,'public source/transcript inventory')
    return lookup,hashes,source_checks

def verify_points(store,lookup):
    plan=store.read('lifecycle/plan.json');same(plan,store.read('lifecycle-v1/plan.json'),'v1/v2 lifecycle plan was rebased')
    same(plan['actions'],['cancel','drop'],'lifecycle actions');require(plan['quantum']==256 and len(plan['points'])==18,'phase schedule inventory')
    role_sets={};phase_data={}
    for point in plan['points']:
        original=lookup[point['id']]
        same(point['request'],original['request'],'point request not exact original')
        same(point['complete'],original['outcome'],'point complete outcome not exact original')
        require(point['point']in ['first','middle','last'],'unknown comparison role')
        role_sets.setdefault(point['id'],[]).append(point['point'])
        if point['id']not in phase_data:
            rows=store.read('current-public/'+point['id']+'-q256-0.stdout')
            same(rows,store.read('current-public/'+point['id']+'-q256-1.stdout'),'q256 fresh phase observations differ')
            for i,row in enumerate(rows):require(row['resumes']==i+1,'phase resume numbering')
            comparison=[r for r in rows if isinstance(r['numeric_phase'],list)and r['numeric_phase'][0]=='kernel'and isinstance(r['numeric_phase'][1],list)and r['numeric_phase'][1][0]=='compare-text']
            require(len(comparison)>=3,'missing three genuine comparison Pending boundaries')
            for row in comparison:
                same(row['progress'],{'pending':True},'completed comparison labeled Pending')
                phase=row['numeric_phase'][1];require(0<phase[1]<phase[2],'comparison progress range')
            same(rows[-1]['progress']['complete'],{'Ok':original['outcome']['evaluation']},'whole typed source/SomeRegistry completion')
            phase_data[point['id']]=(rows,comparison)
        rows,comparison=phase_data[point['id']]
        expected={'first':comparison[0],'middle':comparison[len(comparison)//2],'last':comparison[-1]}[point['point']]
        same(point['witness'],expected,'point role does not select actual comparison boundary')
        require(point['prefix']==expected['resumes'],'selected prefix not actual phase resume')
        same(rows[point['prefix']-1],point['witness'],'selected prefix observation')
    require(len(role_sets)==6,'six long source cases required')
    for roles in role_sets.values():same(roles,['first','middle','last'],'duplicate/missing ordered phase role')
    return plan

def verify_recovery(store,plan):
    recovery=store.read('recovery/terminal.json');same(plan['recovery'],recovery['request'],'original recovery input changed');source_hashes(plan['recovery'])
    require(len(plan['recovery']['bundle']['sources'])==1,'unexpected recovery graph')
    source=plan['recovery']['bundle']['sources'][0]
    require(source['text']=='rcore 1;package equality_recovery version "1";fn answer()->Int=42;','original recovery source changed')
    require(source['id']['sha256']=='45d189275ebcc9b48588f0e9abc53d45f8841d6312e6c0dbcfc4665698627069','original recovery identity changed')
    for mode in ['native','wasm']:
        for replay in [0,1]:
            rows=store.read(f'recovery/{mode}-{replay}.json')
            same(rows,store.read('recovery/native-0.json'),'four complete original recovery captures differ')
            same(rows[1]['request'],plan['recovery'],'recovery source capture association')
            same(rows[-2]['response']['result'],recovery['outcome'],'whole original recovery outcome mismatch')
    same(recovery['outcome']['evaluation']['value']['data']['Scalar']['payload'],{'kind':'int64','value':'42'},'canonical original Int42')
    require('liveCapture'not in store.read('lifecycle-v1/failure.json'),'retroactive missing v1 partial capture')
    return recovery

def verify_lifetimes(store,plan,recovery):
    expected=store.read('lifecycle/native-0.json')
    for mode in ['native','wasm']:
        for replay in [0,1]:require(store.raw(f'lifecycle/{mode}-{replay}.json')==store.raw('lifecycle/native-0.json'),'whole lifecycle fresh captures differ')
    t=Transcript(expected['rows']);same(t.take({'method':'rcore_behavior/jobHost','namespace':'scalar-comparison-lifecycle'}),{'initialized':True},'lifecycle host')
    same(t.take({'method':'rcore_behavior/jobHost','namespace':'foreign-comparison-host'}),{'error':'Function job host already initialized'},'host namespace replacement allowed')
    sequence=0;lifetimes=0
    def start(request):
        nonlocal sequence
        sequence+=1;job='scalar-comparison-lifecycle:'+str(sequence)
        same(t.take(request),{'job':job,'pending':True},'deterministic lifecycle source handle')
        return job
    def recover(job):
        for _ in range(100):
            result=t.take({'method':'rcore_behavior/resumeFunction','job':job,'quantum':256})
            if not result.get('pending'):break
            same(result,{'job':job,'pending':True},'recovery Pending')
        same(result,{'pending':False,'result':recovery['outcome']},'whole old42recovery/program/counters')
        same(t.take({'method':'rcore_behavior/resumeFunction','job':job,'quantum':1}),UNKNOWN,'recovery finished handle')
    for point in plan['points']:
        for action in plan['actions']:
            first=t.at;dormant=[start(plan['recovery'])for _ in range(3)];job=start(point['request'])
            same(t.take(plan['recovery']),{'error':'Active function job budget exceeded (4)'},'four-slot admission')
            same(t.take({'method':'rcore_behavior/resumeFunction','job':'foreign-comparison-host:'+job.rsplit(':',1)[1],'quantum':256}),UNKNOWN,'foreign namespace accepted')
            for _ in range(point['prefix']):same(t.take({'method':'rcore_behavior/resumeFunction','job':job,'quantum':256}),{'job':job,'pending':True},'actual comparison prefix')
            stopped=t.take({'method':'rcore_behavior/'+('cancelFunction'if action=='cancel'else'dropFunction'),'job':job})
            if action=='cancel':
                require(stopped.get('pending')is False and stopped['result'].get('ok')is False,'cancel publishes success')
                require(stopped['result']['failure']['kind']=='Cancelled','wrong selected cancellation')
                same(stopped['result']['program'],point['complete']['program'],'cancel program/source changed')
                require('evaluation'not in stopped['result'],'cancel published evaluation')
            else:same(stopped,{'dropped':True},'drop result')
            for method in ['resumeFunction','cancelFunction','dropFunction']:
                request={'method':'rcore_behavior/'+method,'job':job}
                if method=='resumeFunction':request['quantum']=1
                same(t.take(request),UNKNOWN,'stale terminal handle accepted')
            recover(start(plan['recovery']))
            for other in dormant:recover(other)
            same(expected['lifetimes'][lifetimes],{'id':point['id'],'point':point['point'],'action':action,'prefix':point['prefix'],'witness':point['witness'],'first_row':first,'row_count':t.at-first,'stopped':stopped},'lifetime row/metadata/source association')
            lifetimes+=1
    t.finish();require(t.at==1706 and lifetimes==len(expected['lifetimes'])==36 and sequence==180,'lifecycle inventory changed')
    return t.at,t.source_checks

def optional_numeric_bridge(store):
    if 'numeric-after-native/run-0.json'not in store.manifest['entries']:return 'not_packaged'
    current=store.read('numeric-after-native/run-0.json')
    for key in ['numeric-after-WASM/run-0.json','numeric-after-WASM/run-1.json']:same(store.read(key),current,'whole native/WASM numeric capture differs')
    same(json.loads((ROOT/'numeric-lifecycle-after.json').read_bytes()),current['lifecycle'],'plain new cadence fixture differs')
    before=store.read('numeric-before/lifecycle.json');changes=[]
    def compare(a,b,path=''):
        if exact(a,b):return
        if re.fullmatch(r'/rows/\d+/polls',path):
            require(type(a)is int and type(b)is int,'poll count kind changed');changes.append({'path':path,'before':a,'after':b});return
        require(type(a)is type(b),'numeric semantic type changed: '+path)
        if isinstance(a,dict):
            require(a.keys()==b.keys(),'numeric semantic fields changed: '+path)
            for key in a:compare(a[key],b[key],path+'/'+key)
        elif isinstance(a,list):
            require(len(a)==len(b),'numeric semantic length changed: '+path)
            for i,(x,y)in enumerate(zip(a,b)):compare(x,y,path+'/'+str(i))
        else:raise AssertionError('numeric semantic value changed: '+path)
    compare(before,current['lifecycle']);require(bool(changes),'expected distinct numeric cadence change missing')
    return {'sole_changed_paths':changes,'new_whole_native_WASM_and_plain_fixture_exact':True}

def verify():
    store=Store();lookup,public_hashes,source_checks=verify_public(store)
    plan=verify_points(store,lookup);recovery=verify_recovery(store,plan)
    lifecycle_hashes,life_sources=verify_lifetimes(store,plan,recovery)
    ordinary=store.raw('before-ordinary/run-0.stdout')+store.raw('before-ordinary-unit/run-0.stdout')
    require(len(ordinary.splitlines())==162,'ordinary162 inventory')
    for replay in [0,1]:require(store.raw(f'current-ordinary/run-{replay}.stdout')==ordinary,'whole ordinary result/counters changed')
    native=(ROOT/'native/src/main.rs').read_bytes();expected=store.read('native256/terminal.json')['extra']['host_source']
    require(len(native)==expected['bytes']and sha(native)==expected['sha256'],'portable native phase/RPC helper drift')
    require((ROOT/'verify_runtime.mjs').read_bytes()==store.raw('portable-runtime-v2/verify_runtime.mjs'),'reusable runtime drift')
    print(json.dumps({'state':'pass','raw_origin_entries':len(store.manifest['entries']),'unique_raw_objects':len(store.cache),'public_cases':164,'ordinary_rows':162,'comparison_prefixes':18,'lifetimes':36,'public_request_hashes':public_hashes,'lifecycle_request_hashes':lifecycle_hashes,'source_hash_checks':source_checks+life_sources,'whole_original_recovery_comparisons':144,'numeric_bridge':optional_numeric_bridge(store),'scope':'Portable raw integrity plus exact source/case/role/request/whole-outcome derivations; actual native/WASM/browser execution remains separate.'}))
if __name__=='__main__':verify()
