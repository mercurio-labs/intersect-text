// Guard omission and retained parameter controls in one shared, import-free WASM.
import {spawnSync} from 'node:child_process';
import {readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const root=fileURLToPath(new URL('../',import.meta.url));
const built=spawnSync('cargo',['build','-p','intersect-text-acceptance','--example','xtext_metamodel_inference_probe','--target','wasm32-unknown-unknown','--locked'],{cwd:root,stdio:'inherit'});
assert.equal(built.status,0,built.error?.message);
const target=resolve(root,process.env.CARGO_TARGET_DIR||'target');
const module=await WebAssembly.compile(readFileSync(resolve(target,'wasm32-unknown-unknown/debug/examples/xtext_metamodel_inference_probe.wasm')));
assert.deepEqual(WebAssembly.Module.imports(module),[]);
const {exports}=await WebAssembly.instantiate(module,{});
for(const [name,count]of [['verify_xtext_guard_sequences',2],['verify_xtext_parameters',3],['verify_xtext_imported_parameter_defaults',2],['verify_xtext_named_entries',4],['verify_xtext_ordinary_archives',2]]){
 assert.equal(exports[name](),count,name);
 console.log(`${name}: ${count} groups passed`);
}
console.log('135 independent guard resource observations and original parameter/entry/archive matrices passed in fresh WASM; full support gates remain open.');
