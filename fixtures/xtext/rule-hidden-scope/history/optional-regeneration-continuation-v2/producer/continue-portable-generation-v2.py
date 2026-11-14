# Separate additive continuation after hsp1's strict process-timeout failure.
# Original sources, helpers, 15s deadlines, full comparator and sole exception unchanged.
"""Optional full source-loaded generation replay using byte-exact original cohort producers."""
from pathlib import Path
import argparse,hashlib,importlib.util,json,os,subprocess,sys
HERE=Path(__file__).resolve().parent;ROOT=Path('D:\\dev\\repos\\mercurio-labs\\intersect\\target\\hidden-scope-candidate-v63joined-v1\\portable-hidden-scope-v2')
spec=importlib.util.spec_from_file_location('hidden_portable_verify',ROOT/'verify.py');verify=importlib.util.module_from_spec(spec);spec.loader.exec_module(verify)
def raw(p):return verify.raw(p)
def pin(p):return verify.pin(raw(p))
def save(p,v):verify.disk(p).write_text(json.dumps(v,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
def main():
 p=argparse.ArgumentParser();p.add_argument('--output',required=True,type=Path);p.add_argument('--jdk',required=True,type=Path);p.add_argument('--maven-cache',required=True,type=Path);a=p.parse_args();out=a.output.resolve();assert out!=ROOT and ROOT not in out.parents;out.mkdir(exist_ok=False)
 integrity=verify.verify(True);manifest=verify.read(ROOT/'extraction-manifest.json');planned=[]
 for cohort in [c for c in manifest['cohorts'] if c['id'] in {'discriminator2','qualified2','reference2','suffix1'}]:
  helper=ROOT/'oracle-producer/original'/cohort['original_helper'];planned.append({'id':cohort['id'],'helper':str(helper),'producer':pin(helper/'produce-hidden.py'),'inputs':pin(helper/'inputs/pins.json'),'stack':pin(helper/'inputs/stack.json')})
 save(out/'preexecution.json',{'state':'before_execution','portable_integrity':integrity,'replay_wrapper':pin(__file__),'portable_verifier':pin(ROOT/'verify.py'),'extraction_manifest':pin(ROOT/'extraction-manifest.json'),'jdk_java':pin(a.jdk/'bin'/('java.exe'if os.name=='nt'else'java')),'jdk_javac':pin(a.jdk/'bin'/('javac.exe'if os.name=='nt'else'javac')),'cohorts':planned,'source_uris':'Original logical bindings and source/hash/classifier/diagnostic metadata unchanged. No source/provider/helper rewriting.','sole_exception':'Only exact absolute owned provider-<EcoreSHA12>/provider.genmodel output path'})
 runs=[]
 try:
  for cohort in [c for c in manifest['cohorts'] if c['id'] in {'discriminator2','qualified2','reference2','suffix1'}]:
   helper=ROOT/'oracle-producer/original'/cohort['original_helper'];original=verify.read(ROOT/cohort['record']/'observations.json');receipt=next(verify.read(v)for v in verify.disk(ROOT/'oracle/lineage').glob('*/receipt.json')if any(c.get('output','').replace('\\','/').endswith('/'+{'original23':'hsv2r','namespace17':'hsv4r','boundary4':'hsv5r','discriminator2':'hsv6r','qualified2':'hsv7r','reference2':'hsv8r','suffix1':'hsv9r'}[cohort['id']])for c in verify.read(v).get('commands',[])));expected=verify.comparable(original,receipt['commands'][0]['output'])
   for label in ['record','replay']:
    produced=out/(cohort['id']+'-'+label);command=[sys.executable,'-B',str(helper/'produce-hidden.py'),'--output',str(produced),'--jdk',str(a.jdk.resolve()),'--maven-cache',str(a.maven_cache.resolve())];run={'cohort':cohort['id'],'run':label,'command':command,'output':str(produced)};runs.append(run);save(out/'runs.json',runs);print('START '+cohort['id']+' '+label,flush=True)
    with (out/(cohort['id']+'-'+label+'.stdout.log')).open('xb')as stdout,(out/(cohort['id']+'-'+label+'.stderr.log')).open('xb')as stderr:r=subprocess.run(command,stdout=stdout,stderr=stderr)
    run['exit_code']=r.returncode;save(out/'runs.json',runs);assert r.returncode==0,(cohort['id'],label,r.returncode)
    artifacts=verify.read(produced/'artifacts-sha256.json');actual={v.relative_to(verify.disk(produced)).as_posix()for v in verify.disk(produced).rglob('*')if v.is_file()}-{'artifacts-sha256.json'};assert actual==set(artifacts)
    for rel,h in artifacts.items():assert pin(produced/rel)['sha256']==h,rel
    rows=verify.read(produced/'observations.json');assert verify.comparable(rows,str(produced))==expected,(cohort['id'],label,'complete original observations differ')
    run['artifacts']=len(artifacts);run['artifact_manifest']=pin(produced/'artifacts-sha256.json');run['observations']=pin(produced/'observations.json');run['complete_original_observations_equal']=True;save(out/'runs.json',runs);print('PASS '+cohort['id']+' '+label,flush=True)
  verify.verify(True)
  save(out/'receipt.json',{'state':'selected_remaining4cohorts_source_loaded_original_record_and_independent_fresh_replay_equal','graphs_per_run':7,'resource_observations_per_run':65,'named_observations_per_run':86,'source_unexecuted_resource_per_run':14,'original_full_attempt':'hsp1 terminalFAIL remains immutable; no full all7 attempt PASS claim','runs':runs,'preexecution':pin(out/'preexecution.json'),'gate_closures':[]});print('COMPLETE_ALL',flush=True)
 except BaseException as error:
  save(out/'failure.json',{'state':'failure_preserved_no_source_repair','class':type(error).__name__,'message':str(error),'runs':runs,'preexecution':pin(out/'preexecution.json')});raise
if __name__=='__main__':main()
