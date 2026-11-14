import fs from 'node:fs';import assert from 'node:assert/strict';import{createHash}from'node:crypto';
const at=n=>new URL(n,import.meta.url),read=n=>fs.readFileSync(at(n)),sha=b=>createHash('sha256').update(b).digest('hex'),write=(n,x)=>{assert(!fs.existsSync(at(n)));fs.writeFileSync(at(n),JSON.stringify(x,null,2)+'\n','utf8');};
const artifact='D:/dev/repos/mercurio-labs/intersect/target/s09-intermediate-payload-v22/public-numeric-controls-v1/actual-public-build-v2/intersect_web-34e3bd2da628d87f60be7fc2933419592411842dcc09eeef485df7085463a425.wasm';
const binary=fs.readFileSync(artifact);assert.equal(sha(binary),'34e3bd2da628d87f60be7fc2933419592411842dcc09eeef485df7085463a425');const module=await WebAssembly.compile(binary);assert.deepEqual(WebAssembly.Module.imports(module),[]);
const profile='intersect.xtext.native.v64',cases=JSON.parse(read('inputs/cases.json')),pins=JSON.parse(read('inputs/pins.json'));
function check(){for(const[n,h]of Object.entries(pins))assert.equal(sha(read('inputs/'+n)),h,n);}check();
write('native-preexecution.json',{version:1,artifact:{path:artifact,bytes:binary.length,sha256:sha(binary)},profile,producer_sha256:sha(fs.readFileSync(new URL(import.meta.url))),input_pins_sha256:sha(read('inputs/pins.json')),scope:'Complete fresh zero-import native migration/ordinary source compilation/archive/evaluation BEFORE. No Ecore runtime award; no source repair or predicate edit. Exact converted migration source selected unchanged only when actual migration reports it.'});
let instances=0;const transcript=[];
async function execute(request){const{exports:w}=await WebAssembly.instantiate(module,{});instances++;const b=Buffer.from(JSON.stringify(request),'utf8'),p=w.input_buffer(b.length);new Uint8Array(w.memory.buffer,p,b.length).set(b);const n=w.execute();return JSON.parse(Buffer.from(new Uint8Array(w.memory.buffer,w.output_buffer(),n)).toString('utf8'));}
async function call(request){const result=await execute(request);transcript.push({request,result});return result;}
const rows=[];
for(const c of cases){
 const ecore=read('inputs/'+c.id+'/imported.ecore'),sourceUri='memory:'+c.id+'/imported.ecore',targetUri='memory:'+c.id+'/model.rcore';
 const migration=await call({method:'ecore/migrate',uri:sourceUri,text:ecore.toString('utf8'),target_uri:targetUri});
 const row={id:c.id,original_ecore:{uri:sourceUri,sha256:sha(ecore)},migration,selection:null,compile:null,archive:null,inputs:[]};rows.push(row);
 if(!migration.report?.converted){row.not_executed='Actual migration did not produce converted source; all results retained, no draft/source repair.';continue;}
 const structure=migration.report.converted;assert.equal(sha(Buffer.from(structure.text)),structure.id.sha256);assert.equal(structure.id.uri,targetUri);
 const s=c.sources[0],text=read('inputs/'+c.id+'/'+s.file).toString('utf8');assert.equal(sha(Buffer.from(text)),s.source_id.sha256);
 const xtext={id:s.source_id,kind:'xtext',text,dependencies:[structure.id]},behaviorText='rcore 1;\npackage imported_unique_behavior version "1";\n',behavior={id:{uri:'memory:'+c.id+'/behavior.rcore',sha256:sha(Buffer.from(behaviorText))},kind:'rcore_behavior',text:behaviorText,dependencies:[structure.id]};
 const compilation={bundle:{version:1,sources:[structure,xtext,behavior],namespaces:[]},ecore:structure.id,namespace:'urn:intersect:xtext:imported-unique:1',xtext:xtext.id,rcore_behavior:behavior.id};
 row.selection={policy:'Select actual report.converted unchanged and retain all original Ecore/report/correspondence/unsupported items separately; no native Ecore claim.',compilation};
 row.compile=await call({method:'compile_bundle',compilation,xtext_profile:profile});
 const exported=await call({method:'package/export',compilation,xtext_profile:profile,metadata:{package_id:'imported-unique-before.'+c.id,release:'1',display_name:c.id,description:'Immutable original V64 BEFORE'}});row.archive={export:exported};
 let imported;
 if(exported.manifest){row.archive.import=await call({method:'package/import',files:{...exported.files,'manifest.json':JSON.stringify(exported.manifest)}});imported=row.archive.import.package;}
 for(const text of c.inputs){const run={text};if(row.compile.ok){const sources=[{uri:'memory:'+c.id+'/model.unique',text}];run.direct=await call({method:'evaluate',package:row.compile.package,sources});if(imported)run.archive=await call({method:'evaluate',package:imported,sources});}else run.not_executed='Original V64 source compile failed; no successful native resource/model claim.';row.inputs.push(run);}
 console.log(JSON.stringify({id:c.id,migration_ok:migration.ok,compile_ok:row.compile.ok,diagnostics:row.compile.diagnostics,evaluations:row.inputs.filter(x=>x.direct).length}));
}
write('native-record.json',{version:1,profile,artifact_sha256:sha(binary),fresh_zero_import_instances:instances,cases:rows,transcript});
const replay=[];for(const row of transcript){const result=await execute(row.request);assert.deepEqual(result,row.result);replay.push({request:row.request,result});}
write('native-replay.json',{version:1,profile,artifact_sha256:sha(binary),transcript:replay});check();
write('native-terminal.json',{state:'COMPLETE_RECORD_AND_FRESH_ZERO_IMPORT_REPLAY_EXACT',calls_per_run:transcript.length,total_fresh_instances:instances,compiled:rows.filter(x=>x.compile?.ok).length,evaluations:rows.reduce((n,r)=>n+r.inputs.filter(x=>x.direct).length,0),record_sha256:sha(read('native-record.json')),replay_sha256:sha(read('native-replay.json')),post_input_pins_equal:true});
