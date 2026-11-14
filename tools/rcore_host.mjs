// Shared handwritten native/WASM test adapter. No model or language semantics.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {resolve} from 'node:path';
export async function createRcoreHost(root,nativeOnly=false) {
  const target=resolve(root,process.env.CARGO_TARGET_DIR || 'target');
  function build(args) {
    const result=spawnSync('cargo',args,{cwd:root,stdio:'inherit'});
    assert.equal(result.status,0,result.error?.message);
  }
  build(['build','-p','intersect-web','--example','rcore_behavior_parse','--locked']);
  if (!nativeOnly) build(['build','-p','intersect-web','--target','wasm32-unknown-unknown','--locked']);
  const executable=resolve(target,`debug/examples/rcore_behavior_parse${process.platform==='win32'?'.exe':''}`);
  let wasm;
  if (!nativeOnly) {
    const {instance:{exports:e}}=await WebAssembly.instantiate(readFileSync(resolve(target,'wasm32-unknown-unknown/debug/intersect_web.wasm')),{});
    wasm=request=>{
      const bytes=new TextEncoder().encode(JSON.stringify(request));
      const pointer=e.input_buffer(bytes.length);assert.ok(pointer,'WASM input allowance');
      new Uint8Array(e.memory.buffer,pointer,bytes.length).set(bytes);
      const length=e.execute();
      const response=JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer,e.output_buffer(),length)));
      assert.equal(response.error,undefined,response.error);return response;
    };
  }
  let comparisons=0;
  return {get comparisons(){return comparisons;},target,invoke(request){
    const native=spawnSync(executable,[],{input:JSON.stringify(request),encoding:'utf8',maxBuffer:20_000_000});
    assert.equal(native.status,0,native.stderr);
    const response=JSON.parse(native.stdout);
    if (wasm) {assert.deepEqual(wasm(request),response,`native/WASM ${request.method}`);comparisons++;}
    return response;
  }};
}

export {executionLimits} from '../fixtures/rcore/execution-limits.mjs';
