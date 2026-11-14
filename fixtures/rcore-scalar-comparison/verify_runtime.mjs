// Verification consumer only; all semantics execute in the shared Rust runtime.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,existsSync} from 'node:fs';
import {gunzipSync} from 'node:zlib';
import {createHash} from 'node:crypto';
import {spawn,spawnSync} from 'node:child_process';
import {createInterface} from 'node:readline';
import {fileURLToPath} from 'node:url';
const root=fileURLToPath(new URL('./',import.meta.url));
const manifest=JSON.parse(readFileSync(root+'provenance.json'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const encode=v=>Buffer.from(JSON.stringify(v));
function blob(key){
 const entry=manifest.entries[key];assert.ok(entry,key);
 assert.match(entry.sha256,/^[a-f0-9]{64}$/);
 const raw=gunzipSync(readFileSync(root+'objects/'+entry.sha256+'.gz'));
 assert.equal(raw.length,entry.bytes);assert.equal(sha(raw),entry.sha256);return raw;
}
const read=key=>JSON.parse(blob(key));
const [mode,artifact,output]=process.argv.slice(2);
assert.ok(['native','wasm'].includes(mode)&&artifact&&output,'native|wasm ARTIFACT NEW_OUTPUT');
assert.equal(existsSync(output),false);mkdirSync(output,{recursive:true});
const artifactBytes=readFileSync(artifact),artifactHash=sha(artifactBytes);
const module=mode==='wasm'?new WebAssembly.Module(artifactBytes):null;
if(module)assert.deepEqual(WebAssembly.Module.imports(module),[]);
async function open(){
 if(mode==='native'){
  const p=spawn(artifact,['rpc'],{stdio:['pipe','pipe','inherit']});
  const lines=createInterface({input:p.stdout})[Symbol.asyncIterator]();
  return {async send(request){p.stdin.write(JSON.stringify(request)+'\n');const row=await lines.next();assert.equal(row.done,false);return JSON.parse(row.value);},async close(){p.stdin.end();await new Promise((resolve,reject)=>{p.once('error',reject);p.once('close',code=>code===0?resolve():reject(Error('host exit '+code)));});}};
 }
 const {exports:e}=new WebAssembly.Instance(module,{});
 return {async send(request){const b=encode(request),p=e.input_buffer(b.length);new Uint8Array(e.memory.buffer,p,b.length).set(b);const n=e.execute();return JSON.parse(Buffer.from(new Uint8Array(e.memory.buffer,e.output_buffer(),n)).toString('utf8'));},async close(){}};
}
const suites=['before-main','before-unit','before-unicode'].map(key=>({key,cases:read(key+'/requests.json'),before:read(key+'/native-0.json')}));
const expectedPublic=read('current-public/native-0.json');
const plan=read('lifecycle/plan.json'),expectedLife=read('lifecycle/native-0.json');
const recovery=read('recovery/terminal.json');
assert.deepEqual(plan.recovery,recovery.request);
const unknown={error:'Unknown, stale or foreign function job'},runs=[];
let liveCapture=null;
try{
 for(let replay=0;replay<2;replay++){
  const publicObservations=[];
  for(let si=0;si<suites.length;si++){
   const suite=suites[si],host=await open(),rows=[],outputs=[];
   liveCapture={kind:'public',replay,suite:suite.key,rows,outputs};
   const send=async request=>{const response=await host.send(request);rows.push({request_sha256:sha(encode(request)),response});return response;};
   try{
    assert.deepEqual(await send({method:'rcore_behavior/jobHost',namespace:'scalar-equality-before'}),suite.before.rows[0].response);
    for(let index=0;index<suite.cases.length;index++){
     const c=suite.cases[index],first=rows.length;let result=await send(c.request);
     assert.equal(sha(encode(c.request)),suite.before.rows[suite.before.outputs[index].first_row].request_sha256);
     if(result.pending){
      const job=result.job;
      for(let n=0;n<100000&&result.pending;n++)result=await send({method:'rcore_behavior/resumeFunction',job,quantum:4096});
      assert.equal(result.pending,false);assert.deepEqual(await send({method:'rcore_behavior/resumeFunction',job,quantum:1}),unknown);result=result.result;
     }
     assert.deepEqual(result,suite.before.outputs[index].outcome,'Whole original outcome '+c.id);
     outputs.push({index,id:c.id,first_row:first,row_count:rows.length-first,outcome:result});
    }
   }finally{await host.close();}
   // Only the historical absolute directory metadata is not a runtime field.
   assert.deepEqual({rows,outputs},{rows:expectedPublic[si].rows,outputs:expectedPublic[si].outputs});
   publicObservations.push({suite:suite.key,rows,outputs});
  }
  const host=await open(),rows=[],lifetimes=[];
  liveCapture={kind:'lifecycle',replay,rows,lifetimes};
  const send=async request=>{const response=await host.send(request);rows.push({request_sha256:sha(encode(request)),response});return response;};
  const start=async request=>{const result=await send(request);assert.equal(result.pending,true);assert.equal(typeof result.job,'string');return result.job;};
  const recover=async job=>{
   let result;
   for(let n=0;n<100;n++){result=await send({method:'rcore_behavior/resumeFunction',job,quantum:256});if(!result.pending)break;}
   assert.equal(result.pending,false);assert.deepEqual(result.result,recovery.outcome);
   assert.deepEqual(await send({method:'rcore_behavior/resumeFunction',job,quantum:1}),unknown);
  };
  try{
   assert.deepEqual(await send({method:'rcore_behavior/jobHost',namespace:'scalar-comparison-lifecycle'}),{initialized:true});
   assert.deepEqual(await send({method:'rcore_behavior/jobHost',namespace:'foreign-comparison-host'}),{error:'Function job host already initialized'});
   for(const point of plan.points)for(const action of plan.actions){
    const first=rows.length,dormant=[];for(let i=0;i<3;i++)dormant.push(await start(plan.recovery));
    const job=await start(point.request);
    assert.deepEqual(await send(plan.recovery),{error:'Active function job budget exceeded (4)'});
    assert.deepEqual(await send({method:'rcore_behavior/resumeFunction',job:'foreign-comparison-host:'+job.split(':').at(-1),quantum:256}),unknown);
    for(let n=1;n<=point.prefix;n++)assert.deepEqual(await send({method:'rcore_behavior/resumeFunction',job,quantum:256}),{job,pending:true});
    const stopped=await send({method:'rcore_behavior/'+(action==='cancel'?'cancelFunction':'dropFunction'),job});
    if(action==='cancel'){
     assert.equal(stopped.pending,false);assert.equal(stopped.result.ok,false);assert.equal(stopped.result.failure.kind,'Cancelled');
     assert.deepEqual(stopped.result.program,point.complete.program);assert.equal(Object.hasOwn(stopped.result,'evaluation'),false);
    }else assert.deepEqual(stopped,{dropped:true});
    for(const method of ['resumeFunction','cancelFunction','dropFunction'])assert.deepEqual(await send({method:'rcore_behavior/'+method,job,...(method==='resumeFunction'?{quantum:1}:{})}),unknown);
    const fresh=await start(plan.recovery);await recover(fresh);for(const other of dormant)await recover(other);
    lifetimes.push({id:point.id,point:point.point,action,prefix:point.prefix,witness:point.witness,first_row:first,row_count:rows.length-first,stopped});
   }
  }finally{await host.close();}
  assert.deepEqual({rows,lifetimes},expectedLife,'Every lifecycle request hash and response including canonical recovery');
  const capture={public:publicObservations,lifecycle:{rows,lifetimes}};
  writeFileSync(output+'/run-'+replay+'.json',encode(capture),{flag:'wx'});
  runs.push({replay,sha256:sha(encode(capture)),public_cases:suites.reduce((n,s)=>n+s.cases.length,0),lifetimes:lifetimes.length});
 }
 if(mode==='native'){
  const phases=[];
  for(const point of plan.points.filter(p=>p.point==='first')){
   const run=spawnSync(artifact,[],{input:JSON.stringify({request:{...point.request,expected_program:point.complete.program},quantum:256}),encoding:'utf8',maxBuffer:12*1024*1024});
   assert.equal(run.status,0,run.stderr);const rows=JSON.parse(run.stdout);
   for(const p of plan.points.filter(p=>p.id===point.id))assert.deepEqual(rows[p.prefix-1],p.witness,'Real source/SomeRegistry q256 phase');
   assert.deepEqual(rows.at(-1).progress.complete.Ok,point.complete.evaluation);
   phases.push({id:point.id,rows});
  }
  writeFileSync(output+'/typed-phases.json',encode(phases),{flag:'wx'});
 }
 assert.equal(runs[0].sha256,runs[1].sha256);assert.equal(sha(readFileSync(artifact)),artifactHash);
 writeFileSync(output+'/receipt.json',JSON.stringify({state:'pass',mode,artifact:{path:artifact,bytes:artifactBytes.length,sha256:artifactHash},producer_sha256:sha(readFileSync(fileURLToPath(import.meta.url))),provenance_sha256:sha(readFileSync(root+'provenance.json')),runs,scope:'Whole original164outcomes, exact current public rows,36complete lifetimes twice. Native additionally couples genuine typed SomeRegistry q256 phases. No universal allocator/provider/cleanup or fullS09 award.'},null,2)+'\n',{flag:'wx'});
 console.log(mode,'164whole original outcomes +36lifecycle traces x2 PASS');
}catch(error){writeFileSync(output+'/failure.json',JSON.stringify({runs,liveCapture,error:String(error),stack:error.stack},null,2)+'\n',{flag:'wx'});throw error;}
