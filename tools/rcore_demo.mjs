// Handwritten host harness. Rust owns compilation, model validation and evaluation.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {createRcoreHost,executionLimits} from './rcore_host.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const fixture = resolve(root, 'fixtures/rcore/day01');
const nativeOnly = process.argv.includes('--native-only');
const structural = process.argv.includes('--structure');
const rcore = structural || process.argv.includes('--rcore');
const host = await createRcoreHost(root, nativeOnly);
const invoke = request => host.invoke(request);
const hash = text => createHash('sha256').update(text).digest('hex');
const read = name => readFileSync(resolve(name.endsWith('.rcore') ? resolve(root, 'fixtures/rcore/day02') : fixture, name), 'utf8');
const source = (name, kind, text, dependencies = []) => ({ id: { uri: `fixture:rcore/${name.endsWith('.rcore') ? 'day02' : 'day01'}/${name}`, sha256: hash(text) }, kind, text, dependencies });
let structuralOriginal;
const bundle = sources => ({ version: 1, sources: structuralOriginal ? [...sources, structuralOriginal] : sources, namespaces: [] });
const runtimeType = (model, index) => ({ kind: 'classifier', declaration: { source: model.id, path: [index] }, arguments: [], raw: false });
const provenance = JSON.parse(read('kerml-provenance.json'));
assert.equal(hash(readFileSync(resolve(fixture, 'upstream/kerml.ecore'))), provenance.source_sha256, 'pinned upstream bytes');
const cases = JSON.parse(read('cases.json'));
const report = { checkpoint: structural ? 'Rcore daily 03' : rcore ? 'Rcore daily 02' : 'Rcore daily 01', syntax: rcore ? 'Rcore behavior syntax 1' : 'Rcore v2 compatibility syntax', hosts: nativeOnly ? ['native'] : ['native', 'WASM'], provenance, demonstrations: [] };

for (const name of ['organization', 'kerml-specialization']) {
  const isKerML = name === 'kerml-specialization';
  const namespace = isKerML ? provenance.projection_namespace : 'urn:intersect:rcore:organization';
  structuralOriginal = undefined;
  let model = source(`${name}.ecore`, 'ecore', read(`${name}.ecore`));
  let structuralLowering;
  if (structural) {
    const uri = `fixture:rcore/day03/${name}.rcore`;
    const text = readFileSync(resolve(root, `fixtures/rcore/day03/${name}.rcore`), 'utf8');
    const lowered = invoke({method:'rcore/lowerStructure', uri, text, generated_uri:`fixture:rcore/day03/${name}.generated.ecore`});
    assert.equal(lowered.ok, true, JSON.stringify(lowered));
    structuralLowering = lowered.lowering;
    const orderedSource = s => ({id:{uri:s.id.uri,sha256:s.id.sha256},kind:s.kind,text:s.text,dependencies:s.dependencies});
    structuralOriginal = orderedSource(structuralLowering.original);
    model = orderedSource(structuralLowering.generated);
    assert.equal(structuralOriginal.id.sha256, hash(text));
    assert.deepEqual(model.dependencies, [], "generation origins are separate from Ecore imports");
    assert.notEqual(model.id.sha256, structuralOriginal.id.sha256);
    for (const origin of structuralLowering.origins) {
      assert.deepEqual(origin.original.source, structuralOriginal.id);
      assert.deepEqual(origin.declaration.source, model.id);
    }
  }
  const types = [runtimeType(model, 0), ...(isKerML ? [runtimeType(model, 1)] : [])];
  function materialize(c) {
    const instances = c.nodes.map(id => ({ id, type: types[0] }));
    const edits = [];
    if (isKerML) {
      c.edges.forEach(([specific, general], i) => {
        const id = `s${String(i).padStart(3, '0')}`;
        instances.push({ id, type: types[1] });
        edits.push([id, 'specific', specific], [id, 'general', general]);
      });
    } else {
      for (const id of c.nodes) edits.push([id, 'links', c.edges.filter(([from]) => from === id).map(([, to]) => to)]);
    }
    const result = invoke({ method: 'ecore/materialize', bundle: bundle([model]), entry: model.id, namespace, types, instances, edits });
    assert.equal(result.ok, true, JSON.stringify(result));
    return { bundle: bundle([model]), entry: model.id, namespace, conversions: [], materialized_types: types, model: result.model };
  }
  function compile(file) {
    const text = read(file);
    const entry = source(file, 'rcore_behavior', text, [model.id]);
    const request = { bundle: bundle([model, entry]), entry: entry.id, selection: { symbol: { source: entry.id, declaration: 1 }, types: [], arguments: [] } };
    const result = invoke({ method: 'rcore_behavior/compileFunction', ...request });
    assert.equal(result.ok, true, JSON.stringify(result));
    return { request, result, text };
  }
  const extension = rcore ? 'rcore' : 'rcore_behavior';
  const compiled = compile(`${name}.${extension}`);
  const reloaded = compile(`${name}-seeds-only.${extension}`);
  const legacy = rcore ? compile(`${name}-behavior.rcore`) : null;
  const legacyReloaded = rcore ? compile(`${name}-seeds-only-behavior.rcore`) : null;
  assert.notEqual(compiled.request.entry.sha256, reloaded.request.entry.sha256);
  function evaluation(program, reference, seeds, extra = {}) {
    const p = program.result.program;
    const parameter = p.functions[p.entry].parameters[0];
    const element = p.types[parameter].Collection.element;
    const digest = hash(JSON.stringify(reference));
    const arguments_ = [{ ty: parameter, data: { Collection: seeds.map(id => ({ ty: element, data: { Model: { kind: 'model_reference', context: digest, element: id } } })) } }];
    return invoke({ method: 'rcore_behavior/evaluateFunction', ...program.request, arguments: arguments_, reference_contexts: [reference], ...extra });
  }
  const results = [];
  for (const c of cases) {
    const reference = materialize(c);
    const before = JSON.stringify(reference);
    const result = evaluation(compiled, reference, c.seeds);
    assert.equal(result.ok, true, JSON.stringify(result));
    const actual = result.evaluation.value.data.Collection.map(v => v.data.Model.element);
    assert.deepEqual(actual, c.expected, `${name}/${c.name}`);
    if (legacy) {
      const old = evaluation(legacy, reference, c.seeds);
      assert.equal(old.ok, true, JSON.stringify(old));
      assert.deepEqual(old.evaluation.value.data, result.evaluation.value.data, 'Rcore/Rcore value equivalence');
    }
    assert.equal(JSON.stringify(reference), before, 'read-only evaluation');
    const editedBody = evaluation(reloaded, reference, c.seeds);
    assert.equal(editedBody.ok, true, JSON.stringify(editedBody));
    assert.deepEqual(editedBody.evaluation.value.data.Collection.map(v => v.data.Model.element), [...new Set(c.seeds)], 'source reload without Rust rebuild');
    if (legacyReloaded) {
      const old = evaluation(legacyReloaded, reference, c.seeds);
      assert.equal(old.ok, true, JSON.stringify(old));
      assert.deepEqual(old.evaluation.value.data, editedBody.evaluation.value.data, 'reloaded Rcore/Rcore equivalence');
    }
    results.push({ case: c.name, expected: c.expected, actual, model_view: hash(before), source_reload: 'seeds only' });
  }
  assert.notEqual(results[0].model_view, results.at(-1).model_view, 'edited relationship selects a fresh view');
  const reference = materialize(cases[0]);
  const exhausted = evaluation(compiled, reference, ['A'], { execution_limits: { ...executionLimits, max_provider_work: 0 } });
  assert.equal(exhausted.ok, false);
  assert.equal(exhausted.failure.kind, 'ResourceExhausted');
  const missing = evaluation(compiled, reference, ['missing']);
  assert.equal(missing.ok, false, 'missing seed must not become absence');
  // Negative construction through the same native/WASM API.
  const invalid = invoke({ method: 'ecore/materialize', bundle: bundle([model]), entry: model.id, namespace, types,
    instances: [{ id: 'A', type: types[0] }, ...(isKerML ? [{ id: 's', type: types[1] }] : [])],
    edits: isKerML ? [['s', 'specific', 'A'], ['s', 'general', 'missing']] : [['A', 'links', ['missing']]] });
  assert.equal(invalid.ok, false, 'unresolved relationship endpoint');
  if (rcore) {
    const stale = evaluation(reloaded, reference, ['A'], { expected_program: compiled.result.program });
    assert.equal(stale.ok, false);
    assert.equal(stale.stage, 'function_replay', 'edited source must not trust old IR');
  }
  const recovered = evaluation(compiled, reference, ['A']);
  assert.equal(recovered.ok, true, 'fresh recovery');
  report.demonstrations.push({ name, structural_lowering: structuralLowering, source: compiled.text, program: compiled.result.program,
    static_effects: compiled.result.effects, cases: results,
    controls: { exhausted: exhausted.failure.kind, missing_seed: missing.failure?.kind || missing.stage, invalid_endpoint: invalid.stage, fresh_recovery: true },
    evidence_limits: 'IR and effects are static inspection; consumed budgets are bookkeeping. No complete semantic dependency record or detailed execution trace is claimed.' });
}
structuralOriginal = undefined;
if (structural) {
  const structuralText = 'rcore 1; package runtime version "1"; metamodel "urn:rcore:runtime" prefix r { abstract class Base { attr label: String [1,1]; } class Parent extends Base { ref children: Child [0,*] containment opposite Child.parent; } class Child { ref parent: Parent [0,1] opposite Parent.children; attr count: Int32 [0,1]; } }';
  const structuralResult = invoke({method:'rcore/lowerStructure',uri:'memory:runtime.rcore',generated_uri:'memory:runtime.ecore',text:structuralText});
  assert.equal(structuralResult.ok, true, JSON.stringify(structuralResult));
  const generated = structuralResult.lowering.generated;
  const structureBundle = bundle([generated, structuralResult.lowering.original]);
  const selected = [0,1,2].map(i => runtimeType(generated,i));
  const construction = {method:'ecore/materialize',bundle:structureBundle,entry:generated.id,namespace:'urn:rcore:runtime',types:selected,
    instances:[{id:'p',type:selected[1]},{id:'c',type:selected[2]}], edits:[['p','label','Parent'],['p','children',['c']],['c','count',7]]};
  const constructed = invoke(construction);
  assert.equal(constructed.ok, true, JSON.stringify(constructed));
  const parent = constructed.model.elements.find(e => e.id === 'p');
  const child = constructed.model.elements.find(e => e.id === 'c');
  assert.equal(parent.values.label, 'Parent');
  assert.deepEqual(parent.values.children, ['c']);
  assert.equal(child.values.parent, 'p');
  assert.equal(child.values.count, 7);
  for (const invalid of [
    {...construction, instances:[{id:'a',type:selected[0]}], edits:[]},
    {...construction, edits:[['p','label','Parent'],['c','count','wrong']]},
    {...construction, edits:[['p','label','Parent'],['p','children',['missing']]]},
  ]) { assert.equal(invoke(invalid).ok, false, 'structural model rejection'); }
  report.structural_runtime_controls = {inherited_attribute:true,opposite_mutation:true,containment:true,integer_attribute:true,abstract_rejected:true,wrong_attribute_rejected:true,missing_endpoint_rejected:true};
  const rejected = invoke({method:'rcore/lowerStructure',uri:'memory:bad.rcore',generated_uri:'memory:bad.ecore',text:'rcore 1; package bad version "1"; metamodel "urn:bad" prefix b { class A { ref x: Missing [0,1]; } }'});
  assert.equal(rejected.ok, false);
  assert.equal(rejected.diagnostics[0].span.source.uri, 'memory:bad.rcore');
}
if (rcore) {
  const controls = [];
  for (const [helperText, callerText] of [
    ['rcore 1; package helper version "1"; pub fn keep<T>(x:T)->T=x;', 'rcore 1; package caller version "1"; import package helper version "1" as h; fn main()->Int8 {h.keep::<Int8>(42)}'],
    ['rcore 1; package helper version "1"; pub fn keep<T>(x:T)->T {x}', 'rcore 1; package caller version "1"; import package helper version "1" as h; fn main()->Int8=h.keep::<Int8>(42);'],
  ]) {
    const helper = source('helper.rcore', 'rcore_behavior', helperText);
    const entry = source('caller.rcore', 'rcore_behavior', callerText, [helper.id]);
    const request = { bundle: bundle([helper, entry]), entry: entry.id,
      selection: { symbol: { source: entry.id, declaration: 1 }, types: [], arguments: [] } };
    const compiled = invoke({ method: 'rcore_behavior/compileFunction', ...request });
    assert.equal(compiled.ok, true, JSON.stringify(compiled));
    const result = invoke({ method: 'rcore_behavior/evaluateFunction', ...request, arguments: [], reference_contexts: [] });
    assert.equal(result.ok, true, JSON.stringify(result));
    assert.equal(result.evaluation.value.data.Scalar.payload, 42);
    controls.push({ mixed_package_call: callerText.startsWith('rcore') ? 'Rcore to Rcore' : 'Rcore to Rcore', result: 42 });
  }
  for (const text of [
    'rcore 2; package demo version "1"; fn main()->Int8 {42}',
    'rcore 1; package demo version "1"; class Type {}',
    'rcore 1; package demo version "1"; fn main()->Int8 {let x=1;}',
  ]) {
    const parsed = invoke({ method: 'rcore_behavior/parse', uri: 'memory:invalid.rcore', text });
    assert.equal(parsed.ok, false);
    assert.ok(parsed.document.diagnostics.every(d => d.span.source.uri === 'memory:invalid.rcore' && d.span.end <= Buffer.byteLength(text)));
    controls.push({ rejected_source: text, diagnostics: parsed.document.diagnostics });
  }
  const typed = source('typed.rcore', 'rcore_behavior', 'rcore 1; package typed version "1"; fn main()->Int8 {let label="café😀"; true}');
  const bad = invoke({ method: 'rcore_behavior/compileFunction', bundle: bundle([typed]), entry: typed.id,
    selection: { symbol: { source: typed.id, declaration: 0 }, types: [], arguments: [] } });
  assert.equal(bad.ok, false);
  assert.ok(bad.diagnostics.length > 0);
  assert.ok(bad.diagnostics.every(d => d.span.source.sha256 === typed.id.sha256));
  controls.push({ type_error: bad.diagnostics });
  report.frontend_controls = controls;
}
const comparisons=host.comparisons;
report.native_wasm_comparisons = comparisons;
const outputIndex = process.argv.indexOf('--output');
if (outputIndex >= 0) { const output=resolve(process.argv[outputIndex+1]); mkdirSync(dirname(output),{recursive:true}); writeFileSync(output,JSON.stringify(report,null,2)+'\n'); }
console.log(JSON.stringify({ checkpoint: report.checkpoint, hosts: report.hosts, native_wasm_comparisons: comparisons,
  demonstrations: report.demonstrations.map(d => ({ name: d.name, cases: d.cases.map(c => ({ case: c.case, result: c.actual })), controls: d.controls })) }, null, 2));
