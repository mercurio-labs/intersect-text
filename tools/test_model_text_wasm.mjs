import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const module = await WebAssembly.compile(readFileSync(process.argv[2]));
assert.deepEqual(WebAssembly.Module.imports(module), []);
const {exports: e} = await WebAssembly.instantiate(module, {});
const count = e.intersect_model_text_case_count();
assert.equal(count, 33);
for (let i = 0; i < count; ++i) {
  const packed = e.intersect_model_text_case_name(i);
  const pointer = Number(packed & 0xffffffffn);
  const length = Number(packed >> 32n);
  const name = new TextDecoder().decode(new Uint8Array(e.memory.buffer, pointer, length));
  assert.ok(name.length > 0);
  assert.equal(e.intersect_model_text_run_case(i), 0, name);
  console.log(`${i + 1}/${count}: ${name} passed`);
}
assert.equal(e.intersect_model_text_run_case(count), 1);
console.log('All 33 existing generation/edit/history controls passed in zero-import WASM.');
