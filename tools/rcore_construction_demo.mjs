// Handwritten verification adapter; Rust owns construction, linking and execution.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {resolve,dirname} from 'node:path';
import {createRcoreHost,executionLimits} from './rcore_host.mjs';
const root=fileURLToPath(new URL('../',import.meta.url));
const host=await createRcoreHost(root,process.argv.includes('--native-only'));
const read=p=>readFileSync(resolve(root,p),'utf8');
const hash=text=>createHash('sha256').update(text).digest('hex');
const source=(uri,kind,text,dependencies=[])=>({id:{uri,sha256:hash(text)},kind,text,dependencies});
const bundle=sources=>({version:1,sources,namespaces:[]});
const namespace='urn:intersect:rcore:web-specialization';
const lowered=host.invoke({method:'rcore/lowerStructure',uri:'fixture:rcore/web/specialization.rcore',generated_uri:'fixture:rcore/web/specialization.generated.ecore',text:read('fixtures/rcore/web/specialization.rcore')});
assert.equal(lowered.ok,true);
const ordered=s=>({id:{uri:s.id.uri,sha256:s.id.sha256},kind:s.kind,text:s.text,dependencies:s.dependencies});
const original=ordered(lowered.lowering.original),model=ordered(lowered.lowering.generated);
const types=lowered.lowering.classifiers.map(c=>({kind:'classifier',declaration:{source:model.id,path:c.declaration.path},arguments:[],raw:false}));
const grammar=source('fixture:rcore/web/specialization.xtext','xtext',read('fixtures/rcore/web/specialization.xtext'),[model.id]);
const scoping=source('fixture:rcore/web/scoping-behavior.rcore','rcore_behavior',JSON.stringify({version:1,scope:'unique_names',derived:[],validations:[]}),[model.id]);
const behavior=source('fixture:rcore/web/specialization-behavior.rcore','rcore_behavior',read('fixtures/rcore/web/specialization-behavior.rcore'),[model.id]);
const definitions=bundle([original,model,grammar,scoping,behavior]);
const compilation={bundle:definitions,ecore:model.id,namespace,xtext:grammar.id,rcore_behavior:scoping.id,materialized_types:types};
const compiled=host.invoke({method:'compile_bundle',compilation});
assert.equal(compiled.ok,true,JSON.stringify(compiled));
assert.deepEqual(compiled.package.compilation.bundle,definitions);
const request={bundle:definitions,entry:behavior.id,selection:{symbol:{source:behavior.id,declaration:1},types:[],arguments:[]},receiver_type:types[1],operation:lowered.lowering.operations[0].declaration,materialized_types:types};
const operation=host.invoke({method:'rcore/compileOperation',...request});
assert.equal(operation.ok,true,JSON.stringify(operation));
const nodes=['A','B','C','D'];
const nodeId=name=>`types.demo#${nodes.indexOf(name)+1}`;
const cases=[
 {name:'chain',edges:[['A','B'],['B','C']],expected:['A','B','C']},
 {name:'edited-chain',edges:[['A','B'],['B','D']],expected:['A','B','D']},
 {name:'diamond',edges:[['A','B'],['A','C'],['B','D'],['C','D']],expected:['A','B','C','D']},
 {name:'cycle',edges:[['A','B'],['B','A']],expected:['A','B']},
 {name:'duplicate-edges',edges:[['A','B'],['A','B'],['B','C']],expected:['A','B','C']},
 {name:'isolated',edges:[],expected:['A']}
];
const semanticRows=m=>m.elements.map(e=>({id:e.id,class:e.class,type_identity:e.type_identity,values:e.values}));
const results=[];
for(const c of cases) {
 const sources=[{uri:'types.demo',text:read('fixtures/rcore/web/types.demo')},{uri:'relations.demo',text:`model Relations { ${c.edges.map(([a,b],i)=>`specialization S${i} ${a} to ${b};`).join(' ')} }`}];
 const parsed=host.invoke({method:'evaluate',package:compiled.package,sources});
 assert.deepEqual(parsed.diagnostics,[],JSON.stringify(parsed.diagnostics));
 assert.equal(parsed.references.length,c.edges.length*2);
 const instances=[{id:'types.demo#0',type:types[0]},...nodes.map(name=>({id:nodeId(name),type:types[1]})),{id:'relations.demo#0',type:types[0]},...c.edges.map((_,i)=>({id:`relations.demo#${i+1}`,type:types[2]}))];
 const edits=[['types.demo#0','name','Types'],['relations.demo#0','name','Relations'],['types.demo#0','types',nodes.map(nodeId)],...nodes.map(name=>[nodeId(name),'name',name]),['relations.demo#0','specializations',c.edges.map((_,i)=>`relations.demo#${i+1}`)],...c.edges.flatMap(([a,b],i)=>[[`relations.demo#${i+1}`,'name',`S${i}`],[`relations.demo#${i+1}`,'specific',nodeId(a)],[`relations.demo#${i+1}`,'general',nodeId(b)]])];
 const direct=host.invoke({method:'ecore/materialize',bundle:bundle([original,model]),entry:model.id,namespace,types,instances,edits});
 assert.equal(direct.ok,true,JSON.stringify(direct));
 assert.deepEqual(semanticRows(parsed.model),semanticRows(direct.model));
 assert.equal(parsed.model.schema_revision,direct.model.schema_revision);
 assert.ok(parsed.model.elements.every(e=>e.end>e.start));
 assert.ok(direct.model.elements.every(e=>e.resource==='memory:materialized' && e.start===0 && e.end===0));
 const observations=[];
 for(const constructed of [parsed.model,direct.model]) {
  const context={bundle:bundle([original,model]),entry:model.id,namespace,conversions:[],materialized_types:types,model:constructed};
  const digest=hash(JSON.stringify(context));
  const p=operation.program;
  const evaluated=host.invoke({method:'rcore/invokeOperation',...request,receiver_argument:{ty:p.functions[p.entry].parameters[0],data:{Model:{kind:'model_reference',context:digest,element:nodeId('A')}}},arguments:[],reference_contexts:[context],execution_limits:{...executionLimits,observation:{enabled:true,max_events:4096,max_event_bytes:65536}}});
  assert.equal(evaluated.ok,true,JSON.stringify(evaluated));
  assert.deepEqual(evaluated.evaluation.value.data.Collection.map(v=>v.data.Model.element),c.expected.map(nodeId));
  const report=evaluated.evaluation.observation;
  assert.ok(report.execution_complete && report.events_complete && !report.truncated);
  assert.equal(report.cache_dependencies_complete,false);
  assert.equal(report.validation_admissions.length,1);
  const reads=report.semantic_reads.map(({context,...read})=>read);
  observations.push(reads);
 }
 assert.deepEqual(observations[0],observations[1]);
 results.push({name:c.name,expected:c.expected,elements:instances.length,references:parsed.references.length,semantic_reads:observations[0].length});
}
for(const text of ['model Relations { specialization bad A to Missing; }','model Relations { specialization bad A to Relations; }','model Relations { specialization bad A to ; }']) {
 const bad=host.invoke({method:'evaluate',package:compiled.package,sources:[{uri:'types.demo',text:read('fixtures/rcore/web/types.demo')},{uri:'relations.demo',text}]});
 assert.ok(bad.diagnostics.length>0);
}
const report={checkpoint:'R06',hosts:process.argv.includes('--native-only')?['native']:['native','WASM'],native_wasm_comparisons:host.comparisons,source_ids:definitions.sources.map(s=>s.id),cases:results,provenance_differences:'Text retains document resources and byte ranges; direct materialization retains memory:materialized and zero ranges.',qualification:'Bounded named specialization projection; full KerML/Xtext/Ecore and complete dependency observations remain unqualified.'};
const arg=process.argv.indexOf('--report');
if(arg>=0){const path=resolve(root,process.argv[arg+1]);mkdirSync(dirname(path),{recursive:true});writeFileSync(path,JSON.stringify(report,null,2)+'\n');}
console.log(JSON.stringify(report,null,2));
