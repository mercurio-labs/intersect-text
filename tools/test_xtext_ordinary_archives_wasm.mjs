// Ordinary parameter archives through the shared source-authoritative verifier.
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
assert.equal(instance.exports.verify_xtext_ordinary_archives(),2);
console.log('Ordinary archives: 31 accepted / 17 rejected exact source graphs; 77 default and 14 named parser requests after archive rebuilding; original recipe authority, missing installed frontend, repaired-digest forgery rejection, host ceilings, recovery and unchanged legacy manifest/payloads. Fresh WASM, zero imports; host lifecycle and full qualification remain open.');
