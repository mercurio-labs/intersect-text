from pathlib import Path
import hashlib,json,subprocess,time
T=Path('D:/dev/repos/mercurio-labs/intersect/target');C=T/'hidden-scope-candidate-v63joined-v1';F=T/'hsp1/discriminator2-record';O=C/'portable-exact-timeout-probe-v1';O.mkdir(exist_ok=False)
def pin(p):b=p.read_bytes();return {'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
def read(p):return json.loads(p.read_text(encoding='utf-8'))
def save(p,v):p.write_text(json.dumps(v,indent=2,ensure_ascii=True)+'\n',encoding='utf-8',newline='\n')
failed=next(c for c in read(F/'commands.json')if c['exit_code']==124);command=failed['command'];assert failed['timeout_seconds']==15 and failed['raw_capture_before_decode'];newrow=read(F/'observations.json')[1]['runtime'][12];oldrow=read(T/'hsv6r/observations.json')[1]['runtime'][12];assert newrow['text']==oldrow['text']=='directab?' and newrow['observation']==oldrow['observation'];index=command.index('intersect.oracle.HiddenScopeRuntime');oldcmd=next(c for c in read(T/'hsv6r/commands.json')if 'intersect.oracle.HiddenScopeRuntime'in c['command']and c['command'][c['command'].index('intersect.oracle.HiddenScopeRuntime')+1:c['command'].index('intersect.oracle.HiddenScopeRuntime')+5]==command[index+1:index+5]);assert oldcmd['exit_code']==0
oldstdout=T/'hsv6r'/oldcmd['stdout'];newstdout=F/failed['stdout'];assert oldstdout.read_bytes()==newstdout.read_bytes()
manifest=read(F/'artifacts-sha256.json');before={p:pin(F/p)for p in manifest};assert all(h['sha256']==manifest[p]for p,h in before.items())
save(O/'preexecution.json',{'state':'new exact-command probes; historical124 remains failure','producer':pin(__file__),'command':command,'deadline_seconds':15,'historical_command':failed,'old_command':oldcmd,'old_stdout':pin(oldstdout),'failed_stdout':pin(newstdout),'failed_stderr':pin(F/failed['stderr']),'complete_observation_equal':True,'original_fresh_artifact_manifest':pin(F/'artifacts-sha256.json'),'artifacts_verified':len(before),'java':pin(Path(command[0])),'trials':3,'source_helper_workflow_changes':False})
trials=[]
for n in range(1,4):
 start=time.monotonic()
 try:p=subprocess.run(command,capture_output=True,timeout=15)
 except subprocess.TimeoutExpired as error:p=subprocess.CompletedProcess(command,124,error.stdout or b'',error.stderr or b'')
 stdout=O/(str(n)+'.stdout.log');stderr=O/(str(n)+'.stderr.log');stdout.write_bytes(p.stdout);stderr.write_bytes(p.stderr)
 decoded=p.stdout.decode('utf-8',errors='strict');observations=[json.loads(line)for line in decoded.splitlines()if line.startswith('{')]
 row={'trial':n,'exit_code':p.returncode,'deadline_seconds':15,'wall_seconds_observed':time.monotonic()-start,'stdout':pin(stdout),'stderr':pin(stderr),'raw_stdout_equal_historical_complete_JSON':p.stdout==newstdout.read_bytes(),'complete_observation_equal':len(observations)==1 and observations[0]==oldrow['observation']};trials.append(row);save(O/'trials.json',trials)
assert all(pin(F/p)==h for p,h in before.items())
save(O/'receipt.json',{'state':'separate exact-command process completion observations; original full replay failure remains immutable','preexecution':pin(O/'preexecution.json'),'trials':trials,'completed_within_original_deadline':sum(t['exit_code']==0 for t in trials),'complete_observation_equal':all(t['complete_observation_equal']for t in trials),'historical_classification':'full original run terminalFAIL at15s process timeout despite complete equivalent JSON; never rewritten asPASS','source_generated_original_artifacts_unchanged':True,'gate_closures':[]});print(json.dumps({'receipt':str(O/'receipt.json'),**pin(O/'receipt.json'),'trials':trials}))
