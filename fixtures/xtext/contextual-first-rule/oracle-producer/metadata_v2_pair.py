"""Metadata-only independent addendum; original producer and NPE remain frozen."""
from pathlib import Path
import hashlib,json,os,shutil,subprocess,argparse
HERE=Path(__file__).resolve().parent.parent
HELPERS=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8'))
def save(p,v):p.write_text(json.dumps(v,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
def pin(p):return {'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)}
def main():
 parser=argparse.ArgumentParser();parser.add_argument('--output',type=Path,required=True);parser.add_argument('--jdk',type=Path,required=True);parser.add_argument('--maven-cache',type=Path,required=True);args=parser.parse_args();OUT=args.output.resolve();assert OUT!=HERE and HERE not in OUT.parents,'Output must stay outside the corpus';OUT.mkdir(exist_ok=False)
 cases=read(HERE/'inputs/cases.json');pins=read(HERE/'inputs/pins.json')
 for n,h in pins.items():assert sha(HERE/'inputs'/n)==h,n
 original=read(HERE/'oracle/receipts/preexecution.json')
 assert sha(HELPERS/'FirstRuleMetadata.java')==original['files']['FirstRuleMetadata.java']['sha256']
 stack=read(HERE/'inputs/stack.json');cache=args.maven_cache.resolve();jars=[]
 for row in stack['jars']:
  p=cache/row['path'];assert sha(p)==row['sha256'];jars.append(str(p))
 cp=os.pathsep.join(jars);jdk=args.jdk.resolve();suffix='.exe'if os.name=='nt'else '';java=jdk/'bin'/('java'+suffix);javac=jdk/'bin'/('javac'+suffix)
 freeze=OUT/'metadata-v2-preexecution.json';assert not freeze.exists()
 save(freeze,{'state':'additive_metadata_only_adapter_frozen_before_execution','original_preexecution':pin(HERE/'oracle/receipts/preexecution.json'),'original_helper':pin(HELPERS/'FirstRuleMetadata.java'),'new_helper':pin(HELPERS/'FirstRuleMetadataV2.java'),'producer':pin(Path(__file__)),'source_inputs':pin(HERE/'inputs/pins.json'),'change':'Use Object + explicit Boolean.valueOf for fragment/datatype observations, avoiding EnumRule conditional-null auto-unboxing. No validator logic, source graph, provider or original observation changed.','graphs':len(cases),'source_semantics_change':False})
 outputs={};runs={}
 for key in ['record','replay']:
  root=OUT/('metadata-v2-'+key);root.mkdir(exist_ok=False);commands=[]
  def run(cmd):
   v=subprocess.run(list(map(str,cmd)),cwd=root,capture_output=True,text=True,encoding='utf-8',errors='strict',timeout=60);n=len(commands)+1
   (root/f'{n:03d}.stdout.log').write_text(v.stdout,encoding='utf-8',newline='\n');(root/f'{n:03d}.stderr.log').write_text(v.stderr,encoding='utf-8',newline='\n');commands.append({'command':list(map(str,cmd)),'exit_code':v.returncode,'stdout':f'{n:03d}.stdout.log','stderr':f'{n:03d}.stderr.log'});save(root/'commands.json',commands);assert v.returncode==0,(key,n,v.stderr);return v
  v=run([java,'-version']);assert '21.0.6+7'in v.stderr
  classes=root/'adapter';classes.mkdir();run([javac,'-encoding','UTF-8','--release','17','-cp',cp,'-d',classes,HELPERS/'FirstRuleMetadataV2.java',HERE/'inputs/TerminalNegationOracle.java'])
  rows=[]
  for c in cases:
   target=root/c['id'];target.mkdir();source=HERE/'inputs'/c['id']
   for name in ['imported.ecore',*[s['file']for s in c['sources']]]:shutil.copyfile(source/name,target/name);assert sha(source/name)==sha(target/name)
   (target/'order.txt').write_text(''.join(s['grammar']+'\n'for s in c['sources']),encoding='utf-8',newline='\n')
   result=run([java,'-cp',str(classes)+os.pathsep+cp,'intersect.oracle.FirstRuleMetadataV2',target]);data=[json.loads(s)for s in result.stdout.splitlines()if s.startswith('{')];assert len(data)==1
   rows.append({'id':c['id'],'observation':data[0]});save(root/'observations.json',rows)
  save(root/'artifacts-sha256.json',{p.relative_to(root).as_posix():sha(p)for p in sorted(root.rglob('*'))if p.is_file()})
  outputs[key]=rows;runs[key]={'observations':pin(root/'observations.json'),'artifacts':pin(root/'artifacts-sha256.json'),'commands':len(commands),'fresh_metadata_processes':len(cases)}
 assert outputs['record']==outputs['replay']
 old=read(HERE/'oracle/record/observations.json');unchanged=0;recovered=[]
 for row,prior in zip(outputs['record'],old):
  assert row['id']==prior['id']
  if prior['first_rule_metadata']['exit_code']==0:
   assert row['observation']==prior['first_rule_metadata']['observation'];unchanged+=1
  else:recovered.append(row['id'])
 result={'state':'additive_metadata_v2_record_and_independent_replay_equal','preexecution':pin(freeze),'original_metadata_failure_preserved':True,'unchanged_successful_original_metadata_rows':unchanged,'recovered_original_helper_failure_rows':recovered,'graphs':len(cases),'normalization':'none','runs':runs,'native_support_award':False,'gate_closures':[]}
 save(OUT/'metadata-v2-receipt.json',result);print(json.dumps(pin(OUT/'metadata-v2-receipt.json')))
if __name__=='__main__':main()
