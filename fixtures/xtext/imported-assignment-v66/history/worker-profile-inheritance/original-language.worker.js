import {validateEntryRule} from './entry-selection.js';
import { loadRuntime } from './runtime.js';

// Transport only: Rust verifies every identity and installs the actual catalog.
function conversionSelections(package_) {
  const selected={};
  for (const [key,field] of [['conversions','conversion_contracts'],['applied_conversions','applied_conversion_contracts'],['constructors','constructor_contracts']]) {
    if (package_.schema[field]?.length) selected[key]=package_.schema[field];
  }
  return selected;
}
const runtime = loadRuntime();
const documents = new Map();
let active = null;
let revision = 0;
let evaluation = null;
let documentAnalysis = null;
let documentAnalysisFailure = null;
let analysisVersion = 0;
let documentCandidate = null;
let entryRule = null;
const session = crypto.randomUUID();
let generation = 0;
let overlay = false;
let modelOrigin = 'text';
let initialized = false;
let stopped = false;
const transactions = new Map();
let transactionNumber = 0;
// One bounded, unpublished inferred candidate per worker session. Original APIs
// keep their activation behavior; provider UI checks its draft before committing.
let inferredCandidate = null;

// Host scheduling only: Rust owns semantic state, limits and cancellation results.
const functionRequests = new Map();
let functionWorker = null;
let functionToken = 0;
function reply(data, result, error) {
  if (data.id !== undefined) self.postMessage({ jsonrpc: '2.0', id: data.id, ...(error ? { error: { code: -32603, message: error.message } } : { result }) });
  else if (error) notify('window/logMessage', { type: 1, message: error.message });
}
function finishFunction(record, result, error) {
  if (functionRequests.get(record.key) !== record) return;
  clearTimeout(record.timer);
  functionRequests.delete(record.key);
  reply(record.data, result, error);
}
function failExecutionWorker(worker, message) {
  if (functionWorker !== worker) return;
  worker.terminate();
  functionWorker = null;
  for (const record of [...functionRequests.values()]) {
    finishFunction(record, undefined, new Error(`Function execution transport failed: ${message}`));
  }
}
function executionWorker() {
  if (functionWorker) return functionWorker;
  const worker = new Worker(new URL('./function.worker.js', import.meta.url), { type: 'module' });
  functionWorker = worker;
  worker.onmessage = ({ data }) => {
    if (functionWorker !== worker) return;
    if (data.fatal) { failExecutionWorker(worker, data.error); return; }
    const record = [...functionRequests.values()].find(record => record.token === data.token);
    if (!record) return;
    record.busy = false;
    if (data.error) {
      if (record.job) worker.postMessage({ token: record.token, request: { method: 'rcore_behavior/dropFunction', job: record.job } });
      finishFunction(record, undefined, new Error(data.error));
    } else if (data.response.pending) {
      record.job = data.response.job ?? record.job;
      // Cancellation received during synchronous preparation wins before resumption.
      if (record.cancelled) advanceFunction(record);
      else record.timer = setTimeout(() => advanceFunction(record), 0);
    } else {
      finishFunction(record, data.response.result);
    }
  };
  worker.onerror = event => {
    if (functionWorker !== worker) return;
    event.preventDefault();
    failExecutionWorker(worker, event.message);
  };
  worker.addEventListener('messageerror', () => failExecutionWorker(worker, 'Unable to deserialize execution worker response'));
  return worker;
}
function advanceFunction(record, cancel = false) {
  if (functionRequests.get(record.key) !== record) return;
  record.cancelled ||= cancel;
  clearTimeout(record.timer);
  if (record.busy || !record.job) return;
  record.busy = true;
  executionWorker().postMessage({ token: record.token, request: {
    method: record.cancelled ? 'rcore_behavior/cancelFunction' : 'rcore_behavior/resumeFunction',
    job: record.job, ...(record.cancelled ? {} : { quantum: 256 }),
  } });
}
async function startFunction(data) {
  if (!initialized || stopped) throw new Error('Language server is not initialized.');
  const key = data.id === undefined ? Symbol('notification') : data.id;
  if (functionRequests.has(key)) throw new Error('Function request identity is already active.');
  if (functionRequests.size >= 4) throw new Error('Active function job budget exceeded (4)');
  const record = { key, token: ++functionToken, data, job: null, timer: null, busy: true, cancelled: false };
  functionRequests.set(key, record);
  try {
    executionWorker().postMessage({ token: record.token, request: { ...data.params, method: 'rcore_behavior/startFunction' } });
  } catch (error) {
    finishFunction(record, undefined, error);
  }
}

function position(text, offset) {
  // Rust spans are UTF-8 bytes; LSP positions are UTF-16 code units.
  const prefix = new TextDecoder().decode(new TextEncoder().encode(text).slice(0, offset));
  const lines = prefix.split('\n');
  return { line: lines.length - 1, character: lines.at(-1).length };
}
function byteOffset(text, p) {
  const lines = text.split('\n');
  return new TextEncoder().encode(lines.slice(0, p.line).join('\n') + (p.line ? '\n' : '') + (lines[p.line] || '').slice(0, p.character)).length;
}
function range(uri, start, end) {
  const text = documents.get(uri)?.text || '';
  return { start: position(text, start), end: position(text, Math.max(start + 1, end)) };
}
function notify(method, params) { self.postMessage({ jsonrpc: '2.0', method, params }); }
function durablePackage(pkg) { return pkg?.grammar?.profile === 'intersect.xtext.native.v65'; }
function currentSources() { return [...documents].map(([uri,d]) => ({uri,text:d.text})); }
function currentVersions() { return Object.fromEntries([...documents].map(([uri,d]) => [uri,d.version])); }
async function prepareEvaluation(pkg, selectedEntry = entryRule) {
  const invoke = await runtime;
  const selected = selectedEntry === null ? {} : {entry_rule:selectedEntry};
  if (!durablePackage(pkg)) return {evaluation:invoke({method:'evaluate',package:pkg,...selected,sources:currentSources()}),analysis:null};
  const analysis=invoke({method:'document/analyze',package:pkg,...selected,sources:currentSources()});
  return {evaluation:analysis.valid_model,analysis};
}
function requireComplete(prepared) {
  if (!prepared.evaluation) throw new Error('The complete document batch must be valid before activating this language or entry. Use document preview to inspect recovered input.');
}
function requireCurrentDocument() {
  if (durablePackage(active?.package) && !documentAnalysis?.valid_model) throw new Error('The current document analysis is invalid. Fix its diagnostics before querying, exporting or mutating the model.');
}
function acceptAnalysis(analysis) { documentAnalysis=analysis;documentAnalysisFailure=null;analysisVersion+=1; }
function acceptEvaluation(result,pkg) {
  evaluation=result;generation+=1;overlay=false;modelOrigin=pkg.grammar?'text':'ecore';publish();
}
async function checkEntry(pkg,selectedEntry) {
  if(selectedEntry===null)return;
  const catalog=(await runtime)({method:'text/entries',package:pkg});
  if(!Object.hasOwn(catalog.entries,selectedEntry))throw new Error(`No emitted no-argument parser entry for ${selectedEntry}`);
}
async function activateLanguage(result,requestedEntry=null) {
  documentCandidate=null;
  if(!result.ok)return;
  const selectedEntry=validateEntryRule(requestedEntry??null);
  await checkEntry(result.package,selectedEntry);
  const prepared=await prepareEvaluation(result.package,selectedEntry);
  if(durablePackage(result.package)&&!prepared.evaluation){
    documentCandidate={token:crypto.randomUUID(),result,entryRule:selectedEntry,revision,analysis:prepared.analysis,versions:currentVersions()};
    notify('intersect/documentPreview',{candidate:documentCandidate.token,analysis:prepared.analysis,versions:documentCandidate.versions,revision,interfaceRevision:`${session}:${revision}`,published:false});
  }
  requireComplete(prepared);
  active=result;entryRule=selectedEntry;revision+=1;
  acceptAnalysis(prepared.analysis);acceptEvaluation(prepared.evaluation,result.package);
}
async function evaluate(reset=false) {
  if(!active)return;
  if(!active.package.grammar && evaluation && !reset)return;
  let prepared;
  try { prepared=await prepareEvaluation(active.package); }
  catch(error) {
    if(!durablePackage(active.package))throw error;
    // An unavailable analysis is not an old successful source snapshot or a
    // fabricated recovered parse. Keep the last valid model, fence its actions,
    // and report availability separately with the actual current versions.
    documentAnalysis=null;
    documentAnalysisFailure={message:String(error?.message??error),versions:currentVersions()};
    analysisVersion+=1;
    publish();return;
  }
  acceptAnalysis(prepared.analysis);
  if(prepared.evaluation)acceptEvaluation(prepared.evaluation,active.package);
  else publish();
}
function unitPosition(text,offset) {
  const lines=text.slice(0,offset).split('\n');return {line:lines.length-1,character:lines.at(-1).length};
}
function publish() {
  const versions=currentVersions();
  if(durablePackage(active?.package))notify('intersect/documentAnalysis',{analysis:documentAnalysis,failure:documentAnalysisFailure,analysisVersion,revision,interfaceRevision:`${session}:${revision}`,generation,versions});
  for(const[uri,doc]of documents){
    const diagnostics=(documentAnalysis?.diagnostics??evaluation.diagnostics).filter(d=>d.source===uri||d.source==='model').map(d=>({range:range(uri,d.start,d.end),severity:1,source:'Intersect',message:d.message}));
    if(documentAnalysisFailure)diagnostics.push({range:{start:{line:0,character:0},end:{line:0,character:0}},severity:1,source:'Intersect',code:'intersect.document.analysisUnavailable',message:documentAnalysisFailure.message});
    const snapshot=documentAnalysis?.snapshots.find(snapshot=>snapshot.document.source.uri===uri);
    for(const issue of snapshot?.syntax_issues??[]){
      const exact={range:{start:unitPosition(doc.text,issue.range.start),end:unitPosition(doc.text,issue.range.end)},severity:1,source:'Intersect',message:issue.message,code:issue.issue_code};
      const duplicate=diagnostics.findIndex(d=>d.message===exact.message&&JSON.stringify(d.range)===JSON.stringify(exact.range));
      if(duplicate>=0)diagnostics[duplicate]=exact;else diagnostics.push(exact);
    }
    notify('textDocument/publishDiagnostics',{uri,version:doc.version,revision,diagnostics});
  }
  notify('intersect/model',{...evaluation,...(durablePackage(active?.package)?{documentAnalysisValid:Boolean(documentAnalysis?.valid_model),documentAnalysisFailure,analysisVersion}:{}),revision,generation,overlay,modelOrigin,entry_rule:entryRule,versions});
}

async function linkedDocumentNode(uri,node) {
  requireCurrentDocument();
  if(overlay||modelOrigin!=='text')throw new Error('Document links require the current source-derived model. Re-evaluate text after an overlay.');
  const snapshot=documentAnalysis.snapshots.find(snapshot=>snapshot.document.source.uri===uri);
  if(!snapshot)throw new Error('No current document snapshot for this source.');
  if(!Number.isSafeInteger(node)||node<0||node>=snapshot.document.nodes.length)throw new Error('Unknown document node.');
  const linked=(await runtime)({method:'document/link',package:active.package,sources:currentSources(),...(entryRule===null?{}:{entry_rule:entryRule}),source:snapshot.document.source,schema_revision:snapshot.schema_revision,resource_revision:snapshot.document.resource_revision,node});
  if(linked.object){
    const current=evaluation.model.elements.filter(element=>element.id===linked.object.id);
    if(current.length!==1||JSON.stringify(current[0])!==JSON.stringify(linked.object))throw new Error('The current model differs from the source-bound document object. Re-evaluate text.');
  }
  return linked;
}
async function targetAt(params) {
  if(durablePackage(active?.package)){
    requireCurrentDocument();
    const uri=params.textDocument.uri,text=documents.get(uri)?.text??'';
    const lines=text.split('\n');
    const offset=lines.slice(0,params.position.line).reduce((n,line)=>n+line.length+1,0)+params.position.character;
    const snapshot=documentAnalysis.snapshots.find(snapshot=>snapshot.document.source.uri===uri);
    if(!snapshot)return null;
    const nodes=snapshot.document.nodes.map((node,index)=>({node,index})).filter(({node})=>node.total_range.start<=offset&&offset<=node.total_range.end).sort((a,b)=>(a.node.total_range.end-a.node.total_range.start)-(b.node.total_range.end-b.node.total_range.start));
    if(!nodes.length)return null;
    const owner=(await linkedDocumentNode(uri,nodes[0].index)).object;
    // Navigation consumes the current shared engine's resolved reference table;
    // the fresh node rebuild above checks this exact package/source batch first.
    // Serialized snapshots never supply a target or replace reference resolution.
    if(offset>0&&offset<text.length&&text.charCodeAt(offset)>=0xdc00&&text.charCodeAt(offset)<=0xdfff&&text.charCodeAt(offset-1)>=0xd800&&text.charCodeAt(offset-1)<=0xdbff)throw new Error('Navigation position splits a Unicode scalar.');
    const byteOffset=new TextEncoder().encode(text.slice(0,offset)).length;
    const references=evaluation.references.filter(reference=>reference.uri===uri&&reference.start<=byteOffset&&byteOffset<reference.end);
    if(references.length){
      const targets=new Set(references.map(reference=>reference.target));
      if(targets.size!==1)throw new Error('Ambiguous current source reference.');
      const target=evaluation.model.elements.filter(element=>element.id===references[0].target);
      if(target.length!==1)throw new Error('The current source reference has no unique model target.');
      return target[0];
    }
    return owner;
  }


  if (!evaluation || modelOrigin === 'import' || !active?.package.grammar) return null;
  const uri = params.textDocument.uri;
  const text = documents.get(uri)?.text || '';
  const offset = byteOffset(text, params.position);
  const reference = evaluation.references.find(r => r.uri === uri && r.start <= offset && offset <= r.end);
  if (reference) return evaluation.model.elements.find(e => e.id === reference.target);
  return evaluation.model.elements.filter(e => e.resource === uri && e.start <= offset && offset <= e.end).sort((a, b) => (a.end - a.start) - (b.end - b.start))[0];
}
async function handle(method, params = {}) {
  if (method === 'exit') { self.close(); return null; }
  if (method === 'initialize') {
    await runtime; initialized = true; stopped = false;
    return { capabilities: { positionEncoding: 'utf-16', textDocumentSync: 1, completionProvider: { triggerCharacters: [' '] }, hoverProvider: true, definitionProvider: true }, serverInfo: { name: 'Intersect browser language server', version: '0.1.0' } };
  }
  if (!initialized || stopped) throw new Error('Language server is not initialized.');
  switch (method) {
    case 'initialized': return null;
    case '$/cancelRequest': {
      const record = functionRequests.get(params.id);
      if (record) advanceFunction(record, true);
      return null;
    }
    case 'shutdown':
      stopped = true;
      for (const record of [...functionRequests.values()]) advanceFunction(record, true);
      return null;
    case 'intersect/compileRcoreOperation': return (await runtime)({...params,method:'rcore/compileOperation'});
    case 'intersect/invokeRcoreOperation': return (await runtime)({...params,method:'rcore/invokeOperation'});
    case 'intersect/inspectRcoreStructure': return (await runtime)({...params,method:'rcore/inspectStructure'});
    case 'intersect/inferredStructurePreview': return (await runtime)({...params,method:params.inference_context == null ? 'inferred/structurePreview' : 'inferred/structurePreviewWithContext'});
    case 'intersect/inferredProviderCandidates': return (await runtime)({...params,method:'inferred/providerCandidates'});
    case 'intersect/selectInferredProviders': return (await runtime)({...params,method:'inferred/selectProviders'});
    case 'intersect/lowerRcoreStructure': return (await runtime)({method:'rcore/lowerStructure',...params});
    case 'intersect/parseRcore': return (await runtime)({ method: 'rcore_behavior/parse', ...params });
    case 'intersect/bindRcore': return (await runtime)({ method: 'rcore_behavior/bind', ...params });
    case 'intersect/stepFeatureMap': return (await runtime)({ ...params, method: 'feature-map/step', package: active?.package });
    case 'intersect/bindFeatureReference': return (await runtime)({ ...params, method: 'feature-map/reference-bind', package: active?.package });
    case 'intersect/readFeatureReference': return (await runtime)({ ...params, method: 'feature-map/reference-read', package: active?.package });
    case 'intersect/stepCollection': return (await runtime)({ ...params, method: 'collection/step', package: active?.package });
    case 'intersect/bindCollectionReference': return (await runtime)({ ...params, method: 'collection/reference-bind', package: active?.package });
    case 'intersect/readCollectionReference': return (await runtime)({ ...params, method: 'collection/reference-read', package: active?.package });
    case 'intersect/stepIterator': return (await runtime)({ ...params, method: 'iterator/step', package: active?.package });
    case 'intersect/readResource': return (await runtime)({ ...params, method: 'resource/read', package: active?.package });
    case 'intersect/resolveResource': return (await runtime)({ ...params, method: 'resource/resolve', package: active?.package });
    case 'intersect/collectDiagnostics': return (await runtime)({ ...params, method: 'diagnostic/collect', package: active?.package });
    case 'intersect/compileFunctionRcore': return (await runtime)({ ...params, method: 'rcore_behavior/compileFunction' });
    case 'intersect/evaluateScalarRcore': return (await runtime)({ ...params, method: 'rcore_behavior/evaluateScalar' });
    case 'intersect/checkRcore': return (await runtime)({ method: 'rcore_behavior/check', ...params });
    case 'intersect/ecoreMembers': return (await runtime)({ method: 'ecore/members', ...params });
    case 'intersect/constructorContract': return (await runtime)({ ...params, method: 'conversion/constructor' });
    case 'intersect/materializeGenericTypes': return (await runtime)({ ...params, method: 'ecore/materialize' });
    case 'intersect/compareGenericOperationOverride': return (await runtime)({ ...params, method: 'ecore/generic-operation-override' });
    case 'intersect/resolveGenericOperations': return (await runtime)({ ...params, method: 'ecore/generic-operation-selection' });
    case 'intersect/instantiateGenericOperation': return (await runtime)({ ...params, method: 'ecore/generic-operation-types' });
    case 'intersect/instantiateRuntimeOperation': return (await runtime)({ ...params, method: 'ecore/runtime-operation-types' });
    case 'intersect/runtimeMemberTypes': return (await runtime)({ ...params, method: 'ecore/runtime-member-types' });
    case 'intersect/appliedConversion': return (await runtime)({ ...params, method: 'conversion/applied' });
    case 'intersect/instantiateGenericDatatype': return (await runtime)({ ...params, method: 'ecore/generic-datatype-application' });
    case 'intersect/instantiateGenericMembers': return (await runtime)({ ...params, method: 'ecore/generic-instantiate-members' });
    case 'intersect/genericMemberTypes': return (await runtime)({ ...params, method: 'ecore/generic-member-types' });
    case 'intersect/checkGenericApplicationBounds': return (await runtime)({ ...params, method: 'ecore/generic-application-bounds' });
    case 'intersect/genericInheritance': return (await runtime)({ ...params, method: 'ecore/generic-inheritance' });
    case 'intersect/substituteGenericType': return (await runtime)({ ...params, method: 'ecore/generic-substitute' });
    case 'intersect/checkGenericSignatures': return (await runtime)({ ...params, method: 'ecore/generic-signatures' });
    case 'intersect/checkGenericType': return (await runtime)({ ...params, method: 'ecore/generic-type' });
    case 'intersect/checkGenericParameter': return (await runtime)({ ...params, method: 'ecore/generic-parameter' });
    case 'exit': self.close(); return null;
    case 'intersect/definitionInspect': return (await runtime)({ method: 'definition/inspect', ...params });
    case 'intersect/definitionEdit': return (await runtime)({ method: 'definition/edit', ...params });
    case 'intersect/migrateEcore': return (await runtime)({...params, method:'ecore/migrate'});
    case 'intersect/migrateEcoreBundle': return (await runtime)({...params, method:'ecore/migrateBundle'});
    case 'intersect/migrateEcoreCompilation': return (await runtime)({...params, method:'ecore/migrateCompilation'});
    case 'intersect/compileRcore': {
      const result = (await runtime)({...params, method:'compile_rcore'});
      await activateLanguage({...result, bindings:params.bindings},params.entry_rule);
      return {...result, revision, entry_rule:entryRule};
    }
    case 'intersect/selectEntry': {
      if (!active) throw new Error('No active language package.');
      if (params.interfaceRevision !== `${session}:${revision}`) throw new Error('The active language changed. Retry entry selection.');
      const selectedEntry=validateEntryRule(params.entry_rule);
      await checkEntry(active.package,selectedEntry);
      const prepared=await prepareEvaluation(active.package,selectedEntry);
      requireComplete(prepared);
      entryRule=selectedEntry;acceptAnalysis(prepared.analysis);acceptEvaluation(prepared.evaluation,active.package);
      return {entry_rule:entryRule,interfaceRevision:`${session}:${revision}`,generation,evaluation};
    }
    case 'intersect/entryRules': {
      if (!active) throw new Error('No active language package.');
      const catalog = (await runtime)({ method:'text/entries', package:active.package });
      return { ...catalog, interfaceRevision:`${session}:${revision}` };
    }
    case 'intersect/documentLink': {
      requireCurrentDocument();
      if(!durablePackage(active?.package))throw new Error('Document links require the explicit durable profile.');
      if(params.interfaceRevision!==`${session}:${revision}`||params.generation!==generation||params.analysisVersion!==analysisVersion)throw new Error('Stale document locator: language, model or analysis changed.');
      const doc=documents.get(params.uri);
      if(!doc||params.version!==doc.version)throw new Error('Stale document source version.');
      const snapshot=documentAnalysis.snapshots.find(snapshot=>snapshot.document.source.uri===params.uri);
      if(!snapshot||params.source?.uri!==snapshot.document.source.uri||params.source?.sha256!==snapshot.document.source.sha256)throw new Error('Document source identity changed.');
      return {...await linkedDocumentNode(params.uri,params.node),interfaceRevision:`${session}:${revision}`,generation,analysisVersion,version:doc.version};
    }
    case 'intersect/documentElementSource': {
      requireCurrentDocument();
      if(!durablePackage(active?.package))throw new Error('Document element sources require the explicit durable profile.');
      if(params.interfaceRevision!==`${session}:${revision}`||params.generation!==generation||params.analysisVersion!==analysisVersion)throw new Error('Stale document locator: language, model or analysis changed.');
      const versions=currentVersions();
      if(!params.versions||Object.keys(params.versions).length!==Object.keys(versions).length||Object.entries(versions).some(([uri,version])=>params.versions[uri]!==version))throw new Error('Stale document batch versions.');
      const doc=documents.get(params.uri);
      if(!doc||doc.version!==params.version)throw new Error('Stale document source version.');
      const elements=evaluation.model.elements.filter(element=>element.id===params.element&&element.resource===params.uri);
      if(elements.length!==1)throw new Error('The document element has no unique current source object.');
      const snapshots=documentAnalysis.snapshots.filter(snapshot=>snapshot.document.source.uri===params.uri);
      if(snapshots.length!==1)throw new Error('The document element has no unique current source snapshot.');
      const snapshot=snapshots[0];
      const owners=snapshot.semantic_objects.map((owner,index)=>({owner,index})).filter(({owner})=>owner.id===params.element);
      if(owners.length!==1)throw new Error('The document element has no unique retained semantic identity.');
      const nodes=snapshot.document.nodes.map((node,index)=>({node,index})).filter(({node})=>node.kind==='composite'&&node.data.direct_semantic_association&&node.data.semantic_object===owners[0].index);
      if(nodes.length!==1)throw new Error('The document element has no unique directly associated source node.');
      const linked=await linkedDocumentNode(params.uri,nodes[0].index);
      if(!linked.object||JSON.stringify(linked.object)!==JSON.stringify(elements[0]))throw new Error('The rebuilt document source differs from the current element.');
      return {...linked,interfaceRevision:`${session}:${revision}`,generation,analysisVersion,version:doc.version,versions};
    }
    case 'intersect/documentPreview': {
      const pending=params.candidate===undefined?null:documentCandidate;
      if(params.candidate!==undefined&&(!pending||pending.token!==params.candidate||pending.revision!==revision))throw new Error('The unpublished document language is obsolete. Compile again.');
      const pkg=pending?.result.package??active?.package;
      if(!durablePackage(pkg))throw new Error('Document preview requires the explicit durable profile.');
      if(params.interfaceRevision!==`${session}:${revision}`)throw new Error('The active language changed. Retry document preview.');
      const analysis=(await runtime)({method:'document/analyze',package:pkg,entry_rule:params.entry_rule===undefined?(pending?.entryRule??entryRule):params.entry_rule,sources:params.sources??currentSources()});
      return {analysis,published:false,...(pending?{candidate:pending.token}:{}),interfaceRevision:`${session}:${revision}`,versions:currentVersions()};
    }
    case 'intersect/evaluateEntry': {
      if (!active) throw new Error('No active language package.');
      if (params.interfaceRevision !== `${session}:${revision}`) throw new Error('The active language changed. Retry the entry invocation.');
      if (typeof params.entry_rule !== 'string') throw new Error('entry_rule must be a parser entry name.');
      const sources = params.sources ?? [...documents].map(([uri, d]) => ({uri,text:d.text}));
      // A source-bound invocation is a preview, not an active model publication.
      // Preview leaves the selected entry and published world unchanged.
      return (await runtime)({method:durablePackage(active.package)?'document/analyze':'evaluate',package:active.package,entry_rule:params.entry_rule,sources});
    }
    case 'intersect/compileBundle': {
      const result = (await runtime)({ method: 'compile_bundle', ...params });
      await activateLanguage(result,params.entry_rule);
      return { ...result, revision, entry_rule:entryRule };
    }
    case 'intersect/prepareInferred': {
      inferredCandidate = null;
      const method=params.inference_context == null ? 'compile_inferred_with_origins' : 'compile_inferred_with_context';
      const result = (await runtime)({ ...params, method });
      if (!result.ok) return { ...result, revision, entry_rule:entryRule };
      const selectedEntry=validateEntryRule(params.entry_rule??null);
      await checkEntry(result.package,selectedEntry);
      const token=crypto.randomUUID();
      inferredCandidate={token,result,entryRule:selectedEntry,revision};
      return {...result,candidate:token,revision,entry_rule:selectedEntry};
    }
    case 'intersect/discardInferred': {
      if (inferredCandidate?.token === params.candidate) inferredCandidate=null;
      return null;
    }
    case 'intersect/activateInferred': {
      const candidate=inferredCandidate;
      if(!candidate || candidate.token!==params.candidate || candidate.revision!==revision)throw new Error('The prepared inferred language is obsolete. Compile again.');
      inferredCandidate=null;
      await activateLanguage(candidate.result,candidate.entryRule);
      return {...candidate.result,revision,entry_rule:entryRule};
    }
    case 'intersect/compileInferred': {
      inferredCandidate=null;
      const method=params.inference_context == null ? 'compile_inferred_with_origins' : 'compile_inferred_with_context';
      const result = (await runtime)({ ...params, method });
      await activateLanguage(result,params.entry_rule);
      return { ...result, revision, entry_rule:entryRule };
    }
    case 'intersect/packageImport': {
      if (params.revision !== `${session}:${revision}`) throw new Error('The active language changed. Retry package import.');
      const explicitProviders=['conversions','applied_conversions','constructors'].some(key=>Object.hasOwn(params,key));
      if(params.inference_context != null && explicitProviders)throw new Error('Conflicting ordinary and inferred package provider contexts.');
      const method=params.inference_context != null ? 'package/import_with_context' : explicitProviders ? 'package/import_with_conversions' : 'package/import';
      const imported = (await runtime)({ ...params, method });
      const result = { ok: true, package: imported.package, keywords: imported.keywords };
      await activateLanguage(result,params.entry_rule);
      return { ...result, manifest: imported.manifest, revision, entry_rule:entryRule };
    }
    case 'textDocument/didOpen': {
      const d = params.textDocument; documents.set(d.uri, { text: d.text, version: d.version }); await evaluate(); return null;
    }
    case 'textDocument/didChange': {
      const old = documents.get(params.textDocument.uri);
      if (old && params.textDocument.version <= old.version) return null;
      if (params.contentChanges.some(c => c.range)) throw new Error('Only full document synchronization is supported.');
      documents.set(params.textDocument.uri, { text: params.contentChanges.at(-1).text, version: params.textDocument.version }); await evaluate(); return null;
    }
    case 'textDocument/didClose': {
      documents.delete(params.textDocument.uri);
      notify('textDocument/publishDiagnostics', { uri: params.textDocument.uri, diagnostics: [] }); await evaluate(); return null;
    }
    case 'textDocument/completion': {
      if (!active || !active.package.grammar) return [];
      const items = active.keywords.filter(k => /^[\p{L}_]/u.test(k)).map(label => ({ label, kind: 14, detail: 'Grammar keyword' }));
      for (const e of evaluation?.model.elements || []) {
        if (e.values.name) items.push({ label: e.values.name, kind: 6, detail: `${e.class} · ${e.resource.split('/').at(-1)}` });
      }
      return { isIncomplete: false, items };
    }
    case 'textDocument/hover': {
      const e = await targetAt(params);
      if (!e) return null;
      return { contents: { kind: 'plaintext', value: `${e.class} ${e.values.name || ''}\n${e.type_identity}\n${JSON.stringify({ ...e.values, ...evaluation.derived[e.id] }, null, 2)}` } };
    }
    case 'textDocument/definition': {
      const e = await targetAt(params);
      return e ? [{ uri: e.resource, range: range(e.resource, e.start, e.end) }] : [];
    }
    case 'intersect/inspect': return {...(documentCandidate?{documentPreview:{candidate:documentCandidate.token,analysis:documentCandidate.analysis,versions:documentCandidate.versions,revision:documentCandidate.revision,published:false}}:{}),...(durablePackage(active?.package)?{documentAnalysis,documentAnalysisFailure,analysisVersion,versions:currentVersions()}:{}), entry_rule:entryRule, revision, interfaceRevision: `${session}:${revision}`, generation, overlay, modelOrigin, evaluation, package: active?.package || null };
    case 'intersect/query': {
      requireCurrentDocument();
      if (!active || !evaluation || evaluation.diagnostics.length) throw new Error('Fix model diagnostics before querying.');
      if (params.interfaceRevision !== `${session}:${revision}` || params.generation !== generation) throw new Error('Stale query: language or model changed. Run again.');
      const method = active.package.schema.conversion_contracts?.length ? 'query/execute_with_conversions' : 'query/execute';
      const result = (await runtime)({ method, package: active.package, model: evaluation.model, query: params.query, parameters: params.parameters || {} });
      return { ...result, interfaceRevision: `${session}:${revision}`, generation };
    }
    case 'intersect/modelFormats': return (await runtime)({ method: 'model/formats' });
    case 'intersect/modelExport': {
      requireCurrentDocument();
      if (!active || !evaluation || evaluation.diagnostics.length) throw new Error('A valid model is required for export.');
      return (await runtime)({ method: 'model/export', package: active.package, model: evaluation.model, format: params.format, ...(params.selected === undefined ? {} : { selected: params.selected, closure: params.closure }) });
    }
    case 'intersect/modelImport': {
      requireCurrentDocument();
      if (!active) throw new Error('Compile a language before importing a model.');
      if (params.interfaceRevision !== `${session}:${revision}` || params.generation !== generation) throw new Error('The language or model changed while files were being selected. Retry import.');
      if (transactions.size) throw new Error('Finish the running model transaction before importing.');
      const imported = (await runtime)({ method: 'model/import', package: active.package, resources: params.resources, format: params.format, ...(params.selected === undefined ? {} : { selected: params.selected, model: evaluation?.model }) });
      evaluation = imported; generation += 1; overlay = true; modelOrigin = 'import'; publish();
      return { generation, evaluation };
    }
    case 'intersect/packageExport': {
      requireCurrentDocument();
      if (!active) throw new Error('Compile a language before exporting a package.');
      if (!active.package.grammar) throw new Error('Language ZIP export requires Xtext syntax. Save the Rcore project instead.');
      if (params.revision !== `${session}:${revision}`) throw new Error('The active language changed. Reopen the export dialog.');
      if (active.package.xtext_inference) {
        const providers=conversionSelections(active.package);
        const behavior=active.package.xtext_inference.behavior;
        const computed_settings=active.package.grammar?.computed_settings ?? [];
        const context=behavior || computed_settings.length ? {version:2,...providers,behavior:behavior ?? null,computed_settings} : {version:1,...providers};
        return (await runtime)({ method:'package/export', inference:active.package.xtext_inference, metadata:params.metadata, ...(Object.keys(providers).length || context.version===2 ? {inference_context:context} : {}) });
      }
      if (!active.package.compilation) throw new Error('Missing retained compilation inputs.');
      const providers = conversionSelections(active.package);
      return (await runtime)({ method:'package/export', compilation:active.package.compilation, xtext_profile:active.package.grammar.profile, metadata:params.metadata, ...providers });
    }
    case 'intersect/transactionBegin': {
      requireCurrentDocument();
      if (!active || !evaluation || (!params.disposable && evaluation.diagnostics.length)) throw new Error('Fix model diagnostics before starting a transaction.');
      if (transactions.size) throw new Error('A model transaction is already running.');
      const context = { id: `${session}/tx-${++transactionNumber}`, language_revision: `${session}:${revision}`, base_generation: generation };
      const disposable = params.disposable === true;
      const fixture = disposable ? (await runtime)({ method: 'script/check', package: active.package, model: { schema_revision: active.package.schema.revision, resources: {}, elements: [] } }) : evaluation;
      const transaction = (await runtime)({ method: 'script/report', package: active.package, before: fixture.model, model: fixture.model, context });
      transactions.set(context.id, { context, package: active.package, before: fixture.model, transaction, disposable });
      return { revision, interfaceRevision: context.language_revision, generation, evaluation: fixture, package: active.package, bindings: active.bindings, transaction, disposable };
    }
    case 'intersect/transactionFinish': {
      const pending = transactions.get(params.id);
      if (!pending) throw new Error('Transaction is already closed or belongs to another runtime.');
      const report = params.transaction || pending.transaction;
      if (report.context.id !== pending.context.id || report.context.language_revision !== pending.context.language_revision || report.context.base_generation !== pending.context.base_generation) throw new Error('Transaction context mismatch.');
      const receipt = (await runtime)({ method: 'script/finish', transaction: report, status: params.status, reason: params.reason, changes_complete: params.changes_complete });
      transactions.delete(params.id);
      return receipt;
    }
    case 'intersect/testFinish': {
      requireCurrentDocument();
      if (!active || params.interfaceRevision !== `${session}:${revision}` || params.generation !== generation) throw new Error('Stale test: language or model changed while Python was running.');
      const pending = transactions.get(params.transactionId);
      if (!pending?.disposable) throw new Error('Unknown or non-test transaction.');
      const checked = (await runtime)({ method: 'script/check', package: pending.package, model: params.model });
      const report = (await runtime)({ method: 'script/report', package: pending.package, before: pending.before, model: checked.model, context: pending.context });
      const transaction = (await runtime)({ method: 'script/finish', transaction: report, status: 'rolled_back', reason: 'Tests passed; disposable fixture discarded.', changes_complete: true });
      transactions.delete(params.transactionId);
      return { disposable: true, evaluation: checked, transaction, generation };
    }
    case 'intersect/scriptCommit': {
      requireCurrentDocument();
      if (!active || params.interfaceRevision !== `${session}:${revision}` || params.generation !== generation) throw new Error('Stale transaction: language or model changed while Python was running.');
      const pending = transactions.get(params.transactionId);
      if (!pending) throw new Error('Unknown or closed model transaction.');
      if (pending.disposable) throw new Error('Disposable test fixtures cannot be published.');
      const checked = (await runtime)({ method: 'script/publish', package: pending.package, before: pending.before, current: evaluation.model, model: params.model, context: pending.context, revision: `${session}:${revision}`, generation });
      const { transaction, ...result } = checked;
      transactions.delete(params.transactionId);
      evaluation = { ...result, references: evaluation.references };
      generation += 1; overlay = true; publish();
      return { generation, evaluation, transaction };
    }
    default: throw new Error(`Unsupported LSP method: ${method}`);
  }
}
let queue = Promise.resolve();
self.onmessage = ({ data }) => {
  queue = queue.then(async () => {
    try {
      if (data.method === 'intersect/evaluateFunctionRcore') { await startFunction(data); return; }
      const result = await handle(data.method, data.params);
      if (data.id !== undefined) self.postMessage({ jsonrpc: '2.0', id: data.id, result });
    } catch (error) {
      if (data.id !== undefined) self.postMessage({ jsonrpc: '2.0', id: data.id, error: { code: -32603, message: error.message } });
      else notify('window/logMessage', { type: 1, message: error.message });
    }
  });
};
