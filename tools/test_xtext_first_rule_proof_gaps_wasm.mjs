// Verification-only actual zero-import WASM helper; explicit private artifact/output.
import fs from 'node:fs';import assert from 'node:assert/strict';import {createHash} from 'node:crypto';import {resolve} from 'node:path';
const sha=b=>createHash('sha256').update(b).digest('hex');
assert(process.env.INTERSECT_PROOF_GAP_WASM,'Select the verification-only WASM artifact explicitly');
assert(process.env.INTERSECT_PROOF_GAP_NATIVE,'Select shared native suite JSON explicitly');
assert(process.env.INTERSECT_PROOF_GAP_OUTPUT,'Select a NEW evidence directory explicitly');
const output=resolve(process.env.INTERSECT_PROOF_GAP_OUTPUT);assert(!fs.existsSync(output),'Never overwrite evidence');fs.mkdirSync(output,{recursive:true});
const fixture=new URL('../fixtures/xtext/contextual-first-rule-proof-gaps/',import.meta.url);
const first=JSON.parse(fs.readFileSync(new URL('native/helper/helper-record.json',fixture),'utf8'));
const native=JSON.parse(fs.readFileSync(process.env.INTERSECT_PROOF_GAP_NATIVE,'utf8'));assert.deepEqual(native.baseline,first);
const bytes=fs.readFileSync(process.env.INTERSECT_PROOF_GAP_WASM),module=await WebAssembly.compile(bytes);assert.deepEqual(WebAssembly.Module.imports(module),[]);
const observations=[];
for(let i=0;i<2;i++){
 const{exports:w}=await WebAssembly.instantiate(module,{});const length=w.run_controls();assert(length>0);const result=JSON.parse(Buffer.from(new Uint8Array(w.memory.buffer,w.output_buffer(),length)).toString('utf8'));
 assert(!result.error,JSON.stringify(result));assert.deepEqual(result.baseline,first);assert.deepEqual(result,native);assert.equal(result.coverage.length,5);
 observations.push(result);
}
const receipt={scope:'Verification-only helper baseline and shared native/WASM assertions. No public JSON endpoint, production/profile or package/runtime qualification.',wasm:{path:resolve(process.env.INTERSECT_PROOF_GAP_WASM),bytes:bytes.length,sha256:sha(bytes)},native:{path:resolve(process.env.INTERSECT_PROOF_GAP_NATIVE),sha256:sha(fs.readFileSync(process.env.INTERSECT_PROOF_GAP_NATIVE))},fresh_zero_import_instances:2,complete_helper_rows_per_instance:5,observations};
fs.writeFileSync(resolve(output,'observations.json'),JSON.stringify(receipt,null,2)+'\n','utf8');console.log(JSON.stringify({fresh_instances:2,complete_helper_rows_each:5,shared_source_authority_controls:true,sha256:sha(bytes)}));
