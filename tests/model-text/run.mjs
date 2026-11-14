// Test glue only: all assertions and semantics run in the shared Rust controls.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
const path=process.argv[2];
assert.ok(path,'Pass the built intersect_model_text_tests.wasm path');
const {instance}=await WebAssembly.instantiate(await readFile(path),{});
const api=instance.exports;
const count=api.intersect_model_text_case_count();
assert.equal(count,33,'Frozen raw-WASM acceptance inventory changed');
for(let index=0;index<count;index++){
 const encoded=api.intersect_model_text_case_name(index);
 const pointer=Number(encoded&0xffffffffn),length=Number(encoded>>32n);
 const name=new TextDecoder().decode(new Uint8Array(api.memory.buffer,pointer,length));
 try { assert.equal(api.intersect_model_text_run_case(index),0); }
 catch(error){throw new Error(`Raw WASM control ${name} failed`,{cause:error});}
 process.stdout.write(`PASS ${name}\n`);
}
process.stdout.write(`${count} shared native/WASM controls passed.\n`);
