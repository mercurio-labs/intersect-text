import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const path = resolve(process.argv[2] || 'target/model-only/wasm32-unknown-unknown/debug/intersect_model_only_consumer.wasm');
const module = await WebAssembly.compile(await readFile(path));
assert.deepEqual(WebAssembly.Module.imports(module), [], 'Model-only WASM needs no host or Xtext imports');
const instance = await WebAssembly.instantiate(module, {});
assert.equal(instance.exports.verify_model_boundary(), 0, 'Model-only Rcore controls failed');
console.log('Model-only WASM construction, source getters, execution, rejection and recovery passed.');
