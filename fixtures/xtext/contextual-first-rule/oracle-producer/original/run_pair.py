from pathlib import Path
import subprocess,sys,json
HERE=Path(__file__).resolve().parent
for name in ['record','replay']:
 cmd=[sys.executable,str(HERE/'produce.py'),'--output',str(HERE/name),'--jdk','D:/dev/jdks/jdk21','--maven-cache','C:/Users/MarkPetrotta/.m2/repository']
 with (HERE/(name+'.stdout.log')).open('w',encoding='utf-8',newline='\n')as out,(HERE/(name+'.stderr.log')).open('w',encoding='utf-8',newline='\n')as err:
  result=subprocess.run(cmd,stdout=out,stderr=err)
 (HERE/(name+'.exit')).write_text(str(result.returncode)+'\n',encoding='utf-8')
 print(name+' exit '+str(result.returncode),flush=True)
 if result.returncode:raise SystemExit(result.returncode)
result=subprocess.run([sys.executable,str(HERE/'verify.py'),'--record',str(HERE/'record'),'--replay',str(HERE/'replay'),'--receipt',str(HERE/'receipt.json')])
(HERE/'verify.exit').write_text(str(result.returncode)+'\n',encoding='utf-8')
raise SystemExit(result.returncode)
