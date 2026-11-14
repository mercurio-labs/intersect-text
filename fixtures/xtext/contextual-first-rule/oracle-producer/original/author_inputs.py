from pathlib import Path
import hashlib,json,shutil
HERE=Path(__file__).resolve().parent
OLD=HERE.parent/'full-until-literal-suffix-v1'
UNTIL=HERE.parent/'full-finite-until-context-v1'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(p,v):p.write_text(json.dumps(v,ensure_ascii=True,indent=2)+'\n',encoding='utf-8',newline='\n')
original=(OLD/'inputs/literal_until_suffix_c/imported.ecore').read_bytes()
assert hashlib.sha256(original).hexdigest()=='726b2116570fad065036c20eca40be7c4bda665317089fc2cd1f1cab5ea3f734'
new=original.decode('utf-8').replace('name="terminalSpellings"','name="firstRuleIdentity"').replace('urn:intersect:xtext:mini-kerml:terminal-spellings:1','urn:intersect:xtext:first-rule:identity:1').replace('</ecore:EPackage>','  <eClassifiers xsi:type="ecore:EClass" name="EString" eSuperTypes="#//Model"/>\n  <eClassifiers xsi:type="ecore:EEnum" name="Choice">\n    <eLiterals name="A" value="0"/>\n    <eLiterals name="B" value="1"/>\n  </eClassifiers>\n</ecore:EPackage>').encode('utf-8')
cases=[]
def add(id,sources,question,identity=False):
 p=HERE/'inputs'/id;p.mkdir(exist_ok=False);(p/'imported.ecore').write_bytes(new if identity else original)
 names=[]
 for grammar,parent,body in sources:
  head='grammar '+grammar+(' with '+parent if parent else '')+'\n\nimport "'+('urn:intersect:xtext:first-rule:identity:1'if identity else 'urn:intersect:xtext:mini-kerml:terminal-spellings:1')+'" as k\nimport "http://www.eclipse.org/emf/2002/Ecore" as ecore\n\n'
  file=grammar+'.xtext';(p/file).write_text(head+body+'\n',encoding='utf-8',newline='\n')
  names.append({'grammar':grammar,'file':file,'source_id':{'uri':'memory:'+id+'/'+file,'sha256':sha(p/file)}})
 cases.append({'id':id,'entry':sources[-1][0],'sources':names,'inputs':['a','b','','ab','a\U0001f30d'],'question':question,'provider_cohort':'separate_authored_identity_provider'if identity else 'unchanged_original_provider','origin':'New independently authored pre-production contextual first-rule control; no original graph repair.'})

add('terminal_only_library',[('audit.FirstRuleTerminalOnly',None,"terminal VALUE: 'a';")],'Terminal-only grammar has no non-datatype parser in its used closure.')
add('datatype_only_library',[('audit.FirstRuleDatatypeOnly',None,"Text returns ecore::EString: 'a';\nterminal VALUE: 'a';")],'Datatype-only grammar first rule with no non-datatype parser.')
add('terminal_library_model_child',[('audit.FirstRuleTerminalLibraryBase',None,"terminal VALUE: 'a';"),('audit.FirstRuleTerminalLibraryChild','audit.FirstRuleTerminalLibraryBase','Model returns k::Model: name=VALUE;')],'A terminal-only library is used by a child whose own first rule is an object parser.')
add('datatype_library_model_child',[('audit.FirstRuleDatatypeLibraryBase',None,"Text returns ecore::EString: 'a';"),('audit.FirstRuleDatatypeLibraryChild','audit.FirstRuleDatatypeLibraryBase','Model returns k::Model: name=Text;')],'A datatype-only library is used by a child whose own first rule is an object parser.')
add('terminal_before_local_model',[('audit.FirstRuleTerminalBeforeModel',None,"terminal VALUE: 'a';\nModel returns k::Model: name=VALUE;")],'Terminal-first grammar has an object parser later in the same grammar.')
add('datatype_before_local_model',[('audit.FirstRuleDatatypeBeforeModel',None,"Text returns ecore::EString: 'a';\nModel returns k::Model: name=Text;")],'Datatype-first grammar has an object parser later in the same grammar.')
add('terminal_child_over_model_parent',[('audit.FirstRuleTerminalChildBase',None,"Model returns k::Model: name=VALUE;\nterminal VALUE: 'a';"),('audit.FirstRuleTerminalChildChild','audit.FirstRuleTerminalChildBase',"terminal OTHER: 'b';")],'Terminal-first child reaches an object parser in its used grammar.')
add('datatype_child_over_model_parent',[('audit.FirstRuleDatatypeChildBase',None,"Model returns k::Model: name=VALUE;\nterminal VALUE: 'a';"),('audit.FirstRuleDatatypeChildChild','audit.FirstRuleDatatypeChildBase',"Text returns ecore::EString: 'b';")],'Datatype-first child reaches an object parser in its used grammar.')
add('fragment_class_first',[('audit.FirstRuleClassFragment',None,"fragment First returns k::Model: name=VALUE;\nModel returns k::Model: First;\nterminal VALUE: 'a';")],'First parser rule is an explicitly class-typed fragment.')
add('fragment_untyped_first',[('audit.FirstRuleUntypedFragment',None,"fragment First: 'a';\nModel returns k::Model: name=VALUE;\nterminal VALUE: 'a';")],'First parser fragment has no authored return type; resolved classifier is observed, not assumed.')
add('terminal_first_three_level_object_closure',[('audit.FirstRuleDeepObjectBase',None,"Model returns k::Model: name=VALUE;\nterminal VALUE: 'a';"),('audit.FirstRuleDeepObjectMiddle','audit.FirstRuleDeepObjectBase',''),('audit.FirstRuleDeepObjectChild','audit.FirstRuleDeepObjectMiddle',"terminal OTHER: 'b';")],'Terminal-first child reaches an object parser only through a three-level used-grammar chain.')
add('datatype_first_three_level_scalar_closure',[('audit.FirstRuleDeepScalarBase',None,"Text returns ecore::EString: 'a';"),('audit.FirstRuleDeepScalarMiddle','audit.FirstRuleDeepScalarBase',"terminal OTHER: 'b';"),('audit.FirstRuleDeepScalarChild','audit.FirstRuleDeepScalarMiddle',"More returns ecore::EString: 'c';")],'Datatype-first child through terminal and datatype-only ancestors has no non-datatype parser in its used closure.')
add('enum_before_local_model',[('audit.FirstRuleEnumBeforeModel',None,"enum Choice returns k::Choice: A='a' | B='b';\nModel returns k::Model: name=VALUE;\nterminal VALUE: 'a' | 'b';")],'True EEnum rule precedes a class-returning parser.',True)
add('enum_library_model_child',[('audit.FirstRuleEnumLibraryBase',None,"enum Choice returns k::Choice: A='a' | B='b';"),('audit.FirstRuleEnumLibraryChild','audit.FirstRuleEnumLibraryBase',"Model returns k::Model: name=VALUE;\nterminal VALUE: 'a' | 'b';")],'True enum-only library consumed by model-first child.',True)
add('same_name_class_first',[('audit.FirstRuleIdentityClass',None,"Model returns k::EString: name=VALUE;\nterminal VALUE: 'a';")],'First parser returns EClass EString in the separately authored package, not Ecore EDataType EString.',True)
add('same_name_datatype_first',[('audit.FirstRuleIdentityDatatype',None,"Text returns ecore::EString: 'a';\nModel returns k::EString: name=Text;")],'First parser returns Ecore EDataType EString while later parser returns distinct EClass EString.',True)

for directory, ids in [(UNTIL/'inputs',{'inherited_operand_override','explicit_parent_operand'}),(UNTIL/'inheritance-valid-inputs',{'valid_inherited_operand_override','valid_explicit_parent_operand'})]:
 for c in json.loads((directory/'cases.json').read_text(encoding='utf-8')):
  if c['id']in ids:
   shutil.copytree(directory/c['id'],HERE/'inputs'/c['id']);cases.append(c)
save(HERE/'inputs/cases.json',cases)
save(HERE/'draft-plan.json',{'graphs':len(cases),'new_original_provider_controls':12,'new_separate_provider_controls':4,'unchanged_original_graphs':4,'planned_inputs':sum(len(c['inputs'])for c in cases),'ecore_original_sha256':hashlib.sha256(original).hexdigest(),'ecore_separate_sha256':hashlib.sha256(new).hexdigest(),'inputs_not_frozen_yet':True})
print(json.dumps({'graphs':len(cases),'ids':[c['id']for c in cases]}))
