// Source-authoritative named entries through the shared native/WASM machine.
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
assert.equal(instance.exports.verify_xtext_named_entries(),4);
console.log('Named entries: original 14 requests (6 models / 8 rejections); supplemental 38 resource/entry requests and 25 direct scalar/object observations across 7 independently generated graphs; 2 original inferred named-entry rejections and 4 inferred counterparts; source authority, all six evaluation ceilings, rollback and package-bound setting selection. Fresh WASM, zero imports. Scalar-first compilation, ordinary archives and host lifecycle adoption remain open.');
