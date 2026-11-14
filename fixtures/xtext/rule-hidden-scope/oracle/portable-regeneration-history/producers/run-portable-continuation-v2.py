from pathlib import Path
import hashlib,json,subprocess,sys
R=Path(__file__).parent;C=R.parent/'hidden-scope-candidate-v63joined-v1';O=C/'portable-generation-continuation-v2';P=C/'portable-hidden-scope-v2'
def pin(p):b=p.read_bytes();return {'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
driver=R/'continue-portable-generation-v2.py';held={str(p):pin(p)for p in [driver,P/'verify.py',P/'oracle-producer/replay.py',P/'extraction-manifest.json']};command=[sys.executable,'-B',str(driver),'--output',str(R.parent/'hsp2'),'--jdk','D:/dev/jdks/jdk21','--maven-cache','C:/Users/MarkPetrotta/.m2/repository']
(O/'command.json').write_text(json.dumps({'command':command,'held':held,'prepared_preexecution':pin(O/'preexecution.json')},indent=2)+'\n',encoding='utf-8',newline='\n')
with(O/'stdout.log').open('xb')as stdout,(O/'stderr.log').open('xb')as stderr:r=subprocess.run(command,stdout=stdout,stderr=stderr)
assert all(pin(Path(p))==value for p,value in held.items())
terminal={'exit_code':r.returncode,'inputs_unchanged':held,'stdout':pin(O/'stdout.log'),'stderr':pin(O/'stderr.log'),'command':pin(O/'command.json'),'state':'separate continuation terminal; original full hsp1 FAIL preserved'}
p=O/'terminal.json';p.write_text(json.dumps(terminal,indent=2)+'\n',encoding='utf-8',newline='\n');print(json.dumps({'terminal':str(p),**pin(p),'exit_code':r.returncode}));raise SystemExit(r.returncode)
