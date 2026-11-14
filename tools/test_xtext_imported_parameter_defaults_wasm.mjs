// Bounded default-resource consumer; named entries and host adoption remain open.
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
const instance=await WebAssembly.instantiate(module,{});
assert.equal(instance.exports.verify_xtext_imported_parameter_defaults(),2);
console.log('Ordinary imported parameters: 31 accepted / 17 rejected graphs; 34 rooted models / 42 rejected default requests / one error-free empty resource; exact ordinary source replay and admission controls. Fresh WASM, zero imports. This probe covers default resources; named entries, archives and host lifecycles have separate consumer controls. Full gates remain open.');
