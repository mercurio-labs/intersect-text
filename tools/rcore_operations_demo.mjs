// Handwritten host verification; Rust owns model, binding and execution semantics.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync,writeFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import {createRcoreHost,executionLimits} from './rcore_host.mjs';
const root=fileURLToPath(new URL('../',import.meta.url));
const host=await createRcoreHost(root,process.argv.includes('--native-only'));
const hash=s=>createHash('sha256').update(s).digest('hex');
const read=p=>readFileSync(resolve(root,p),'utf8');
const ordered=s=>({id:{uri:s.id.uri,sha256:s.id.sha256},kind:s.kind,text:s.text,dependencies:s.dependencies});
const bundle=sources=>({version:1,sources,namespaces:[]});
const type=(source,index)=>({kind:'classifier',declaration:{source:source.id,path:[index]},arguments:[],raw:false});
const cases=JSON.parse(read('fixtures/rcore/day01/cases.json'));
const observe=process.argv.includes('--observe');
const report={checkpoint:observe?'R05':'R04',hosts:process.argv.includes('--native-only')?['native']:['native','WASM'],demonstrations:[]};
for (const name of ['organization','kerml-specialization']) {
  const kerml=name!=='organization';
  const lower=host.invoke({method:'rcore/lowerStructure',uri:`fixture:rcore/day04/${name}.rcore`,text:read(`fixtures/rcore/day04/${name}.rcore`),generated_uri:`fixture:rcore/day04/${name}.generated.ecore`});
  assert.equal(lower.ok,true,JSON.stringify(lower));
  const original=ordered(lower.lowering.original),model=ordered(lower.lowering.generated);
  const receiver=type(model,0),types=[receiver,...(kerml?[type(model,1)]:[])];
  const operation=lower.lowering.operations[0];
  const namespace=kerml?'urn:intersect:rcore:kerml-specialization-demo':'urn:intersect:rcore:organization';
  function compile(text) {
    const entry={id:{uri:`fixture:rcore/day04/${name}-behavior.rcore`,sha256:hash(text)},kind:'rcore_behavior',text,dependencies:[model.id]};
    const request={bundle:bundle([original,model,entry]),entry:entry.id,selection:{symbol:{source:entry.id,declaration:1},types:[],arguments:[]},receiver_type:receiver,operation:operation.declaration,materialized_types:types};
    const result=host.invoke({method:'rcore/compileOperation',...request});
    return {request,result};
  }
  const text=read(`fixtures/rcore/day04/${name}-behavior.rcore`),compiled=compile(text);
  assert.equal(compiled.result.ok,true,JSON.stringify(compiled.result));
  assert.deepEqual(compiled.result.binding.operation.declaration,operation.declaration);
  assert.deepEqual(compiled.result.binding.implementation_span.source,compiled.request.entry);
  const reloaded=compile(text.replace(/closure\(\[node\], [^\n]+/, 'OrderedSet([node])'));
  assert.equal(reloaded.result.ok,true,JSON.stringify(reloaded.result));
  const wrong=compile(text.replace('node:m.', 'node:Sequence<m.').replace(')->OrderedSet', '>)->OrderedSet'));
  assert.equal(wrong.result.ok,false);
  const results=[];
  for (const c of cases) {
    const instances=c.nodes.map(id=>({id,type:receiver})),edits=[];
    if (kerml) c.edges.forEach(([specific,general],i)=>{const id=`s${i}`;instances.push({id,type:types[1]});edits.push([id,'specific',specific],[id,'general',general]);});
    else for (const id of c.nodes) edits.push([id,'links',c.edges.filter(([from])=>from===id).map(([,to])=>to)]);
    const built=host.invoke({method:'ecore/materialize',bundle:bundle([original,model]),entry:model.id,namespace,types,instances,edits});
    assert.equal(built.ok,true,JSON.stringify(built));
    const context={bundle:bundle([original,model]),entry:model.id,namespace,conversions:[],materialized_types:types,model:built.model};
    const before=JSON.stringify(context),digest=hash(before);
    function invoke(image,id,extra={}) {
      const p=image.result.program,parameter=p.functions[p.entry].parameters[0];
      return host.invoke({method:'rcore/invokeOperation',...image.request,receiver_argument:{ty:parameter,data:{Model:{kind:'model_reference',context:digest,element:id}}},arguments:[],reference_contexts:[context],...(observe?{execution_limits:{...executionLimits,observation:{enabled:true,max_events:4096,max_event_bytes:65536}}}:{}),...extra});
    }
    if (!c.seeds.length) {assert.equal(invoke(compiled,'missing').ok,false);results.push({case:c.name,receiver_required:true});continue;}
    const result=invoke(compiled,c.seeds[0]);assert.equal(result.ok,true,JSON.stringify(result));
    const actual=result.evaluation.value.data.Collection.map(v=>v.data.Model.element);assert.deepEqual(actual,c.expected);
    const edited=invoke(reloaded,c.seeds[0]);assert.equal(edited.ok,true,JSON.stringify(edited));assert.deepEqual(edited.evaluation.value.data.Collection.map(v=>v.data.Model.element),[c.seeds[0]]);
    assert.equal(invoke(compiled,'missing').ok,false);
    assert.equal(invoke(compiled,c.seeds[0],{reference_contexts:[]}).ok,false);
    assert.equal(invoke(compiled,c.seeds[0],{expected_program:reloaded.result.program}).ok,false);
    if (observe) {
      const evidence=result.evaluation.observation;
      assert.ok(evidence.execution_complete && evidence.events_complete && !evidence.truncated);
      assert.equal(evidence.cache_dependencies_complete,false);
      assert.equal(evidence.validation_admissions.length,1);
      for(const r of evidence.semantic_reads) {assert.equal(r.context,digest);assert.deepEqual(r.member.source,model.id);assert.deepEqual(r.source.source,compiled.request.entry);}
      assert.equal(edited.evaluation.observation.semantic_reads.length,0,'changed source actually performs no semantic reads');
      if(kerml) {
        assert.deepEqual(evidence.semantic_reads.filter(r=>r.kind==='incoming_start').map(r=>r.element),c.expected);
        const matches=c.expected.flatMap(id=>c.edges.map(([from],i)=>from===id?`s${i}`:null).filter(Boolean));
        assert.deepEqual(evidence.semantic_reads.filter(r=>r.kind==='incoming_match').map(r=>r.matched),matches);
        for(const id of c.expected.filter(id=>!c.edges.some(([from])=>from===id))) assert.ok(evidence.semantic_reads.some(r=>r.kind==='incoming_end' && r.element===id && r.cursor===0),'empty query is observed');
      }else {
        const pages=c.expected.flatMap(id=>{const out=c.edges.filter(([from])=>from===id).map(([,to])=>to);return [...out.map((value,cursor)=>({id,cursor,value})),{id,cursor:out.length,value:null}];});
        assert.deepEqual(evidence.semantic_reads.map(r=>({id:r.element,cursor:r.cursor,value:r.value})),pages);
      }
      const bounded=invoke(compiled,c.seeds[0],{execution_limits:{...executionLimits,observation:{enabled:true,max_events:1,max_event_bytes:65536}}});
      assert.equal(bounded.ok,true);assert.deepEqual(bounded.evaluation.value,result.evaluation.value);
      assert.ok(bounded.evaluation.observation.truncated && !bounded.evaluation.observation.events_complete);
      assert.ok(bounded.evaluation.observation.event_bytes<=65536);
    }
    assert.equal(JSON.stringify(context),before);
    results.push({case:c.name,expected:c.expected,actual,source_reload:'receiver only',...(observe?{observation:result.evaluation.observation}: {})});
  }
  report.demonstrations.push({name,operation,binding:compiled.result.binding,results});
}
report.native_wasm_comparisons=host.comparisons;
const reportArg=process.argv.indexOf('--report');
if(reportArg>=0) writeFileSync(resolve(root,process.argv[reportArg+1]),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify({checkpoint:report.checkpoint,hosts:report.hosts,native_wasm_comparisons:host.comparisons,demonstrations:report.demonstrations.map(d=>({name:d.name,cases:d.results.map(r=>({case:r.case,actual:r.actual,semantic_reads:r.observation?.semantic_reads.length,validation_admissions:r.observation?.validation_admissions.length}))}))},null,2));
