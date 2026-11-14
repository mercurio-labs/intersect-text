// Host verification only: compare real native observations with fresh WASM instances.
import assert from 'node:assert/strict';
import {readFile,readdir} from 'node:fs/promises';
import {resolve} from 'node:path';
const [wasmPath,observationsPath]=process.argv.slice(2);
assert.ok(wasmPath&&observationsPath,'Pass the WASM file and native observation directory');
const module=await WebAssembly.compile(await readFile(wasmPath));
assert.deepEqual(WebAssembly.Module.imports(module),[]);
async function runtime(){
 const {exports:e}=await WebAssembly.instantiate(module,{});
 return request=>{const bytes=new TextEncoder().encode(JSON.stringify(request)),p=e.input_buffer(bytes.length);assert.ok(p);new Uint8Array(e.memory.buffer,p,bytes.length).set(bytes);const length=e.execute();return JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer,e.output_buffer(),length)));};
}
let count=0;
for(const name of (await readdir(observationsPath)).filter(n=>n.endsWith('.json')).sort()){
 const observation=JSON.parse(await readFile(resolve(observationsPath,name),'utf8'));
 const invoke=await runtime(),{inference}=observation;
 assert.deepEqual(invoke({method:'compile_inferred',inference}),observation.legacy,name);
 assert.deepEqual(invoke({method:'compile_inferred_with_origins',inference}),observation.detailed,name);
 assert.deepEqual(invoke({method:'compile_inferred_with_origins',inference,diagnostic_origins:[{kind:'source',source:{uri:'memory:forged',sha256:'0'.repeat(64)}}]}),observation.detailed,name);
 count++;
}
assert.equal(count,8,'All declared native ownership cases must have observations');
const transport=JSON.parse(await readFile(resolve(observationsPath,'..','diagnostic-identities-legacy-transport.json'),'utf8'));
assert.deepEqual((await runtime())(transport.request),{error:transport.error},'Legacy archive error strings must retain four-field diagnostics');
console.log(`${count} complete native/fresh-WASM diagnostic comparisons and the legacy archive error-string control passed, including forged host metadata; zero WASM imports.`);
