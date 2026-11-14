import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const path = resolve(process.argv[2] || 'target/xtext-only/wasm32-unknown-unknown/debug/intersect_text_only_consumer.wasm');
const module = await WebAssembly.compile(await readFile(path));
assert.deepEqual(WebAssembly.Module.imports(module), [], 'Xtext consumer requires no Java, Toolkit or host imports');
for (let fresh = 0; fresh < 2; fresh++) {
  const instance = await WebAssembly.instantiate(module, {});
  assert.equal(instance.exports.verify_ordinary_boundary(), 0, 'Ordinary compilation/linking/generation/history controls failed');
  assert.equal(instance.exports.verify_inference_boundary(), 0, 'Inference/source replay/recovery controls failed');
  assert.equal(instance.exports.verify_profiles_boundary(), 0, 'Profile admission/execution/wire identity controls failed');
}
console.log('Two fresh zero-import WASM instances passed ordinary, inference and selected-profile public API controls.');
