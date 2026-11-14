// Real JSON host ABI: inferred sources survive independent WASM instances.
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
const root=fileURLToPath(new URL('../',import.meta.url));
const build=spawnSync('cargo',['build','-p','intersect-web','--target','wasm32-unknown-unknown','--locked'],{cwd:root,stdio:'inherit'});
assert.equal(build.status,0,build.error?.message);
const target=resolve(root,process.env.CARGO_TARGET_DIR||'target');
const module=await WebAssembly.compile(readFileSync(resolve(target,'wasm32-unknown-unknown/debug/intersect_web.wasm')));
assert.deepEqual(WebAssembly.Module.imports(module),[]);
async function runtime(){
  const {exports:e}=await WebAssembly.instantiate(module,{});
  return request=>{
    const bytes=new TextEncoder().encode(JSON.stringify(request));
    const p=e.input_buffer(bytes.length);assert.ok(p);
    new Uint8Array(e.memory.buffer,p,bytes.length).set(bytes);
    const length=e.execute();
    const result=JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer,e.output_buffer(),length)));
    if(result.error)throw new Error(result.error);
    return result;
  };
}
const fixture=resolve(root,'fixtures/xtext/inference-fragment-runtime');
const cases=JSON.parse(readFileSync(resolve(fixture,'cases.json'),'utf8'));
const observations=JSON.parse(readFileSync(resolve(fixture,'observations.json'),'utf8'));
function inputs(c){
  const ids=new Map();
  const sources=c.sources.map(s=>{
    const id={uri:`memory:host-inference/${c.id}/${s.id}.xtext`,sha256:createHash('sha256').update(s.source).digest('hex')};
    const dependencies=s.dependencies.map(n=>{assert.ok(ids.has(n));return ids.get(n);});
    ids.set(s.id,id);return {id,kind:'xtext',text:s.source,dependencies};
  });
  return {version:1,bundle:{version:1,sources,namespaces:[]},grammar:ids.get(c.entry)};
}
const source=text=>[{uri:'memory:host-model.test',text}];
function projection(pkg,model){
  const uri=key=>{const d=pkg.schema.declarations[key];return `${d.namespace}#//${d.name}`;};
  const value=v=>{
    if(typeof v==='string'){
      const i=model.elements.findIndex(e=>e.id===v);
      if(i>=0)return {object:i,class:uri(model.elements[i].class)};
    }
    return Array.isArray(v)?v.map(value):v;
  };
  return model.elements.map(e=>({class:uri(e.class),values:Object.fromEntries(Object.entries(e.values).map(([k,v])=>[k,value(v)]))}));
}
let successes=0,rejections=0,first;
for(const c of cases){
  const invoke=await runtime();
  const input=inputs(c);
  const compiled=invoke({method:'compile_inferred',inference:input});
  assert.equal(compiled.ok,true,JSON.stringify(compiled));
  const pkg=JSON.parse(JSON.stringify(compiled.package));
  assert.deepEqual(pkg.xtext_inference,input);
  assert.equal(pkg.compilation,undefined);
  const restored=await runtime();
  for(const run of observations.runs.filter(r=>r.case===c.id&&r.entry==='resource')){
    const result=restored({method:'evaluate',package:pkg,sources:source(run.text)});
    assert.equal(result.diagnostics.length===0,run.accepted,`${c.id}: ${JSON.stringify(result.diagnostics)}`);
    if(run.accepted){successes++;assert.deepEqual(projection(pkg,result.model),run.model);}
    else{rejections++;assert.deepEqual(result.model.elements,[]);}
    if(c.id==='typed_basic')first={pkg,input,result};
  }
}
assert.deepEqual([cases.length,successes,rejections],[10,7,4]);
const invoke=await runtime();
const evaluate=pkg=>invoke({method:'evaluate',package:pkg,sources:source('alpha')});
const forged=structuredClone(first.pkg);forged.grammar=null;forged.schema.uri='urn:forged';forged.sources={};
assert.deepEqual(evaluate(forged),first.result);
const bads=[];
for(const mutation of [
  p=>{p.xtext_inference.version=99;},
  p=>{p.xtext_inference.bundle.sources[0].text+=' ';},
  p=>{p.xtext_inference.grammar.sha256='0'.repeat(64);},
  p=>{p.xtext_inference.schema={};},
  p=>{p.source_policy.xtext_bytes=256001;},
  p=>{delete p.xtext_inference;},
]){const p=structuredClone(first.pkg);mutation(p);bads.push(p);}
for(const bad of bads){assert.throws(()=>evaluate(bad));assert.deepEqual(evaluate(first.pkg),first.result);}
const invalid=structuredClone(first.input);invalid.version=99;
assert.equal(invoke({method:'compile_inferred',inference:invalid}).ok,false);
for(const [name,value] of [['conversions',[]],['bindings',{}],['xtext_profile','native.v56']]){
  assert.throws(()=>invoke({method:'compile_inferred',inference:first.input,[name]:value}));
}
assert.deepEqual(evaluate(first.pkg),first.result);
const {pkg,result}=first;
const query=invoke({method:'query/execute',package:pkg,model:result.model,query:{schema_uri:pkg.schema.uri,schema_revision:pkg.schema.revision,from:result.model.elements[0].class,select:[{name:'name',expression:{op:'field',path:'name'}}],limit:10}});
assert.deepEqual(query.rows,[['alpha']]);
const exported=invoke({method:'model/export',package:pkg,model:result.model,format:'xmi'});
const imported=invoke({method:'model/import',package:pkg,format:'xmi',resources:exported.resources});
assert.deepEqual(imported.diagnostics,[]);
assert.deepEqual(projection(pkg,imported.model),projection(pkg,result.model));
console.log('Ten exact-source inferred graphs, seven oracle models, four rejections, fresh-instance replay, source/policy failures, shared query and XMI services passed through zero-import WASM JSON host; full X03/Toolkit/archive adoption remains open.');
