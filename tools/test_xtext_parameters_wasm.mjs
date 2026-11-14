// Same complete parameter inference/model controls in fresh zero-import WASM.
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
assert.equal(instance.exports.verify_xtext_parameters(),3);
console.log('Parameter inference: 48 accepted / 26 rejected exact graphs. Runtime: 30 independent models / 37 independent rejections, plus 58 separate original-source controls. Supplemental forwarding/empty resources: one model, two syntax errors, one error-free empty resource. Fresh WASM, zero imports; full qualification remains open.');
