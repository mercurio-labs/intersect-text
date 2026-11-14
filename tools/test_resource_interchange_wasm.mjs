import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
const binary = await readFile(resolve(process.argv[2] || 'target/wasm32-unknown-unknown/debug/intersect_web.wasm'));
const { instance: { exports: e } } = await WebAssembly.instantiate(binary, {});
function invoke(request) {
  const bytes = new TextEncoder().encode(JSON.stringify(request));
  const p = e.input_buffer(bytes.length); assert.ok(p);
  new Uint8Array(e.memory.buffer, p, bytes.length).set(bytes);
  const length = e.execute();
  const value = JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer, e.output_buffer(), length)));
  if (value.error) throw new Error(value.error);
  return value;
}
const source = async name => readFile(new URL(`../fixtures/library/${name}`, import.meta.url), 'utf8');
const compiled = invoke({ method: 'compile_rcore', rcore: await source('library.rcore'), xtext: await source('library.xtext'), rcore_behavior: await source('library-behavior.rcore') });
assert.equal(compiled.ok, true, JSON.stringify(compiled));
const pkg = compiled.package;
const evaluated = invoke({ method: 'evaluate', package: pkg, sources: [
  { uri: 'catalog.library', text: await source('catalog.library') },
  { uri: 'loans.library', text: await source('loans.library') },
] });
assert.deepEqual(evaluated.diagnostics, []);
const model = evaluated.model;
model.resources['empty.library'] = { uri:'empty.library', roots:[], content_revision:"0", load_state:'Loaded', diagnostics:[], source:null };
const selected = ['loans.library'];
const exported = invoke({ method:'model/export', package:pkg, model, selected, closure:'selected', format:'xmi' });
assert.deepEqual(exported.resources.map(r => r.uri), selected);
const imported = invoke({ method:'model/import', package:pkg, model, selected, resources:exported.resources, format:'xmi' });
assert.deepEqual(imported.diagnostics, []);
assert.deepEqual(imported.model.elements.filter(e => e.resource === 'catalog.library'), model.elements.filter(e => e.resource === 'catalog.library'));
assert.deepEqual(imported.model.resources['empty.library'], model.resources['empty.library']);
assert.throws(() => invoke({ method:'model/import', package:pkg, model, selected, resources:[{ uri:'loans.library',bytes:Array.from(new TextEncoder().encode('<unsupported/>')) }], format:'xmi' }));
const all = invoke({ method:'model/export', package:pkg, model, selected:Object.keys(model.resources), closure:'selected', format:'xmi' });
assert.deepEqual(invoke({ method:'model/import',package:pkg,resources:all.resources,format:'xmi' }).model, model);
const closure = invoke({ method:'model/export',package:pkg,model,selected,closure:'dependencies',format:'xmi' });
assert.deepEqual(closure.resources.map(r=>r.uri), ['catalog.library','loans.library']);
console.log('Real WASM JSON resource selection, retained references, empty resource, closure and rejection passed.');
