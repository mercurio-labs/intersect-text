// Exact source/terminal semantics and real shared public-host lifecycle.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,appendFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {gunzipSync} from 'node:zlib';
import {fileURLToPath} from 'node:url';
import {resolve,join} from 'node:path';
const root=fileURLToPath(new URL('../',import.meta.url)),F=join(root,'fixtures/rcore-input-string-transport');
const read=p=>JSON.parse(readFileSync(p,'utf8'));
const pin=p=>{const b=readFileSync(p);return {bytes:b.length,sha256:createHash('sha256').update(b).digest('hex')};};
const args=process.argv.slice(2),value=k=>{const n=args.indexOf(k);return n<0?undefined:args[n+1];};
const manifest=read(join(F,'provenance.json'));
for(const row of manifest.files)assert.deepEqual(pin(join(root,row.path)),row.pin,row.path);
const lineage=read(join(F,'lineage.json'));
for(const row of lineage.compressed_complete_originals_and_candidate){const raw=gunzipSync(readFileSync(join(F,row.path)));assert.deepEqual({bytes:raw.length,sha256:createHash('sha256').update(raw).digest('hex')},row.uncompressed);}
const phaseLineage=read(join(F,'phase-lineage.json'));
for(const row of phaseLineage.lossless_records){const raw=gunzipSync(readFileSync(join(root,row.path)));assert.deepEqual({bytes:raw.length,sha256:createHash('sha256').update(raw).digest('hex')},row.uncompressed);}
if(args.includes('--integrity-only')){console.log(JSON.stringify({files:manifest.files.length,original_lossless_captures:4,phase_lossless_captures:phaseLineage.lossless_records.length,gate_closures:[]}));process.exit(0);}
const wasm=resolve(value('--wasm')||process.env.INTERSECT_TEST_WASM||join(root,'toolkit/public/intersect_web.wasm'));
const originalSchedule=args.includes('--original-schedule');
const planFile=join(F,originalSchedule?'recorded/public-candidate-plan.gz':'recorded/public-retained-json-phase-plan.gz');
const plan=JSON.parse(gunzipSync(readFileSync(planFile)));assert.equal(plan.groups.length,24);
const canonical=v=>Array.isArray(v)?v.map(canonical):v&&typeof v==='object'?Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])])):v;
const module=new WebAssembly.Module(readFileSync(wasm));assert.deepEqual(WebAssembly.Module.imports(module),[]);
const output=value('--out');if(output)mkdirSync(output);const save=(name,data)=>{if(output)writeFileSync(join(output,name),JSON.stringify(data,null,2)+'\n',{flag:'wx'});};
save('preexecution.json',{runner:{path:fileURLToPath(import.meta.url),...pin(fileURLToPath(import.meta.url))},provenance:pin(join(F,'provenance.json')),plan:{original_schedule:originalSchedule,...pin(planFile)},artifact:{path:wasm,...pin(wasm)},imports:[],node:process.version,scope:'Whole persistent public host per group. Complete original source programs/results/errors/fees; Pending topology is observed, not an immutable scheduling contract.'});
const encoder=new TextEncoder(),decoder=new TextDecoder('utf-8',{fatal:true}),runs=[];let current;
try{
 for(let replay=0;replay<2;replay++){
  let count=0;const schedules=[];const path=output?join(output,`replay-${replay}.jsonl`):null;if(path)writeFileSync(path,'',{flag:'wx'});
  for(const group of plan.groups){
   const e=new WebAssembly.Instance(module,{}).exports,baseline=group.id.endsWith('-baseline');let completeAt=null;
   const terminal=baseline?group.native_responses.find(r=>r.pending===false):null;
   for(let index=0;index<group.requests.length;index++){
    const request=group.requests[index];current={replay,group:group.id,index,request};const bytes=encoder.encode(JSON.stringify(canonical(request))),pointer=e.input_buffer(bytes.length);assert.notEqual(pointer,0);new Uint8Array(e.memory.buffer,pointer,bytes.length).set(bytes);const n=e.execute(),response=JSON.parse(decoder.decode(new Uint8Array(e.memory.buffer,e.output_buffer(),n)));current.response=response;count++;
    if(path)appendFileSync(path,JSON.stringify({group:group.id,index,request,response})+'\n');
    if(!baseline||index<3)assert.deepEqual(response,group.native_responses[index],`${group.id}/${index}`);
    else if(completeAt!==null)assert.deepEqual(response,{error:'Unknown, stale or foreign function job'});
    else if(response.pending===false){assert.deepEqual(response,terminal);completeAt=index;}
    else assert.deepEqual(response,{job:group.native_responses[2].job,pending:true});
   }
   if(baseline){assert.notEqual(completeAt,null);if(group.case==='ascii240000')assert.ok(completeAt>3,'Large default String must expose resumable work');schedules.push({group:group.id,completeAt});}
  }
  runs.push({replay,groups:24,responses:count,schedules,...(path?{capture:pin(path)}:{})});
 }
 const receipt={exit_code:0,artifact:{path:wasm,...pin(wasm)},runs,limits:['Native allocator observations are separate; no architecture counter/physical-copy/OS/peak-heap or cleanup claim.','Whole-root shape/count/reservation, JSON parsing/host marshaling, stored model projection/context and other payloads remain separate W1/W4 obligations.','Cancellation bytes are cumulative admitted work with zero published nodes during input copy; no refund claim.'],gate_closures:[]};save('receipt.json',receipt);console.log(JSON.stringify({exit_code:0,groups:48,responses:runs.reduce((n,r)=>n+r.responses,0),artifact:pin(wasm),gate_closures:[]}));
}catch(error){save('failure.json',{current,error:{message:error.message,stack:error.stack}});throw error;}
