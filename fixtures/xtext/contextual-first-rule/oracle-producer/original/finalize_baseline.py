from pathlib import Path
import base64,copy,hashlib,json
HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def read(p):return json.loads(p.read_text(encoding='utf-8'))
def pin(p):return {'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)}
def main():
 assert int((HERE/'verify.exit').read_text(encoding='utf-8'))==0
 pre=read(HERE/'preexecution.json')
 for n,p in pre['files'].items():assert sha(HERE/n)==p['sha256'],n
 for n,h in read(HERE/'inputs/pins.json').items():assert sha(HERE/'inputs'/n)==h,n
 cases=read(HERE/'inputs/cases.json');record=read(HERE/'record/observations.json');classifier=read(HERE/'metadata-v2-record/observations.json')
 expected_runtime=[(r['entry'],base64.b64encode(v['text'].encode('utf-8')).decode('ascii'))for r in record for v in r['runtime']]
 runs={};original_helper_failures={}
 for key in ['record','replay']:
  root=HERE/key;commands=read(root/'commands.json');actual=[tuple(c['command'][-2:])for c in commands if 'intersect.oracle.FullRuntime'in c['command']]
  assert actual==expected_runtime
  original_helper_failures[key]=[{'command_index':i+1,'exit_code':c['exit_code'],'stderr':pin(root/c['stderr'])}for i,c in enumerate(commands)if 'intersect.oracle.FirstRuleMetadata'in c['command']and c['exit_code']]
  assert len(original_helper_failures[key])==2
  toolchain=read(root/'toolchain.json');assert '21.0.6+7'in toolchain['jdk']and len(toolchain['jars'])==60
  runs[key]={'observations':pin(root/'observations.json'),'commands':pin(root/'commands.json'),'artifact_manifest':pin(root/'artifacts-sha256.json'),'toolchain':pin(root/'toolchain.json'),'actual_runtime_command_arguments_checked':len(actual)}
  metadata=HERE/('metadata-v2-'+key);files=read(metadata/'artifacts-sha256.json')
  assert set(files)=={p.relative_to(metadata).as_posix()for p in metadata.rglob('*')if p.is_file()and p.name!='artifacts-sha256.json'}
  for n,h in files.items():assert sha(metadata/n)==h
 prior_root=HERE.parent/'full-finite-until-context-v1';lineage=[];current={r['id']:r for r in record}
 for directory in ['remaining-record','valid-inheritance-v2-record']:
  p=prior_root/directory/'observations.json'
  for row in read(p):
   if row['id']not in pre['original_graph_ids']:continue
   before=copy.deepcopy(row);now=copy.deepcopy(current[row['id']]);now.pop('first_rule_metadata')
   for value in [before,now]:
    path=Path(value['provider']['genmodel']);expected='provider-'+value['provider']['ecore_sha256'][:12]
    assert path.is_absolute()and path.name=='provider.genmodel'and path.parent.name==expected
    value['provider']['genmodel']='OWNED_OUTPUT_ROOT/'+expected+'/provider.genmodel'
   assert now==before,row['id']
   lineage.append({'id':row['id'],'original_observations':pin(p),'existing_full_row_equal_except_additive_classifier_report_and_owned_genmodel_output_root':True})
 assert len(lineage)==4
 raw=[v for r in record for v in r['runtime']]
 accepted=sum(bool(v['observation']and v['observation']['resource']and v['observation']['resource'].get('accepted'))for v in raw)
 partial=sum(bool(v['observation']and v['observation']['resource']and not v['observation']['resource'].get('accepted')and v['observation']['resource'].get('model'))for v in raw)
 result={'version':1,'state':'original_contextual_first_rule_before_baseline_ready','scope':'Independent pinned Xtext2.38 full source validation/generation/runtime record and fresh replay, plus additive metadata-only V2 repair. No production implementation or native qualification.','input_freeze':pin(HERE/'preexecution.json'),'original_full_replay_receipt':pin(HERE/'receipt.json'),'metadata_v2_addendum':pin(HERE/'metadata-v2-receipt.json'),'source_pin_manifest':pin(HERE/'inputs/pins.json'),'runs':runs,'graphs':len(cases),'new_original_provider_graphs':12,'new_separate_identity_provider_graphs':4,'unchanged_original_graphs':4,'planned_runtime_inputs_per_run':sum(len(c['inputs'])for c in cases),'generation_and_java_compilation_successes':sum(bool(r['compilation']and r['compilation']['exit_code']==0)for r in record),'generation_failures':sum(bool(r['generation']and r['generation']['exit_code']!=0)for r in record),'actual_runtime_inputs_per_run':len(raw),'actual_full_runtime_processes_both_runs':len(raw)*2,'unexecuted_original_requests_per_run':sum(len(c['inputs'])for c,r in zip(cases,record)if not r['runtime']),'accepted_models_per_run':accepted,'rejected_per_run':len(raw)-accepted,'rejected_partial_models_per_run':partial,'original_helper_failure_proof':original_helper_failures,'original_four_complete_observation_lineage':lineage,'stage_notes':[
 'Terminal-only grammar is validation-clean but default standalone StandardLanguage generation rejects a grammar without production rules; child/library controls retain actual standalone library generation failures separately from admission.',
 'Frozen datatype_only_library has no contextual first-rule error but has an independent keyword-hides-terminal error. It is not a whole-language generation positive.',
 'Class-fragment-first reports InvalidFragmentFirstRule validator ERROR, yet MWE generation and javac succeed; all five actual resource and named Model parser probes then fail missing generated entry methods. Generation success is not runtime support.',
 'No-authored-return fragment resolves to Ecore EString EDataType; it does not qualify null resolved classifier behavior.',
 'The separately authored provider retains EClass EString and EEnum Choice with exact namespace identity distinct from Ecore EDataType EString. The class-returning Model graph generates, compiles and executes; datatype-first counterpart fails contextual first-rule validation.',
 'Two enum rows retain the initial extra metadata helper NullPointerException in both original runs. The additive V2 helper boxes optional boolean observations, matches all18 initially successful metadata rows and independently replays all20 exact classifier/used-grammar rows without normalization.',
 'All four reused source graphs and original stage/runtime row projections match their original complete observations; neither failed graph nor its unexecuted requests was repaired.'
 ],'classification_rows':classifier,'qualification_limits':['Validation, MWE generation, javac, lexer, resource and named parser stages remain separate.','Raw UTF16 tokens/errors/models/partials/leaves are preserved, not reduced to accepted model outcomes.','Only exact absolute owned provider.genmodel output path is a comparison exception.','No native source changes, gate closure, Java production dependency or native Ecore import award.'],'producer':pin(Path(__file__)),'native_support_award':False,'gate_closures':[]}
 out=HERE/'baseline-ready.json';assert not out.exists();out.write_text(json.dumps(result,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n');print(json.dumps(pin(out)))
if __name__=='__main__':main()
