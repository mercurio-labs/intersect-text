// Replays actual persistent public hosts; no production serializer or graph reader.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {join,resolve,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {loadControls,expandReferences} from './string_transport_controls.mjs';
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const options={};for(let i=2;i<process.argv.length;i+=2){const key=process.argv[i],value=process.argv[i+1];if(!['--wasm','--native','--out'].includes(key)||!value||Object.hasOwn(options,key))throw new Error('Use --wasm FILE [--native EXE] --out NEW_DIRECTORY');options[key]=value;}
if(!options['--wasm']||!options['--out'])throw new Error('Explicit WASM and new output directory required');
const out=resolve(options['--out']);mkdirSync(out);
const pin=p=>{const b=readFileSync(p);return {path:resolve(p),bytes:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const save=(name,value)=>writeFileSync(join(out,name),JSON.stringify(value,null,2)+'\n',{flag:'wx'});
const {groups,packed}=loadControls(root);assert.equal(groups.length,16);assert.equal(groups.reduce((n,g)=>n+g.requests.length,0),160);
assert.throws(()=>expandReferences({$ref:'../../other'},{safe:1}),/Unknown/);
assert.throws(()=>expandReferences({$ref:'safe',extra:true},{safe:1}),/augmented/);
const wasm=pin(options['--wasm']),native=options['--native']?pin(options['--native']):null;
const module=new WebAssembly.Module(readFileSync(wasm.path));assert.deepEqual(WebAssembly.Module.imports(module),[]);
const canonical=v=>Array.isArray(v)?v.map(canonical):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])])):v;
save('preexecution.json',{runner:pin(fileURLToPath(import.meta.url)),codec:pin(join(root,'tools/string_transport_controls.mjs')),controls:pin(join(root,'fixtures/rcore-string-transport/public-controls.json')),case_plan:pin(join(root,'fixtures/rcore-string-transport/plan.json')),source_native_plan_sha256:packed.source_native_plan_sha256,wasm,native,imports:[],node:process.version,scope:'Source-checked native example supplied explicitly; one persistent host per group, two fresh runs. Exact full outcomes, no response normalization. Not allocator/OS/cleanup evidence.'});
const encode=new TextEncoder(),decode=new TextDecoder('utf-8',{fatal:true}),runs=[];let current;
try{
  for(let replay=0;replay<2;replay++){
    const wasmRows=[],nativeRows=[];
    for(const g of groups){
      let nativeResponses;
      if(native){nativeResponses=JSON.parse(execFileSync(native.path,[],{input:JSON.stringify(canonical(g.requests)),encoding:'utf8',timeout:90000,maxBuffer:32*1024*1024}));assert.deepEqual(nativeResponses,g.expected);nativeRows.push({group:g.id,responses:nativeResponses});}
      const instance=new WebAssembly.Instance(module,{}),e=instance.exports;
      for(let index=0;index<g.requests.length;index++){
        const request=g.requests[index];current={replay,group:g.id,index,request};
        const bytes=encode.encode(JSON.stringify(canonical(request))),ptr=e.input_buffer(bytes.length);assert.notEqual(ptr,0);new Uint8Array(e.memory.buffer,ptr,bytes.length).set(bytes);
        const length=e.execute(),response=JSON.parse(decode.decode(new Uint8Array(e.memory.buffer,e.output_buffer(),length)));
        wasmRows.push({group:g.id,index,request,response});current.response=response;
        assert.deepEqual(response,g.expected[index]);if(nativeResponses)assert.deepEqual(response,nativeResponses[index]);
      }
    }
    const path=join(out,`wasm-${replay}.jsonl`);writeFileSync(path,wasmRows.map(v=>JSON.stringify(v)).join('\n')+'\n',{flag:'wx'});
    const row={replay,wasm_instances:16,responses:wasmRows.length,capture:pin(path)};
    if(native){const p=join(out,`native-${replay}.jsonl`);writeFileSync(p,nativeRows.map(v=>JSON.stringify(v)).join('\n')+'\n',{flag:'wx'});row.native_capture=pin(p);row.native_persistent_hosts=16;}
    runs.push(row);
  }
  assert.deepEqual(pin(wasm.path),wasm);if(native)assert.deepEqual(pin(native.path),native);
  save('receipt.json',{state:'PASS',runs,wasm,native,imports:[],preexecution:pin(join(out,'preexecution.json')),limits:['Exact public responses and logical admission/ownership limits; no native/WASM allocator equivalence','Native example wraps Result.Err exactly as public ABI error; actual outputs not otherwise normalized','Earlier input/context/other payload/JSON/structured clone/destructor work and full W1/W4/W5 remain open'],gate_closures:[]});
  console.log(JSON.stringify({receipt:pin(join(out,'receipt.json')),responses:320,persistent_wasm_hosts:32}));
}catch(error){save('failure.json',{current,error:{message:error.message,stack:error.stack}});throw error;}
