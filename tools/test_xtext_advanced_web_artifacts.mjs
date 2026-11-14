// Verification-only transport: semantics remain in the shared Rust implementation.
// Pass already-built artifacts so this check never starts or competes with builds.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
const [nativePath, wasmPath] = process.argv.slice(2);
assert.ok(nativePath && wasmPath, 'Usage: node tools/test_xtext_advanced_web_artifacts.mjs NATIVE_HELPER WASM');
const root = fileURLToPath(new URL('../', import.meta.url));
const read = path => readFileSync(resolve(root, 'fixtures', path), 'utf8');
const { instance: { exports: wasm } } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
function parity(request) {
  const input = JSON.stringify(request);
  const native = spawnSync(nativePath, [], { input, encoding: 'utf8', maxBuffer: 10_000_000 });
  assert.equal(native.status, 0, native.error?.message || native.stderr);
  const bytes = new TextEncoder().encode(input);
  const pointer = wasm.input_buffer(bytes.length);
  assert.ok(pointer, 'WASM input allocation');
  new Uint8Array(wasm.memory.buffer, pointer, bytes.length).set(bytes);
  const length = wasm.execute();
  const result = JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, wasm.output_buffer(), length)));
  assert.deepEqual(result, JSON.parse(native.stdout));
  return result;
}
for (const [name, modelRoot, modelName, documents, count, negatives, directory = 'advanced-web', profile = 'intersect.xtext.native.v27'] of [
  ['library', 'library', 'library', ['catalog.library', 'loans.library'], 6, [
    [1, 'library Bad { loan bad copy Missing to ada; }'],
    [1, 'library Bad { loan first copy dune to ada; loan duplicate copy dune to ada; }', 'more than one active loan'],
  ]],
  ['expressions', 'xtext/current-actions', 'model', ['catalog.kerml', 'more.kerml'], 9, [
    [0, 'Demo = (1 +);'],
  ]],
  ['library', 'library', 'library', ['catalog.library', 'loans.library'], 6, [
    [1, 'library Bad { loan bad copy Missing to ada_1; }'],
    [1, 'library Bad { loan a copy dune_1 to ada_1; loan b copy dune_1 to ada_1; }', 'more than one active loan'],
    [0, 'library Bad { copy bad : Title edition ; }'],
  ], 'composed-web', 'intersect.xtext.native.v42'],
  ['expressions', 'xtext/current-actions', 'model', ['catalog.kerml', 'more.kerml'], 17, [
    [0, 'Bad = (1 +);'], [0, 'Bad = all of ();'], [1, 'Bad = all 1 with ;'],
  ], 'composed-web', 'intersect.xtext.native.v42'],
  ['library-v52', 'library', 'library', ['catalog-v52.library', 'loans.library'], 6, [
    [1, 'library Bad { loan bad copy Missing to ada_1; }'],
    [1, 'library Bad { loan a copy dune_1 to ada_1; loan b copy dune_1 to ada_1; }', 'more than one active loan'],
    [0, 'library Bad { copy bad : Title edition ; }'],
    [0, String.raw`library Bad { copy bad : "bad\q"; }`, 'Invalid STRING literal'],
  ], 'composed-web', 'intersect.xtext.native.v52'],
  ['expressions', 'xtext/current-actions', 'model', ['catalog.kerml', 'more.kerml'], 17, [
    [0, 'Bad = (1 +);'], [0, 'Bad = all of ();'], [1, 'Bad = all 1 with ;'],
  ], 'composed-web', 'intersect.xtext.native.v52'],
]) {
  const compiled = parity({ method: 'compile', xtext_profile: profile,
    ecore: read(`${modelRoot}/${modelName}.ecore`), rcore_behavior: read(`${modelRoot}/${modelName}-behavior.rcore`),
    xtext: read(`xtext/${directory}/${name}.xtext`) });
  assert.equal(compiled.ok, true, JSON.stringify(compiled));
  assert.equal(compiled.package.grammar.profile, profile);
  const sources = documents.map(uri => ({ uri, text: read(`xtext/${directory}/${uri}`) }));
  const request = { method: 'evaluate', package: compiled.package, sources };
  const accepted = parity(request);
  assert.deepEqual(accepted.diagnostics, []);
  assert.equal(accepted.model.elements.length, count);
  if (name.startsWith('library')) {
    const named = name => accepted.model.elements.find(e => e.values.name === name).values;
    assert.equal(named(directory === 'advanced-web' ? 'dune' : 'dune_1').edition, 2);
    assert.equal(named(directory === 'advanced-web' ? 'spare' : 'spare_2').title, 'untitled');
    assert.equal(named(directory === 'advanced-web' ? 'spare' : 'spare_2').edition, 1);
    assert.equal(accepted.references.length, 2);
    if(name === 'library-v52') assert.equal(named('dune_1').title, 'Dune\nVolume 1');
  } else {
    assert.deepEqual(accepted.model.elements.filter(e => e.class === 'OperatorExpression').map(e => e.values.operator), directory === 'advanced-web' ? ['+', '-', 'all'] : ['+', '-', 'all', '+', 'all', '+']);
  }
  for (const [index, text, message] of negatives) {
    const invalid = sources.map((source, i) => i === index ? { ...source, text } : source);
    const rejected = parity({ ...request, sources: invalid });
    assert.ok(rejected.diagnostics.length);
    if (message) assert.ok(rejected.diagnostics.some(d => d.message.includes(message)));
    assert.deepEqual(parity(request), accepted, 'Fresh evaluation recovers exactly after rejection');
  }
  console.log(`${directory}/${name}: exact native/WASM compile, execution, rejection and recovery passed`);
}
