"""Pure verification; optional --corpus selects the unchanged portable baseline."""
from pathlib import Path,PureWindowsPath,PurePosixPath
import argparse,copy,hashlib,json,os,re
HERE=Path(__file__).resolve().parent
def disk(p):
 p=Path(p).resolve();return Path('\\\\?\\'+str(p))if os.name=='nt'and not str(p).startswith('\\\\?\\')else p
def raw(p):return disk(p).read_bytes()
def read(p):return json.loads(raw(p).decode('utf-8'))
def digest(p):b=raw(p);return {'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
def comparable(rows,output):
 rows=copy.deepcopy(rows)
 for row in rows:
  provider=row.get('provider')
  if provider is None:continue
  h=provider['ecore_sha256'];assert re.fullmatch('[0-9a-f]{64}',h)
  kind=PureWindowsPath if re.match(r'^[A-Za-z]:[\\/]',output)else PurePosixPath
  root=kind(output);actual=kind(provider['genmodel']);assert root.is_absolute()and actual.is_absolute()
  assert actual.parts==(root/('provider-'+h[:12])/'provider.genmodel').parts
  provider['genmodel']='<exact-owned-provider-output>/provider.genmodel'
 return rows
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--corpus',type=Path,default=HERE.parents[1]);args=parser.parse_args();corpus=args.corpus.resolve()
 manifest=read(HERE/'raw-copy-manifest.json');seen=set()
 for entry in manifest['entries']:
  rel=entry['path'];assert rel not in seen;seen.add(rel);p=(HERE/rel).resolve();assert HERE in p.parents
  assert digest(p)=={k:entry[k]for k in ['bytes','sha256']},rel
 receipt=read(HERE/'outputs/receipt.json');extraction=read(corpus/'extraction-manifest.json');ids={'discriminator2':'hsv6r','qualified2':'hsv7r','reference2':'hsv8r','suffix1':'hsv9r'};pairs=0
 for cohort in [c for c in extraction['cohorts']if c['id']in ids]:
  lineage=next(read(p)for p in disk(corpus/'oracle/lineage').glob('*/receipt.json')if any(c.get('output','').replace('\\','/').endswith('/'+ids[cohort['id']])for c in read(p).get('commands',[])))
  expected=comparable(read(corpus/cohort['record']/'observations.json'),lineage['commands'][0]['output'])
  runs=[r for r in receipt['runs']if r['cohort']==cohort['id']];assert [r['run']for r in runs]==['record','replay']
  for run in runs:
   out=HERE/'outputs'/(cohort['id']+'-'+run['run']);assert run['exit_code']==0
   artifacts=read(out/'artifacts-sha256.json')
   for name,h in artifacts.items():assert digest(out/name)['sha256']==h
   assert comparable(read(out/'observations.json'),run['output'])==expected;pairs+=1
 assert pairs==8
 print(json.dumps({'state':'additive_continuation_raw_integrity_and_complete_original_JSON_equal','raw_files':len(seen),'generation_runs':pairs,'counts_per_run':read(HERE/'review.json')['counts_per_run'],'original_full_attempt':'hsp1 failed process timeout remains failed; no all7 full-attempt PASS','gate_closures':[]},sort_keys=True))
if __name__=='__main__':main()
