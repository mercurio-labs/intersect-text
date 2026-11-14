"""Optional pinned Xtext terminal translation + ANTLR execution, verification only."""
import argparse,base64,hashlib,json,os,subprocess
from pathlib import Path

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def compare_observations(actual,expected):
 # Only a JVM stack-overflow traceback is unstable: keep both raw traces pinned.
 # Require the exact failure class and both recursive ANTLR generator frames.
 actual=json.loads(json.dumps(actual));expected=json.loads(json.dumps(expected));exceptions=[]
 for left,right in zip(actual,expected):
  if left.get('id')!='cyclic_fragment' or right.get('id')!='cyclic_fragment':continue
  a=left.get('generation');b=right.get('generation')
  if a is None or b is None or a.get('diagnostic')==b.get('diagnostic'):continue
  required=['Exception in thread "main" java.lang.StackOverflowError','org.antlr.grammar.v2.TreeToNFAConverter.testSetElement','org.antlr.grammar.v2.TreeToNFAConverter.testSetRule']
  assert a['exit_code']==b['exit_code']==1
  assert all(value in item['diagnostic'] for item in [a,b] for value in required)
  a['diagnostic']=b['diagnostic']='<variable JVM traceback; same StackOverflowError in recursive ANTLR set generation>'
  exceptions.append('cyclic_fragment/generation/diagnostic: variable JVM StackOverflowError stack frames; raw traces retained')
 assert actual==expected
 return exceptions

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--jdk',type=Path,required=True);p.add_argument('--artifact-dir',type=Path,required=True);p.add_argument('--maven-cache',type=Path,default=Path.home()/'.m2/repository');p.add_argument('--record',action='store_true');a=p.parse_args()
 root=Path(__file__).resolve().parent.parent;fixture=root/'fixtures/xtext/terminal-negation';out=a.artifact_dir.resolve();out.mkdir(parents=True,exist_ok=False)
 stack=root/'fixtures/xtext/syntax-ast/provenance.json';pins=json.loads(stack.read_text(encoding='utf-8'))['jars'];jars=[]
 for item in pins:
  path=a.maven_cache/item['path'];assert digest(path)==item['sha256'],item['path'];jars.append(str(path))
 if not a.record:
  frozen=json.loads((fixture/'provenance.json').read_text(encoding='utf-8'))
  for name,pin in frozen['files'].items():assert digest(root/name)==pin,name
 cp=os.pathsep.join(jars);suffix='.exe' if os.name=='nt' else '';java=str(a.jdk/'bin'/('java'+suffix));javac=str(a.jdk/'bin'/('javac'+suffix));number=0
 def run(command,cwd=out,timeout=60):
  nonlocal number
  number+=1
  try:r=subprocess.run(command,cwd=cwd,capture_output=True,text=True,encoding='utf-8',errors='strict',timeout=timeout)
  except subprocess.TimeoutExpired as error:
   stdout=error.stdout or b'';stderr=error.stderr or b''
   r=subprocess.CompletedProcess(command,124,stdout.decode('utf-8') if isinstance(stdout,bytes) else stdout,stderr.decode('utf-8') if isinstance(stderr,bytes) else stderr)
  (out/f'{number:03d}.stdout.log').write_text(r.stdout,encoding='utf-8',newline='\n');(out/f'{number:03d}.stderr.log').write_text(r.stderr,encoding='utf-8',newline='\n');return r
 compiled=run([javac,'-encoding','UTF-8','-cp',cp,'-d',str(out),str(fixture/'TerminalNegationOracle.java')]);compiled.check_returncode()
 runtime=str(out)+os.pathsep+cp;rows=[]
 for index,case in enumerate(json.loads((fixture/'cases.json').read_text(encoding='utf-8'))):
  directory=out/case['id'];directory.mkdir();(directory/'imported.ecore').write_bytes((root/'fixtures/xtext/terminal-spellings/model.ecore').read_bytes());(directory/'order.txt').write_text('\n'.join(s['name'] for s in case['sources'])+'\n',encoding='utf-8',newline='\n');(directory/'entry.txt').write_text(case['entry']+'\n',encoding='utf-8',newline='\n')
  for source in case['sources']:(directory/(source['name']+'.xtext')).write_text(source['text'],encoding='utf-8',newline='\n')
  result=run([java,'-cp',runtime,'intersect.oracle.TerminalNegationOracle','metadata',str(directory)]);result.check_returncode();lines=[line for line in result.stdout.splitlines() if line.startswith('{')];assert len(lines)==1,case['id'];row={'id':case['id'],**json.loads(lines[0]),'generation':None,'runtime':[]}
  if row['translation_failure'] is None:
   name='NegationAudit'+str(index);grammar='lexer grammar '+name+';\n@members { public int errors=0; @Override public void reportError(RecognitionException e) { errors++; } }\n'+'\n'.join(('fragment ' if r['fragment'] else '')+r['name']+': '+r['body']+';' for r in row['translated_rules'])+'\n';g=directory/(name+'.g');g.write_text(grammar,encoding='utf-8',newline='\n')
   generated=run([java,'-cp',cp,'org.antlr.Tool',str(g)]);row['generation']={'exit_code':generated.returncode,'diagnostic':generated.stderr.replace(str(out),'<artifact-dir>'),'grammar':grammar,'grammar_sha256':digest(g),'java_canonical_sha256':None,'compilation_exit_code':None}
   if generated.returncode==0:
    src=directory/(name+'.java');canonical='\n'.join(src.read_text(encoding='utf-8').splitlines()[1:])+'\n';canonical=canonical.replace(str(g).replace(chr(92),chr(92)*2),'<grammar>').replace(str(g),'<grammar>');row['generation']['java_canonical_sha256']=hashlib.sha256(canonical.encode('utf-8')).hexdigest()
    built=run([javac,'-encoding','UTF-8','-cp',cp,'-d',str(out),str(src)]);row['generation']['compilation_exit_code']=built.returncode;row['generation']['compilation_diagnostic']=built.stderr.replace(str(out),'<artifact-dir>')
    if built.returncode==0:
     for text in case['inputs']:
      lexed=run([java,'-cp',runtime,'intersect.oracle.TerminalNegationOracle','lex',name,base64.b64encode(text.encode('utf-8')).decode('ascii')],timeout=10)
      if lexed.returncode==124:row['runtime'].append({'text':text,'timeout_seconds':10,'exit_code':124})
      else:
       lexed.check_returncode();values=[json.loads(line) for line in lexed.stdout.splitlines() if line.startswith('{')];assert len(values)==1;row['runtime'].append(values[0])
  rows.append(row);(directory/'observation.json').write_text(json.dumps(row,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n');print(case['id'],'translated',len(row['translated_rules']),'generation',row['generation']['exit_code'] if row['generation'] else None,'runtime',len(row['runtime']),flush=True)
 (out/'observations.json').write_text(json.dumps(rows,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
 artifact_pins={str(path.relative_to(out)).replace(chr(92),'/'):digest(path) for path in sorted(out.rglob('*')) if path.is_file()};(out/'artifacts-sha256.json').write_text(json.dumps(artifact_pins,indent=2)+'\n',encoding='utf-8',newline='\n')
 if a.record:
  (fixture/'observations.json').write_bytes((out/'observations.json').read_bytes());files=['fixtures/xtext/terminal-negation/cases.json','fixtures/xtext/terminal-negation/TerminalNegationOracle.java','fixtures/xtext/terminal-negation/observations.json','tools/xtext_terminal_negation_oracle.py','fixtures/xtext/syntax-ast/provenance.json','fixtures/xtext/terminal-spellings/model.ecore'];version=run([java,'-version']).stderr
  provenance={'scope':__doc__,'stages':'Original Xtext syntax, post-link resource diagnostics/call identities, full validation, pinned TerminalRuleToLexerBody translation, pinned ANTLR generation and lexer execution recorded separately. Translation/generation are attempted even when validation reports errors; this does not qualify the source grammar.','generated_java_comparison':'Generated Java text line endings, first timestamp header and exact input grammar path are canonicalized for replay digest; raw generated bytes remain untouched and SHA256-pinned in each artifact directory artifacts-sha256.json. No source semantics or token observations are normalized. Runtime inputs run in separate processes with a 10-second verification timeout recorded explicitly.','jdk':version,'jars':pins,'files':{name:digest(root/name) for name in files}}
  (fixture/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n',encoding='utf-8',newline='\n')
 else:
  exceptions=compare_observations(rows,json.loads((fixture/'observations.json').read_text(encoding='utf-8')))
  (out/'comparison.json').write_text(json.dumps({'matched_cases':len(rows),'diagnostic_exceptions':exceptions},indent=2)+'\n',encoding='utf-8',newline='\n')
 print('Complete independent stage observations:',len(rows))
if __name__=='__main__':main()
