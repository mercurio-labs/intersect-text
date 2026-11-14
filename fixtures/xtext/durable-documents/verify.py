"""Standalone, stdlib-only integrity and source-bound observation verifier.

Historical absolute paths are data only. This tool reads only this corpus and an
optional explicitly selected repository's pinned consumer fixture paths.
"""
from pathlib import Path,PurePosixPath
import argparse,copy,gzip,hashlib,io,json,re,struct,sys
from unittest.mock import patch

ROOT=Path(__file__).resolve().parent
def require(condition,message):
    if not condition:raise ValueError(message)
def pin(b):return {'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
def canonical(v):return hashlib.sha256(json.dumps(v,ensure_ascii=True,sort_keys=True,separators=(',',':')).encode('ascii')).hexdigest()
def pointer(value,path):
    require(path==''or path.startswith('/'),'invalid JSON pointer')
    for token in path.split('/')[1:]:
        token=token.replace('~1','/').replace('~0','~')
        value=value[int(token)]if isinstance(value,list)else value[token]
    return value
def read_json(p):return json.loads(p.read_text(encoding='utf-8'))
def source_identity(source):
    require(hashlib.sha256(source['text'].encode('utf-8')).hexdigest()==source['id']['sha256'],'authored source identity mismatch')
    require(isinstance(source['id']['uri'],str)and bool(source['id']['uri']),'empty source URI')
def check_derivation(corpus,item):
    a=pointer(corpus.json(item['fixture']),item['pointer'])
    b=pointer(corpus.json(item['raw']['origin']),item['raw']['pointer'])
    require(a==b,'whole raw field differs: '+item['fixture']+item['pointer'])
    require(canonical(a)==item['canonical_value_sha256'],'derivation digest differs')
def carrier(value):
    if isinstance(value,str)and any(0xd800<=ord(c)<=0xdfff for c in value):
        data=value.encode('utf-16-le',errors='surrogatepass')
        return {'utf16_units':list(struct.unpack('<'+'H'*(len(data)//2),data))}
    if isinstance(value,dict):return {k:carrier(v)for k,v in value.items()}
    if isinstance(value,list):return [carrier(v)for v in value]
    return value

class Corpus:
    def __init__(self,root=ROOT):
        self.root=root;self.origins={};self.cache={}
        for name in ['origins.json','origins-addendum-v1.json','origins-addendum-v2.json','origins-addendum-v3.json','origins-addendum-v4.json']:
            part=read_json(root/name)
            require(not self.origins.keys()&part.keys(),'duplicate origin')
            self.origins.update(part)
    def raw(self,name):
        q=self.origins[name]
        require(re.fullmatch('[a-f0-9]{64}',q['sha256'])is not None,'invalid blob digest')
        require(q['blob']=='blobs/'+q['sha256'],'unowned blob path')
        b=(self.root/q['blob']).read_bytes()
        require(pin(b)=={k:q[k]for k in ['bytes','sha256']},'blob bytes/digest mismatch: '+name)
        return b
    def json(self,name):
        if name not in self.cache:
            text=self.raw(name).decode('utf-8',errors='strict')
            self.cache[name]=[json.loads(line)for line in text.splitlines()if line]if name.endswith('.jsonl')else json.loads(text)
        return self.cache[name]
    def rows(self,name,expected_raw):
        raw=self.raw(name);require(raw[3]==0 and raw[4:8]==bytes(4),'non-deterministic gzip header')
        sha=hashlib.sha256();length=0
        with gzip.GzipFile(fileobj=io.BytesIO(raw),mode='rb')as stream:
            for line in stream:
                sha.update(line);length+=len(line);yield json.loads(line.decode('utf-8',errors='strict'))
        require({'bytes':length,'sha256':sha.hexdigest()}==expected_raw,'expanded complete capture mismatch')

def verify(corpus,repository=None):
    inventory=read_json(corpus.root/'file-manifest.json')['files']
    actual={p.relative_to(corpus.root).as_posix()for p in corpus.root.rglob('*')if p.is_file()and'__pycache__'not in p.parts and p.suffix!='.pyc'}-{'file-manifest.json'}
    require(actual==set(inventory),'portable file inventory differs')
    for name,expected in inventory.items():
        require(not PurePosixPath(name).is_absolute()and'..'not in PurePosixPath(name).parts,'manifest path traversal')
        require(pin((corpus.root/name).read_bytes())==expected,'portable file pin differs: '+name)
    unique=set();byte_count=0;consumer=0
    for name,q in corpus.origins.items():
        require(not PurePosixPath(name).is_absolute()and'..'not in PurePosixPath(name).parts,'origin path traversal')
        b=corpus.raw(name)
        if q['sha256']not in unique:unique.add(q['sha256']);byte_count+=len(b)
        if name.startswith('consumer/'):
            consumer+=1
            if repository is not None:require((repository/name[len('consumer/'):]).read_bytes()==b,'installed consumer differs: '+name)
    require(consumer==14,'consumer fixture inventory changed')
    derivations=read_json(corpus.root/'derivations-v1.json')
    addendum=read_json(corpus.root/'derivations-addendum-v2.json')
    require(not addendum['unmapped'],'unmapped additional source observation')
    derivations['recipes']+=addendum['recipes']
    require(not derivations['unmapped'],'unmapped compared observation')
    for item in derivations['recipes']:
        check_derivation(corpus,item)
    # Exact original observer, default resource and named IParser are separate.
    observed=corpus.json('original/node-observer-review-packet-v1/observed/observations.json')
    record=[{k:v for k,v in r.items()if k!='run'}for r in observed if r['run']=='record']
    replay=[{k:v for k,v in r.items()if k!='run'}for r in observed if r['run']=='replay']
    require(record==replay and len(record)==194,'original388 complete observations differ')
    lookup={(r['case'],r['text'],r['entry']):r['observation']for r in record}
    named=corpus.json('consumer/crates/intersect-xtext/src/document_tests/named_nodes.json')['rows']
    units=corpus.json('consumer/crates/intersect-xtext/src/document_tests/named_nodes_units.json')['rows']
    require(len(named)==len(units)==104,'named104 changed')
    for source,actual in zip(named,units):
        expected=carrier(source)
        expected['default_resource']=carrier(lookup[(source['id'],source['text'],source['entry'])]['resource'])
        require(actual==expected,'named stage/unit carrier mismatch')
    constructions=corpus.json('consumer/crates/intersect-xtext/src/document_tests/construction_nodes.json')['rows']
    construction_units=corpus.json('consumer/crates/intersect-xtext/src/document_tests/construction_nodes_units.json')['rows']
    require(len(constructions)==len(construction_units)==26,'construction26 changed')
    expected_cases={'current_actions_original':'current_actions_original',
        'grouped_objects_namespace_companion':'grouped_objects_namespace_companion',
        'simple_actions_same_source_library_workflow_addendum':'simple_actions_empty_behavior_selected_companion'}
    for source,actual in zip(constructions,construction_units):
        expected=carrier(source);expected['compilation_case_id']=expected_cases[source['id']]
        if source['id']=='simple_actions_same_source_library_workflow_addendum':
            # Preserve this historical note verbatim. Actual current context is
            # the distinct SELECTED EMPTY behavior source; absent selector failed
            # transport and has no execution credit. Source envelopes, not this
            # old prose annotation, determine context authority.
            expected['native_context_note']='distinct absent initializer selection; original selected-provider fixture remains in original_provider_positive.json'
        require(actual==expected,'construction lossless carrier/context mapping differs')
    # Identical raw record/replay; no filtering diagnostics or converter fields.
    pairs=[
      'original/provider-rgb-node-observer-v1/execution/',
      'original/provider-rgb-source-node-bindings-v2/execution/',
      'original/nested-provider-source-bindings-v1/execution/',
    ]
    complete_pair_rows=0
    for base in pairs:
        a=corpus.json(base+'record.json');b=corpus.json(base+'replay.json')
        require(a==b,'complete recorded replay differs: '+base);complete_pair_rows+=len(a)
    for origin in ['generated/xncl1/observations.json']:
        rows=corpus.json(origin)
        a=[{k:v for k,v in r.items()if k!='run'}for r in rows if r['run']=='record']
        b=[{k:v for k,v in r.items()if k!='run'}for r in rows if r['run']=='replay']
        require(a==b and len(a)==17,'separated-loader17 raw outcomes differ')
    bound=corpus.json('source-bindings-manifest-original/xnso3/observations.json')
    a=[{k:v for k,v in r.items()if k!='run'}for r in bound if r['run']=='record']
    b=[{k:v for k,v in r.items()if k!='run'}for r in bound if r['run']=='replay']
    require(a==b and len(a)==11,'whole source-loaded binding replay differs')
    # Every source envelope checked by its authored bytes, never classifier spelling.
    source_checks=0
    def sources(v):
        nonlocal source_checks
        if isinstance(v,dict):
            source=v.get('id')
            if isinstance(v.get('text'),str)and isinstance(source,dict)and 'uri'in source and 'sha256'in source:
                source_identity(v);source_checks+=1
            for x in v.values():sources(x)
        elif isinstance(v,list):
            for x in v:sources(x)
    for name in corpus.origins:
        if name.startswith('consumer/'):sources(corpus.json(name))
    # License entries are extracted from the exact pinned unchanged artifacts.
    licenses=read_json(corpus.root/'upstream-license-notices.json')
    require(len(licenses)==60,'pinned runtime JAR inventory changed')
    for jar in licenses:
        require(re.fullmatch('[a-f0-9]{64}',jar['jar_sha256'])is not None,'bad upstream JAR pin')
        for item in jar['notices']:require(pin(corpus.raw(item['origin']))=={k:item[k]for k in ['bytes','sha256']},'license notice differs')
    dependency=read_json(corpus.root/'existing-corpus-dependencies.json')
    require(dependency['repository_corpus']=='fixtures/xtext/rule-hidden-scope','incorrect existing corpus lineage')
    require(pin(corpus.raw('dependency/'+dependency['repository_corpus']+'/extraction-manifest.json'))==dependency['manifest'],'existing generation manifest differs')
    for item in dependency['root_manifests']:
        require(pin(corpus.raw('dependency/'+item['repository_path']))=={k:item[k]for k in ['bytes','sha256']},'old generated root manifest differs')
    storage=read_json(corpus.root/'original2147-storage.json')
    baseline=list(corpus.rows(storage['origin'],storage['raw']))
    require(len(baseline)==2147,'original2147 count changed')
    receipt=corpus.json('final-proof/original2147-no-document-offset-attempt1/receipt.json')
    public_rows=0
    for capture in receipt['captures']:
        origin='final-proof/original2147-no-document-offset-attempt1/'+capture['pass']+'.jsonl.gz'
        count=0
        for old,actual in zip(baseline,corpus.rows(origin,capture['raw']),strict=True):
            require(actual['request']==old['request']and actual['label']==old['label']and actual.get('cohort')==old.get('cohort'),'original request/identity changed')
            require(actual['native']==old['result']and actual['wasm']==old['result'],'complete old result changed, including fees/errors')
            count+=1
        require(count==2147,'complete2147 truncated');public_rows+=count
    legacy=corpus.json('consumer/crates/intersect-web/tests/fixtures/legacy_document_tail.json')
    require(sum(legacy==row for row in baseline)==1,'original trailing-token fixture no longer exact baseline row')
    final128=corpus.json('public-native/no-document-offset-complete-abi-attempt1/record.jsonl')
    require(final128==corpus.json('public-native/no-document-offset-complete-abi-attempt1/replay.jsonl')and len(final128)==128,'complete128 replay changed')
    for row in final128:require(row['native']==row['wasm'],'complete128 cross-ABI mismatch')
    return {'status':'PASS pure portable byte/source/whole-observation integrity',
      'origins':len(corpus.origins),'unique_blobs':len(unique),'unique_bytes':byte_count,
      'consumer_fixtures':consumer,'whole_field_derivations':len(derivations['recipes']),
      'original_node_observations':388,'named_stage_pairs':104,'construction_pairs':26,
      'typed_and_binding_replay_rows':complete_pair_rows,'source_identity_checks':source_checks,
      'upstream_jars_pinned':60,'license_notices':sum(len(x['notices'])for x in licenses),
      'existing_generated_root_bindings':len(dependency['root_manifests']),
      'original2147_complete_capture_rows':public_rows,'complete_abi_rows':len(final128),
      'limits':['Pure verifier does not execute Java/native/WASM or award language gates.','Raw source/generator/process failures, partials and withdrawn context credit remain unchanged.','Raw getLookAhead internal counters remain unqualified; semantic prediction dependencies are separate.'],
      'gate_closures':[]}

def negatives(corpus):
    results=[]
    def rejected(label,action):
        failed=False
        try:action()
        except(ValueError,KeyError,EOFError,OSError):failed=True
        require(failed,'negative control accepted: '+label);results.append(label)
    name=next(iter(corpus.origins));entry=corpus.origins[name]
    for label,data in [('mutated-blob',b'changed'),('truncated-blob',corpus.raw(name)[:-1])]:
        # Invoke the SAME raw reader, including pin and owned-path checks.
        with patch.object(Path,'read_bytes',return_value=data):
            rejected(label,lambda:corpus.raw(name))
    sources=corpus.json('consumer/crates/intersect-xtext/src/document_tests/cases.json')
    source=copy.deepcopy(next(s for c in sources['cases']for s in c['compilation']['bundle']['sources']if s['text']))
    source['text']+=' ';rejected('source-identity-mismatch',lambda:source_identity(source))
    item=copy.deepcopy(read_json(corpus.root/'derivations-v1.json')['recipes'][0]);item['raw']['origin']='unmapped-source'
    rejected('unmapped-source-derivation',lambda:check_derivation(corpus,item))
    item=copy.deepcopy(read_json(corpus.root/'derivations-v1.json')['recipes'][0]);item['canonical_value_sha256']='0'*64
    rejected('mismatched-source-derivation',lambda:check_derivation(corpus,item))
    lone='x\ud83d';require(carrier(lone)=={'utf16_units':[120,0xd83d]},'lone surrogate lost')
    rejected('lone-surrogate-replacement',lambda:require(carrier(lone)=={'utf16_units':[120,0xfffd]},'replacement character differs'))
    compressed=next((corpus.raw(n)for n in corpus.origins if n.endswith('.jsonl.gz')),None)
    require(compressed is not None,'missing original compressed capture')
    for label,data in [('truncated-gzip',compressed[:-8]),('corrupt-gzip',compressed[:-1]+bytes([compressed[-1]^1]))]:
        rejected(label,lambda:gzip.decompress(data))
    return results

def main():
    parser=argparse.ArgumentParser();parser.add_argument('--repository',type=Path);parser.add_argument('--negatives',action='store_true')
    args=parser.parse_args();corpus=Corpus();result=verify(corpus,args.repository)
    if args.negatives:result['negative_controls']=negatives(corpus)
    print(json.dumps(result,ensure_ascii=True,sort_keys=True))
if __name__=='__main__':
    try:main()
    except(Exception)as error:
        print(json.dumps({'status':'FAIL','error':str(error)},ensure_ascii=True),file=sys.stderr);raise
