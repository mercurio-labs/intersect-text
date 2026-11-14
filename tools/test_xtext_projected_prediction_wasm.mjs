// Verification-only host; all parser/model assertions execute in shared Rust.
import {spawnSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const root=fileURLToPath(new URL('../',import.meta.url));
const build=spawnSync('cargo',['build','-p','intersect-text-acceptance','--example','projected_prediction_probe','--target','wasm32-unknown-unknown','--locked'],{cwd:root,stdio:'inherit'});
assert.equal(build.status,0,build.error?.message);
const target=resolve(root,process.env.CARGO_TARGET_DIR||'target');
const module=await WebAssembly.compile(readFileSync(resolve(target,'wasm32-unknown-unknown/debug/examples/projected_prediction_probe.wasm')));
assert.deepEqual(WebAssembly.Module.imports(module),[],'Prediction/model WASM needs no host imports');
const instance=await WebAssembly.instantiate(module,{});
assert.equal(instance.exports.verify_projected_prediction(),11);
console.log('Eleven projected/certificate/filtered/discovery prediction/model groups passed in real WASM.');
