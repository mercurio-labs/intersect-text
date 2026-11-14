import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const path = resolve(process.argv[2] || 'target/wasm32-unknown-unknown/debug/intersect_resource_lifecycle_tests.wasm');
const module = await WebAssembly.compile(await readFile(path));
assert.deepEqual(WebAssembly.Module.imports(module), [], 'Resource lifecycle WASM requires no host imports');
const { exports } = await WebAssembly.instantiate(module, {});
for (const increment of ['f17', 'f18', 'f19', 'f20_residency', 'f20_cache', 'f20_cancellation', 'f20_kerml']) {
  assert.equal(exports[`verify_resource_${increment}`](), 0, `${increment} resource lifecycle controls failed`);
  console.log(`${increment}: portable resource lifecycle controls passed`);
}
