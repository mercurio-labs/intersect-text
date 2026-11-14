"""Read-only replay of the separate nested-negation upstream companion."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import subprocess

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--jdk',type=Path,required=True);parser.add_argument('--artifact-dir',type=Path,required=True);parser.add_argument('--maven-cache',type=Path,default=Path.home()/'.m2/repository');args=parser.parse_args()
 root=Path(__file__).resolve().parent.parent;fixture=root/'fixtures/xtext/terminal-negation';out=args.artifact_dir.resolve();out.mkdir(parents=True,exist_ok=False)
 provenance=json.loads((fixture/'nested-provenance.json').read_text(encoding='utf-8'))
 for name,pin in provenance['files'].items():assert sha(root/name)==pin,name
 jars=[]
 for pin in provenance['jars']:
  path=args.maven_cache/pin['path'];assert sha(path)==pin['sha256'];jars.append(str(path))
 cp=os.pathsep.join(jars);suffix='.exe'if os.name=='nt'else '';java=str(args.jdk/'bin'/('java'+suffix));javac=str(args.jdk/'bin'/('javac'+suffix));serial=0
 def run(command,cwd=out):
  nonlocal serial
  serial+=1;result=subprocess.run(command,cwd=cwd,capture_output=True,text=True,encoding='utf-8',timeout=20);(out/f'{serial:03d}.stdout.log').write_text(result.stdout,encoding='utf-8',newline='\n');(out/f'{serial:03d}.stderr.log').write_text(result.stderr,encoding='utf-8',newline='\n');result.check_returncode();return result
 run([javac,'-encoding','UTF-8','-cp',cp,'-d',str(out),str(fixture/'TerminalNegationOracle.java')]);runtime=str(out)+os.pathsep+cp;rows=[]
 for index,case in enumerate(json.loads((fixture/'nested-cases.json').read_text(encoding='utf-8'))):
  directory=out/case['id'];directory.mkdir();(directory/'imported.ecore').write_bytes((root/'fixtures/xtext/terminal-spellings/model.ecore').read_bytes());(directory/'order.txt').write_text('\n'.join(s['name']for s in case['sources'])+'\n',encoding='utf-8',newline='\n');(directory/'entry.txt').write_text(case['entry']+'\n',encoding='utf-8',newline='\n')
  for source in case['sources']:(directory/(source['name']+'.xtext')).write_text(source['text'],encoding='utf-8',newline='\n')
  result=run([java,'-cp',runtime,'intersect.oracle.TerminalNegationOracle','metadata',str(directory)]);metadata=json.loads(next(line for line in result.stdout.splitlines()if line.startswith('{')));row={'id':case['id'],**metadata,'generation_exit':None,'runtime':[]};assert row['translation_failure']is None
  name='ExploreNegation'+str(index);grammar='lexer grammar '+name+';\n@members {public int errors=0; @Override public void reportError(RecognitionException e){errors++;}}\n'+'\n'.join(('fragment 'if r['fragment']else '')+r['name']+': '+r['body']+';'for r in row['translated_rules'])+'\n';path=directory/(name+'.g');path.write_text(grammar,encoding='utf-8',newline='\n');generated=run([java,'-cp',cp,'org.antlr.Tool',str(path)]);row['generation_exit']=generated.returncode;row['generation_diagnostic']=generated.stderr.replace(str(out),'<artifact-dir>');run([javac,'-encoding','UTF-8','-cp',cp,'-d',str(out),str(directory/(name+'.java'))])
  for text in case['inputs']:
   result=run([java,'-cp',runtime,'intersect.oracle.TerminalNegationOracle','lex',name,base64.b64encode(text.encode('utf-8')).decode('ascii')]);row['runtime'].extend(json.loads(line)for line in result.stdout.splitlines()if line.startswith('{'))
  rows.append(row)
 (out/'observations.json').write_text(json.dumps(rows,indent=2)+'\n',encoding='utf-8',newline='\n');(out/'artifacts-sha256.json').write_text(json.dumps({str(path.relative_to(out)).replace(chr(92),'/'):sha(path)for path in sorted(out.rglob('*'))if path.is_file()},indent=2)+'\n',encoding='utf-8',newline='\n');assert rows==json.loads((fixture/'nested-observations.json').read_text(encoding='utf-8'));print('Two independent nested-negation graphs and six runtime observations replayed exactly')
if __name__=='__main__':main()
