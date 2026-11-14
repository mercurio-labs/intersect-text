from pathlib import Path
import hashlib,importlib.util,json
T=Path('D:/dev/repos/mercurio-labs/intersect/target');C=T/'hidden-scope-candidate-v63joined-v1';P=C/'portable-hidden-scope-v2';O=C/'portable-generation-difference-v1';O.mkdir(exist_ok=False)
spec=importlib.util.spec_from_file_location('frozen_verify',P/'verify.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
old=P/'oracle/record/discriminator2/observations.json';new=T/'hsp1/discriminator2-record/observations.json';a=v.comparable(v.read(old),str(T/'hsv6r'));b=v.comparable(v.read(new),str(T/'hsp1/discriminator2-record'))
diff=[]
def visit(x,y,p):
 if type(x)!=type(y):diff.append({'path':p,'old':x,'new':y});return
 if isinstance(x,dict):
  for key in sorted(set(x)|set(y)):
   if key not in x or key not in y:diff.append({'path':p+'/'+key,'old':x.get(key),'new':y.get(key),'missing_key':True})
   else:visit(x[key],y[key],p+'/'+key)
 elif isinstance(x,list):
  if len(x)!=len(y):diff.append({'path':p+'/length','old':len(x),'new':len(y)})
  for i,(left,right)in enumerate(zip(x,y)):visit(left,right,p+'/'+str(i))
 elif x!=y:diff.append({'path':p,'old':x,'new':y})
visit(a,b,'')
def pin(p):raw=p.read_bytes();return {'bytes':len(raw),'sha256':hashlib.sha256(raw).hexdigest()}
receipt={'state':'read-only exact comparison difference, no new exception or baseline modification','original':pin(old),'fresh':pin(new),'frozen_comparator':pin(P/'verify.py'),'sole_original_exception':'exact owned provider GenModel path only','differences':diff,'difference_count':len(diff),'old_case_ids':[r['id']for r in a],'new_case_ids':[r['id']for r in b],'fresh_failure':pin(T/'hsp1/failure.json'),'raw_terminal':pin(C/'portable-generation-replay-v1/terminal.json')}
(O/'difference.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=True)+'\n',encoding='utf-8',newline='\n');print(json.dumps({'difference':str(O/'difference.json'),**pin(O/'difference.json'),'differences':diff},ensure_ascii=True))
