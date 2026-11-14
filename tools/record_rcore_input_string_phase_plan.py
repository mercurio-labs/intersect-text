"""Record phase-aware controls without changing original logical expectations."""
import argparse,gzip,hashlib,json,subprocess,traceback
from pathlib import Path
R=Path(__file__).resolve().parent.parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--phase-witness',type=Path,required=True,help='Output from the shared native quantum-one phase witness')
parser.add_argument('--oracle',type=Path,required=True,help='Built intersect-web function_job_transcript executable')
parser.add_argument('--out',type=Path,required=True,help='New output directory, never overwritten')
args=parser.parse_args()
O=args.out.resolve()
def pin(p):
 p=Path(p)
 with p.open('rb')as f:h=hashlib.file_digest(f,'sha256').hexdigest()
 return {'bytes':p.stat().st_size,'sha256':h}
def save(n,v):(O/n).write_text(json.dumps(v,indent=2)+'\n',encoding='utf-8',newline='\n')
try:
 witness_path=args.phase_witness.resolve();witness=json.loads(witness_path.read_bytes())
 literal=json.loads((R/'fixtures/rcore-input-string-transport/plan.json').read_bytes())
 assert len(witness['cases'])==len(literal['cases'])==2
 checkpoints={}
 for observed,original_case in zip(witness['cases'],literal['cases']):
  assert observed['case']==original_case['id'] and observed['quantum']==1
  assert observed['original_request']==original_case['request']
  assert observed['expected_evaluation']==original_case['complete']['evaluation']
  for name,point in observed['checkpoints'].items():
   assert point['poll']>0 and point['arena'][1:3]==[0,0]
   if name=='measure':assert point['phase'][:2]==['decode','measure'] and point['arena'][0]==576
   else:
    assert point['phase'][:2]==['decode','string-copy']
    assert 0<point['phase'][2][7]<point['phase'][2][6]==original_case['literal_bytes']
    assert point['arena'][0]==original_case['decoder_admission']
  checkpoints[observed['case']]=observed['checkpoints']
 original_path=R/'fixtures/rcore-input-string-transport/recorded/public-candidate-plan.gz'
 original=json.loads(gzip.decompress(original_path.read_bytes()));assert len(original['groups'])==24
 oracle={'path':str(args.oracle.resolve()),**pin(args.oracle)}
 O.mkdir(exist_ok=False)
 groups=[]
 for g in original['groups']:
  requests=g['requests'];expected=g['native_responses'];change=None
  if 'action_index'in g:
   action=g['action_index'];kind='copy_middle'if g['id'].endswith('cancelFunction')else'copy_late';point=checkpoints[g['case']][kind];poll=point['poll']
   # Index3 is the original foreign-job probe. Preserve it and every other request.
   assert all(r==requests[4]for r in requests[4:action]);assert requests[4]['quantum']==1
   assert len(requests[4:action])==g['input_copy_prefix_units']
   assert all(r==expected[4]for r in expected[4:action]);assert expected[4]['pending']is True
   if kind=='copy_middle':assert expected[action]['result']['failure']==point['failure']
   requests=requests[:4]+[requests[4]]*poll+requests[action:]
   expected=expected[:4]+[expected[4]]*poll+expected[action:]
   change={'original_action_index':action,'original_input_copy_prefix_units':g['input_copy_prefix_units'],'new_action_index':4+poll,'checkpoint':point,'preserved':'Only repeated quantum-one resume prefix length changes. All original source/start/action/recovery/limit/stale/foreign requests and exact logical result/error assertions remain unchanged.'}
  groups.append({'id':g['id'],'case':g['case'],'requests':requests,'expected_responses':expected,'phase_schedule_bridge':change})
 save('preexecution.json',{'state':'prepared_before_any_native_execution','producer':pin(__file__),'original_plan':{'path':str(original_path),**pin(original_path)},'witness':{'path':str(witness_path),**pin(witness_path)},'oracle':oracle,'groups':24,'changed_prefixes':sum(g['phase_schedule_bridge']is not None for g in groups),'gate_closures':[]})
 save('phase-plan-before-execution.json',{'groups':groups})
 captures=[]
 for replay in range(2):
  print('START native public replay '+str(replay),flush=True);observed=[]
  for g in groups:
   request=json.dumps(g['requests'],ensure_ascii=True,sort_keys=True,separators=(',',':')).encode()
   result=subprocess.run([oracle['path']],input=request,capture_output=True)
   (O/f"native-{replay}-{g['id']}.stdout").write_bytes(result.stdout);(O/f"native-{replay}-{g['id']}.stderr").write_bytes(result.stderr)
   assert result.returncode==0,(g['id'],result.stderr.decode(errors='replace'))
   rows=json.loads(result.stdout);assert len(rows)==len(g['requests'])
   if g['id'].endswith('-baseline'):
    terminal=next(x for x in g['expected_responses']if x.get('pending')is False);complete=None
    for i,response in enumerate(rows):
     if i<3:assert response==g['expected_responses'][i],(g['id'],i,response)
     elif complete is not None:assert response=={'error':'Unknown, stale or foreign function job'},(g['id'],i,response)
     elif response.get('pending')is False:assert response==terminal,(g['id'],i,response);complete=i
     else:assert response=={'pending':True,'job':g['expected_responses'][2]['job']},(g['id'],i,response)
    assert complete is not None and complete>3,(g['id'],complete)
   else:assert rows==g['expected_responses'],g['id']
   observed.append({'id':g['id'],'responses':rows})
  save(f'native-replay-{replay}.json',{'groups':observed});captures.append(observed)
 assert captures[0]==captures[1]
 for g,observed in zip(groups,captures[0]):assert g['id']==observed['id'];g['qualified_native_responses']=observed['responses']
 save('qualified-phase-plan.json',{'groups':groups})
 save('native-receipt.json',{'state':'all_24_original_public_groups_pass_twice_with_four_source_witnessed_copy_prefixes','preexecution':{'path':str(O/'preexecution.json'),**pin(O/'preexecution.json')},'plan':{'path':str(O/'qualified-phase-plan.json'),**pin(O/'qualified-phase-plan.json')},'captures':[{'path':str(O/f'native-replay-{i}.json'),**pin(O/f'native-replay-{i}.json')}for i in range(2)],'groups':24,'responses_each':sum(len(g['requests'])for g in groups),'scope':'Original fixed-schedule failure remains preserved. New schedule explicitly selects true partial String copy for four controls. Complete old terminal/error/fee assertions and all other20groups unchanged; native observations supply exact WASM replay reference, never semantic authority.','gate_closures':[]})
 print(json.dumps({'receipt':str(O/'native-receipt.json'),**pin(O/'native-receipt.json')}),flush=True)
except BaseException:
 if O.exists():save('failure.json',{'state':'original_failure_preserved','traceback':traceback.format_exc()})
 raise
