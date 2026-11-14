"""Verification-only portable integrity; no Java, native build or model execution by default."""
from pathlib import Path, PurePosixPath, PureWindowsPath
import argparse,copy,gzip,hashlib,io,json,os,re
ROOT=Path(__file__).resolve().parent

def disk(path):
 path=Path(path).resolve()
 return Path('\\\\?\\'+str(path)) if os.name=='nt' and not str(path).startswith('\\\\?\\') else path

def raw(path):return disk(path).read_bytes()
def pin(value):return {'bytes':len(value),'sha256':hashlib.sha256(value).hexdigest()}
def read(path):return json.loads(raw(path).decode('utf-8'))
def owned(relative):
 p=PurePosixPath(relative)
 assert str(p)==relative and not p.is_absolute() and '..'not in p.parts and '\\'not in relative,relative
 return ROOT/relative

def decode_native(payload,entry):
 assert pin(payload)==entry['stored'],'compressed native bytes differ'
 assert payload[:3]==b'\x1f\x8b\x08' and payload[4:8]==bytes(4) and not payload[3]&8
 with gzip.GzipFile(fileobj=io.BytesIO(payload),mode='rb') as zipped:
  decoded=zipped.read(entry['original_raw']['bytes']+1)
  assert len(decoded)==entry['original_raw']['bytes'],'native raw byte length differs'
  assert zipped.read(1)==b'','native raw stream exceeds frozen bound'
 assert pin(decoded)==entry['original_raw'],'native raw SHA differs'
 decoded.decode('utf-8',errors='strict')
 return decoded

def comparable(rows,output):
 rows=copy.deepcopy(rows)
 for row in rows:
  provider=row.get('provider')
  if provider is None:continue
  h=provider['ecore_sha256'];assert re.fullmatch('[0-9a-f]{64}',h)
  assert row['source_pins']['imported.ecore']==h
  if re.match(r'^[A-Za-z]:[\\/]',output):
   root=PureWindowsPath(output);actual=PureWindowsPath(provider['genmodel']);expected=root/('provider-'+h[:12])/'provider.genmodel'
  else:
   root=PurePosixPath(output);actual=PurePosixPath(provider['genmodel']);expected=root/('provider-'+h[:12])/'provider.genmodel'
  assert actual.is_absolute() and root.is_absolute() and '..'not in actual.parts
  assert actual.parts==expected.parts,'GenModel exception escaped its exact owned output'
  provider['genmodel']='<exact-owned-provider-output>/provider.genmodel'
 return rows

def source_ids(value):
 if not isinstance(value,dict):return 0
 count=0
 compilation=value.get('compilation')
 if isinstance(compilation,dict):
  for source in compilation.get('bundle',{}).get('sources',[]):
   assert hashlib.sha256(source['text'].encode('utf-8')).hexdigest()==source['id']['sha256']
   count+=1
 return count

def verify(storage_negatives=False):
 manifest=read(ROOT/'extraction-manifest.json');entries=manifest['entries'];seen=set();native_streams=0;native_rows=0;source_checks=0;stored=0;raw_bytes=0;sample=None
 for entry in entries:
  rel=entry['path'];assert rel not in seen;seen.add(rel);payload=raw(owned(rel));stored+=len(payload)
  if entry['storage']=='raw':assert pin(payload)=={key:entry[key] for key in ['bytes','sha256']}
  else:
   assert entry['storage']=='gzip';decoded=decode_native(payload,entry);native_streams+=1;raw_bytes+=len(decoded)
   if sample is None:sample=(payload,entry)
   for line in decoded.decode('utf-8').splitlines():
    row=json.loads(line);assert 'request'in row and 'result'in row
    source_checks+=source_ids(row['request']);source_checks+=source_ids(row['request'].get('package',{}));native_rows+=1
 counts={'graphs':0,'clean_generation':0,'resource_complete':0,'resource_accepted':0,'resource_rejected':0,'resource_rejected_partials':0,'named_complete':0,'named_parser_accepted':0,'named_parser_executed':0,'named_parser_syntax_rejected':0,'named_entry_availability_failed':0,'resource_unexecuted':0,'resource_planned':0,'named_planned':0}
 for cohort in manifest['cohorts']:
  record=read(ROOT/cohort['record']/'observations.json');replay=read(ROOT/cohort['replay']/'observations.json');original_receipt=next(read(p) for p in disk(ROOT/'oracle/lineage').glob('*/receipt.json') if any(command.get('output','').replace('\\','/').endswith('/'+ {'original23':'hsv2r','namespace17':'hsv4r','boundary4':'hsv5r','discriminator2':'hsv6r','qualified2':'hsv7r','reference2':'hsv8r','suffix1':'hsv9r'}[cohort['id']]) for command in read(p).get('commands',[])))
  commands=original_receipt['commands'];assert len(commands)==2
  assert comparable(record,commands[0]['output'])==comparable(replay,commands[1]['output']),'Complete upstream observations differ'
  cases=read(ROOT/'oracle-producer/original'/cohort['original_helper']/'inputs/cases.json');assert len(cases)==len(record)
  for c,row in zip(cases,record):
   assert c['id']==row['id'] and c['entry']==row['entry'] and c['inputs']==row['original_inputs']
   counts['graphs']+=1;counts['resource_planned']+=len(c['inputs']);counts['named_planned']+=len(c['inputs'])*len(c['named_entries'])
   for source in c['sources']:
    assert row['source_pins'][source['file']]==source['source_id']['sha256']
    assert pin(raw(ROOT/'oracle-producer/original'/cohort['original_helper']/'inputs'/c['id']/source['file']))['sha256']==source['source_id']['sha256']
   clean=row['generation'] and row['generation']['exit_code']==0 and row['compilation'] and row['compilation']['exit_code']==0
   counts['clean_generation']+=bool(clean)
   if clean:
    assert [r['text'] for r in row['runtime']]==c['inputs']
    assert [(r['entry'],r['text']) for r in row['named_runtime']]==[(entry,text)for entry in c['named_entries']for text in c['inputs']]
   else:assert row['runtime']==[] and row['named_runtime']==[];counts['resource_unexecuted']+=len(c['inputs'])
   for r in row['runtime']:
    if r['exit_code']!=0 or r['observation']is None:continue
    resource=r['observation']['resource'];counts['resource_complete']+=1;counts['resource_accepted']+=bool(resource['accepted']);counts['resource_rejected']+=not resource['accepted'];counts['resource_rejected_partials']+=not resource['accepted'] and bool(resource.get('model'))
   for r in row['named_runtime']:
    if r['exit_code']==0 and r['observation']is not None:
     parser=r['observation']['parser'];counts['named_complete']+=1
     if parser.get('failure'):
      assert not parser['accepted']
      assert any(v.get('class')=='java.lang.NoSuchMethodException' for v in parser['failure']),parser['failure']
      counts['named_entry_availability_failed']+=1
     else:
      counts['named_parser_executed']+=1
      counts['named_parser_accepted']+=bool(parser['accepted'])
      counts['named_parser_syntax_rejected']+=not parser['accepted']
 assert counts=={'graphs':51,'clean_generation':22,'resource_complete':243,'resource_accepted':98,'resource_rejected':145,'resource_rejected_partials':122,'named_complete':178,'named_parser_accepted':56,'named_parser_executed':155,'named_parser_syntax_rejected':99,'named_entry_availability_failed':23,'resource_unexecuted':220,'resource_planned':463,'named_planned':280},counts
 negative=0
 if storage_negatives:
  payload,entry=sample
  trials=[(payload[:-1],entry),(payload[:len(payload)//2]+bytes([payload[len(payload)//2]^1])+payload[len(payload)//2+1:],entry),(payload,{**entry,'original_raw':{**entry['original_raw'],'sha256':'0'*64}}),(payload[:-1],{**entry,'stored':pin(payload[:-1])})]
  for data,expected in trials:
   try:decode_native(data,expected)
   except (AssertionError,EOFError,OSError,gzip.BadGzipFile):negative+=1
   else:raise AssertionError('Corrupt/truncated/mismatched native storage unexpectedly admitted')
 return {'state':'portable_raw_integrity_and_original_complete_record_replay_equal','files':len(entries),'stored_bytes':stored,'native_gzip_streams':native_streams,'native_raw_bytes':raw_bytes,'complete_native_rows':native_rows,'exact_source_id_checks':source_checks,'counts':counts,'storage_negative_controls':negative,'comparison_exception':'Only exact absolute owned provider-<EcoreSHA12>/provider.genmodel output path','gate_closures':[]}

original_verify=verify
def verify(storage_negatives=False):
 result=original_verify(storage_negatives)
 manifest=read(ROOT/'final-evidence-manifest.json')
 assert manifest['original_extraction_manifest']==pin(raw(ROOT/'extraction-manifest.json'))
 streams=[];extra_bytes=0;paths=set()
 for entry in manifest['entries']:
  relative=entry['path'];assert relative not in paths;paths.add(relative)
  payload=raw(owned(relative));assert pin(payload)==entry['stored'],relative
  extra_bytes+=len(payload)
  if entry['storage']=='deterministic-gzip-complete-native-jsonl':
   decoded=decode_native(payload,entry);rows=[json.loads(line) for line in decoded.splitlines()]
   assert len(rows)==2147
   assert all(set(row)=={'label','request','result','cohort'} for row in rows)
   streams.append({'relative':relative,'raw':entry['original_raw'],'rows':len(rows)})
  else:assert entry['storage']=='raw'
 assert len(streams)==2 and streams[0]['raw']==streams[1]['raw']
 native=read(ROOT/'verification/final-v1/native-attempt4-build/terminal.json')
 wasm=read(ROOT/'verification/final-v1/wasm-attempt2-build/terminal.json')
 assert native['exit_code']==wasm['exit_code']==0
 for name in ['dev-attempt1','production-attempt1','old-nine-dev-attempt1','old-nine-production-attempt1']:
  assert read(ROOT/'verification/final-v1/application'/name/'terminal.json')['exit_code']==0
 result['final_attempt4_evidence']={'files':len(paths),'stored_bytes':extra_bytes,'complete_native_record_rows':streams[0]['rows'],'complete_native_replay_rows':streams[1]['rows'],'full_native_record_replay_equal':True,'application_new_controls_per_mode':3,'application_old_controls_per_mode':9,'native_public_WASM_full_request_equality':'2147 requests/run independently recorded/replayed; receipt pins actual producer/artifacts'}
 return result

if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--storage-negatives',action='store_true');p.add_argument('--root',type=Path);a=p.parse_args();ROOT=a.root.resolve() if a.root else ROOT;print(json.dumps(verify(a.storage_negatives),ensure_ascii=True,sort_keys=True))
