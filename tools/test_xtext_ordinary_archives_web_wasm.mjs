// Actual ordinary JSON host and deterministic ZIP transport; independent frozen models.
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import {packageArchive} from '../toolkit/src/package-export.js';
import {unpackLanguageArchive} from '../toolkit/src/package-import.js';
const root=fileURLToPath(new URL('../',import.meta.url));
const built=spawnSync('cargo',['build','-p','intersect-web','--target','wasm32-unknown-unknown','--locked'],{cwd:root,stdio:'inherit'});
assert.equal(built.status,0,built.error?.message);
const target=resolve(root,process.env.CARGO_TARGET_DIR||'target');
const module=await WebAssembly.compile(readFileSync(resolve(target,'wasm32-unknown-unknown/debug/intersect_web.wasm')));
assert.deepEqual(WebAssembly.Module.imports(module),[]);
async function runtime(){const {exports:e}=await WebAssembly.instantiate(module,{});return request=>{const b=new TextEncoder().encode(JSON.stringify(request));const p=e.input_buffer(b.length);assert.ok(p);new Uint8Array(e.memory.buffer,p,b.length).set(b);const n=e.execute();const r=JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer,e.output_buffer(),n)));if(r.error)throw new Error(r.error);return r;};}
const hash=text=>createHash('sha256').update(text).digest('hex');
const fixture=resolve(root,'fixtures/xtext/imported-parameters');
const cases=JSON.parse(readFileSync(resolve(fixture,'cases.json'),'utf8')).cases;
const expected=JSON.parse(readFileSync(resolve(fixture,'observations.json'),'utf8'));
function original(c){
 const models=c.metamodels.map(m=>({id:{uri:m.uri,sha256:hash(m.source)},kind:'ecore',text:m.source,dependencies:[]}));
 const modelIds=models.map(m=>m.id);for(const m of models)m.dependencies=modelIds.filter(id=>id!==m.id);
 const ids=new Map(),sources=[...models];
 for(const g of c.sources){assert.equal(hash(g.source),g.sha256);const id={uri:g.uri,sha256:g.sha256};sources.push({id,kind:'xtext',text:g.source,dependencies:[...modelIds,...g.dependencies.map(n=>ids.get(n))]});ids.set(g.id,id);}
 const text='rcore 1; package ordinary version "1";',behavior={id:{uri:`memory:/ordinary-parameters/${c.id}/behavior.rcore`,sha256:hash(text)},kind:'rcore_behavior',text,dependencies:modelIds};sources.push(behavior);
 return{bundle:{version:1,sources,namespaces:c.metamodels.map((m,i)=>({namespace:m.namespace,source:modelIds[i]}))},ecore:modelIds[0],namespace:c.metamodels[0].namespace,xtext:ids.get(c.entry),rcore_behavior:behavior.id,materialized_types:[]};
}
function projection(pkg,model){const uri=k=>{const d=pkg.schema.declarations[k];return `${d.namespace}#//${d.name}`;};const value=v=>{if(typeof v==='string'){const i=model.elements.findIndex(e=>e.id===v);if(i>=0)return{object:i,class:uri(model.elements[i].class)};}return Array.isArray(v)?v.map(value):v;};return model.elements.map(e=>({class:uri(e.class),values:Object.fromEntries(Object.entries(e.values).map(([k,v])=>[k,value(v)]))}));}
const metadata={package_id:'ordinary.archive',release:'1',display_name:'Ordinary parameter archive',description:''};
const profile='intersect.xtext.native.v57';
const counts=[0,0,0,0,0];
for(const c of cases){
 const invoke=await runtime(),input=original(c);
 const migrated=invoke({method:'ecore/migrateCompilation',compilation:input,targets:c.metamodels.map((m,i)=>({source:input.bundle.sources[i].id,uri:m.uri.replace('.ecore','.rcore')}))});
 assert.equal(migrated.ok,true,c.id);const compilation=migrated.compilation;
 const row=expected.metadata.find(m=>m.id===c.id);
 let distribution;
 try{distribution=invoke({method:'package/export',compilation,xtext_profile:profile,metadata});}
 catch(error){assert.equal(row.all_sources_accepted,false,`${c.id}: ${error}`);counts[1]++;continue;}
 assert.equal(row.all_sources_accepted,true,c.id);counts[0]++;
 assert.deepEqual(distribution.package.compilation,compilation);
 assert.equal(distribution.package.xtext_inference,undefined);
 const direct=invoke({method:'compile_bundle',compilation,xtext_profile:profile});assert.equal(direct.ok,true);assert.deepEqual(direct.package,distribution.package);
 for(const python of [false,true]){
  const bytes=await packageArchive(distribution,python);assert.deepEqual(await packageArchive(distribution,python),bytes);
  const files=unpackLanguageArchive(bytes);assert.deepEqual(JSON.parse(files['definitions/bundle.json']),compilation);
  if(distribution.manifest.compiler.name==='intersect-text')assert.match(files['README.txt'],/intersect_text::distribution::import_files/);
  const consume=await runtime(),loaded=consume({method:'package/import',files});assert.deepEqual(loaded.package,distribution.package);
  for(const run of expected.runs.filter(r=>r.case===c.id&&r.entry==='resource')){
   const result=consume({method:'evaluate',package:loaded.package,sources:[{uri:'memory:ordinary-zip.model',text:run.text}]});
   const parsed=run.exception===null&&run.errors.length===0;assert.equal(result.diagnostics.length===0,parsed,`${c.id}: ${run.text}`);
   if(run.accepted){counts[2]++;assert.deepEqual(projection(loaded.package,result.model),run.model);}
   else if(parsed){counts[4]++;assert.deepEqual(result.model.elements,[]);}
   else{counts[3]++;assert.deepEqual(result.model.elements,[]);}
  }
 }
}
assert.deepEqual(counts,[31,17,68,84,2]);
console.log('Ordinary actual ZIP -> fresh zero-import JSON WASM: 31 accepted/17 rejected source graphs; both Python forms; 68 rooted models, 84 atomic rejections, two error-free empty resources; deterministic bytes and original ordinary authority. Full worker/project/restart and provider qualification remain open.');
