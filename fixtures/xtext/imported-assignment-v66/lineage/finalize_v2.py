from pathlib import Path
import hashlib,json,re,zipfile
H=Path(__file__).resolve().parent;R=Path('C:/Users/MarkPetrotta/.codex/worktrees/rcore-xtext-support/intersect')
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def pin(p):return {'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p)}
def load(p):return json.loads(p.read_text(encoding='utf-8'))
def save(p,x):assert not p.exists(),p;p.write_text(json.dumps(x,indent=2,ensure_ascii=True)+'\n',encoding='utf-8',newline='\n')
cases=load(H/'inputs/cases.json');pins=load(H/'inputs/pins.json')
for n,h in pins.items():assert sha(H/'inputs'/n)==h,n
assert len({(H/'inputs'/c['id']/'audit.ImportedUnique.xtext').read_bytes()for c in cases})==1
ecores=[(H/'inputs'/c['id']/'imported.ecore').read_bytes()for c in cases]
assert len({re.sub(b' unique="(?:true|false)"',b'',b)for b in ecores})==1
native=load(H/'native-record.json');replay=load(H/'native-replay.json');assert native['transcript']==replay['transcript']
assert len(native['transcript'])==22
for i,r in enumerate(native['cases']):
 assert r['id']==cases[i]['id'];assert r['migration']['report']['converted']['text']==r['selection']['compilation']['bundle']['sources'][0]['text']
 assert r['compile']['ok']==(i==2)
 if i<2:assert r['compile']['diagnostics']==[{'end':242,'message':'Grammar assignment conflicts with the imported feature','source':cases[i]['sources'][0]['source_id']['uri'],'start':224}]
 else:
  for x in r['inputs']:assert x['direct']==x['archive']
main=load(H/'oracle-record/observations.json');fresh=load(H/'oracle-replay/observations.json');terminal=load(H/'oracle-terminal.json');assert terminal['state']=='COMPLETE_RECORD_AND_FRESH_REPLAY_EQUAL'
for lane in ['record','replay']:
 root=H/('oracle-'+lane)
 for n,h in load(root/'artifacts-sha256.json').items():assert sha(root/n)==h,(lane,n)
add=load(H/'validation-addendum-v2/record.json');assert add==load(H/'validation-addendum-v2/replay.json');assert len(add)==3
summary=[]
for i,(c,r,a,n)in enumerate(zip(cases,main,add,native['cases'])):
 assert c['id']==r['id']==a['id']==n['id']
 assert r['generation']=={'exit_code':0,'source_unchanged':True};assert r['compilation']=={'exit_code':0}
 assert all(not x['errors']for x in r['metadata']['observation']['syntax'])
 assert all(not x['errors']for x in r['metadata']['observation']['linking'])
 assert all(not x['issues']for x in r['metadata']['observation']['validation'])
 runtime=[]
 for j,(x,y)in enumerate(zip(r['runtime'],a['runtime'])):
  assert x['text']==y['text']==c['inputs'][j];u=y['observation'];assert u['feature']['unique']==(i!=2)
  assert u['feature']['containment']is False and u['feature']['lower']==1 and u['feature']['upper']==-1
  assert u['direct_list_probe']['first_add_changed']is True and u['direct_list_probe']['second_same_target_add_changed']is False and u['direct_list_probe']['size']==1
  refs=u['resource']['model_after_resolving_observation'][0]['values']['refs']
  if j<4:
   assert not u['resource']['errors']and not u['xtext_validation']['issues']
   assert [v['object']for v in refs]==[[1,2],[1,1],[2,1,2],[1]][j]
  else:assert any(v['severity']=='ERROR'for v in u['xtext_validation']['issues'])
  run={'text':x['text'],'main_resource_load_accepted_before_lazy_model_read':x['observation']['resource']['accepted'],'linked_resource_errors':u['resource']['errors'],'xtext_validation':u['xtext_validation'],'emf_severity':u['emf_validation']['severity'],'model_refs':refs,'generated_feature':u['feature'],'generated_list_probe':u['direct_list_probe'],'native':None}
  if i==2:
   v=n['inputs'][j]['direct'];run['native']={'diagnostics':v.get('diagnostics'),'error':v.get('error'),'model_refs':None if not v.get('model',{}).get('elements') else v['model']['elements'][0]['values'].get('refs'),'direct_equals_source_verified_archive':v==n['inputs'][j]['archive']}
  runtime.append(run)
 summary.append({'id':c['id'],'source':c['sources'][0]['source_id'],'original_ecore_sha256':sha(H/'inputs'/c['id']/'imported.ecore'),'grammar_metadata_clean':True,'generation_exit':0,'javac_exit':0,'actual_runtime':6,'fully_linked_no_Xtext_ERROR':4,'native_compile_ok':n['compile']['ok'],'native_diagnostics':n['compile'].get('diagnostics'),'runtime':runtime})
stack=load(H/'inputs/stack.json');archive=Path('C:/Users/MarkPetrotta/.m2/repository')/stack['source_archive']['path'];assert sha(archive)==stack['source_archive']['sha256']
entry='org/eclipse/xtext/xtext/ecoreInference/EClassifierInfo.java';raw=zipfile.ZipFile(archive).read(entry);d=H/'source-evidence';d.mkdir(exist_ok=False);(d/'EClassifierInfo.java.raw').write_bytes(raw)
text=raw.decode('latin-1').splitlines();save(d/'upstream-predicate.json',{'source_archive':pin(archive),'entry':entry,'raw_entry':pin(d/'EClassifierInfo.java.raw'),'decoding_note':'UTF-8 inspection failed at non-ASCII source byte0xf6; raw bytes preserved. Latin-1 inspection used only to extract the ASCII predicate/comment lines without changing originals.','imported_compatibility_lines_195_267':[{'line':i+1,'text':text[i]}for i in range(194,267)],'new_feature_creation_lines_440_457':[{'line':i+1,'text':text[i]}for i in range(439,457)]})
actual_files=['crates/intersect-xtext/src/inference.rs','crates/intersect-xtext/src/ordinary.rs','crates/intersect-model/src/definitions/ecore_document.rs','crates/intersect-model/src/definitions/ecore_migration.rs','crates/intersect-model/src/core/storage/collection.rs','crates/intersect-model/src/core/storage/feature_shape.rs']
sourcepins=[pin(R/p)for p in actual_files]
allpins=[{'relative':p.relative_to(H).as_posix(),'bytes':p.stat().st_size,'sha256':sha(p)}for p in sorted(H.rglob('*'))if p.is_file()]
save(H/'final-before-review.json',{'version':1,'state':'FROZEN_BEFORE_ONLY_NO_PRODUCTION_CHANGE','preexecution':pin(H/'preexecution.json'),'native_terminal':pin(H/'native-terminal.json'),'oracle_terminal':pin(H/'oracle-terminal.json'),'validation_addendum_terminal':pin(H/'validation-addendum-v2/terminal.json'),'actual_current_sources':sourcepins,'summary':summary,'proof_counts':{'graphs':3,'probes_per_graph':6,'full_oracle_runtime_per_run':18,'full_oracle_fresh_runs':2,'validation_addendum_per_run':18,'validation_fresh_runs':2,'native_complete_calls_per_run':22,'native_fresh_zero_import_instances':44,'native_compiled_graphs':1,'native_direct_and_archive_evaluations':12},'findings':['The imported unique mismatch alone is established by otherwise byte-identical minimal Ecore inputs: omitted/true reject, false passes in immutable normal34e3 NativeV64. Actual imported flag/report/source identities are retained unchanged.','Pinned EClassifierInfo.containsCompatibleFeature checks mutable cardinality/type/reference-containment/container constraints, not uniqueness. Its new-feature creation default setsUnique separately. This supports separating imported compatibility from inferred feature creation rather than repairing imported flags.','Both unique=true and false generated provider metadata are correct. Direct generated EList duplicate add returns false/size1 for ALL3. The code generator uses EObjectResolvingEList in each. Do not claim direct nonunique list insertion equivalence.','All3 linked resource observations preserve repeated resolved references [1,1] and [2,1,2] with no Xtext validation ERROR or EMF diagnostic severity. Preserve these pinned proxy/list observations; they do not authorize dropping unique flags or disabling native shared uniqueness.','Main unchanged FullRuntime snapshots resource errors before RuntimeOracle.model triggers lazy crossref resolution. Its raw accepted=true for unresolved missing is retained as pre-link load status, not full semantic acceptance. New additive validator observes the later linking ERROR. Named IParser model refs are empty because it does not perform resource linking.','Explicitfalse native direct/source-verified archive models preserve duplicate references; true/default native package/runtime remain unexecuted because source compile rejected. No prediction of their native duplicate behavior after a future admission change.'],'next_plan':['Review this complete immutable BEFORE before authorizing any source predicate change.','Use a new explicit profile after current pending profile work if compatibility policy requires it; preserve all old profiles/default inference responses.','Keep imported unique flags and native shared storage/validation authority. Separate imported feature assignment compatibility from new inferred feature uniqueness defaults; do not mutate same_shape globally.','Add explicit wrong type/reference-kind/containment/cardinality/changeable/container guards before claiming general imported compatibility. This cohort proves only the3reference-uniqueness cells.','Re-run unchanged whole pinned KerML correct ordinary frontend/contracts/budgets after a justified bounded fix to expose the next actual blocker. No limits increase or source repair.','Duplicate linked-model behavior versus native source uniqueness needs an explicit decision and qualification; do not silently fit proxy-resolution quirks or claim full Xtext/EMF/model equivalence.'],'preserved_failures':['Addendum v1 javac failed wrong static helper qualifier; v2 corrected the verification helper only.','Inline v2 preparation command failed shell/Python quoting before writing files; subsequent missing-file run had no source effect.','UTF-8 inspection of upstream source entry failed; raw source bytes retained separately.','Finalizer v1 assumed every native error model had an element; malformed syntax produced an empty model. v2 handles absent/empty model in report only, without changing any captured result.','Original stale low-level KerML guard failure, original pinned source graphs and original profiles remain unchanged.'],'scope_limits':['Verification-only Java; production Java remains absent.','Native Ecore migration/report is a bounded explicit source adapter; no native Ecore runtime award.','No actual R source/index/docs/public/dist mutation or Cargo build.','No wrong-shape/imported-attribute/containment-unique/general enum/default/covariance qualification from3cells.','No full Xtext, KerML, SysML or S09 gate closure.'],'all_frozen_files':allpins})
print(json.dumps(pin(H/'final-before-review.json')))
