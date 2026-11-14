// Handwritten source-graph transport. Rust alone validates language semantics.
import {sourceEol,sourceIdentity,sourceKey} from './source-drafts.js';
const encoder=new TextEncoder();
const identity=id=>id&&typeof id.uri==='string'&&typeof id.sha256==='string'&&/^[a-f0-9]{64}$/.test(id.sha256);
const exact=(value,keys)=>value&&typeof value==='object'&&!Array.isArray(value)&&Object.keys(value).every(k=>keys.includes(k));
export function validateInferenceProject(record){
  const inputs=record?.inputs,bundle=inputs?.bundle;
  if(!exact(record,['inputs','drafts'])||!exact(inputs,['version','bundle','grammar'])||inputs.version!==1||!identity(inputs.grammar)||!exact(bundle,['version','sources','namespaces'])||bundle.version!==1||!Array.isArray(bundle.sources)||!Array.isArray(bundle.namespaces)||!Array.isArray(record.drafts))throw new Error('Invalid retained inference source project.');
  const sources=new Map();
  for(const source of bundle.sources){
    if(!identity(source.id)||!['rcore','xtext','ecore','rcore_behavior'].includes(source.kind)||typeof source.text!=='string'||encoder.encode(source.text).length>256000||!Array.isArray(source.dependencies)||!source.dependencies.every(identity))throw new Error('Invalid retained inference source.');
    sourceEol(source.text);const key=sourceKey(source.id);if(sources.has(key))throw new Error('Duplicate inference source identity.');sources.set(key,source);
  }
  const entry=sourceKey(inputs.grammar);if(sources.get(entry)?.kind!=='xtext')throw new Error('Inference entry must select a retained Xtext source.');
  const seen=new Set();
  for(const draft of record.drafts){
    if(!exact(draft,['source','text'])||!identity(draft.source)||typeof draft.text!=='string'||encoder.encode(draft.text).length>256000)throw new Error('Invalid inference source draft.');
    const key=sourceKey(draft.source);if(key===entry||!sources.has(key)||seen.has(key))throw new Error('Ambiguous inference source draft.');seen.add(key);sourceEol(draft.text);
  }
  // Rust validates original digests, graph references and semantics at compile.
  return record;
}

const same=(a,b)=>identity(a)&&identity(b)&&sourceKey(a)===sourceKey(b);
function textValue(text){if(typeof text!=='string'||encoder.encode(text).length>256000)throw new Error('Source exceeds the 256 KB limit.');sourceEol(text);return text;}
function opaque(value,label){if(typeof value!=='string'||!value.trim()||/[\u0000-\u001f\u007f-\u009f]/u.test(value))throw new Error(`${label} must be nonempty and contain no controls.`);return value;}
export function inferenceGraphState(record,entryText,selectedSource=record?.inputs.grammar){
  validateInferenceProject(record);textValue(entryText);
  return {record:structuredClone(record),entryText,selectedSource:structuredClone(selectedSource)};
}
export function inferenceSourceTexts(state){
  const result=new Map(state.record.inputs.bundle.sources.map(s=>[sourceKey(s.id),s.text]));
  for(const draft of state.record.drafts)result.set(sourceKey(draft.source),draft.text);
  result.set(sourceKey(state.record.inputs.grammar),state.entryText);return result;
}
function fromTexts(inputs,texts,selectedSource){
  const entry=sourceKey(inputs.grammar),drafts=[];
  for(const source of inputs.bundle.sources){const text=texts.get(sourceKey(source.id))??source.text;if(sourceKey(source.id)!==entry&&text!==source.text)drafts.push({source:structuredClone(source.id),text});}
  return inferenceGraphState({inputs,drafts},texts.get(entry)??inputs.bundle.sources.find(s=>same(s.id,inputs.grammar)).text,selectedSource);
}
// Undo changes graph metadata; newer text for surviving exact identities wins.
export function retainInferenceDrafts(target,current){
  const texts=inferenceSourceTexts(target),latest=inferenceSourceTexts(current);
  for(const key of texts.keys())if(latest.has(key))texts.set(key,latest.get(key));
  return fromTexts(structuredClone(target.record.inputs),texts,target.selectedSource);
}
export async function mutateInferenceGraph(state,action){
  inferenceGraphState(state.record,state.entryText,state.selectedSource);
  const inputs=structuredClone(state.record.inputs),texts=inferenceSourceTexts(state);
  let selected=state.selectedSource;
  const find=id=>{const source=inputs.bundle.sources.find(s=>same(s.id,id));if(!source)throw new Error('The selected exact source is no longer retained.');return source;};
  switch(action.type){
    case 'add': {
      opaque(action.uri,'Source URI');textValue(action.text);
      if(!['xtext','rcore','ecore','rcore_behavior'].includes(action.kind))throw new Error('Unsupported source kind.');
      const id=await sourceIdentity(action.uri,action.text);
      if(inputs.bundle.sources.some(s=>same(s.id,id)))throw new Error('That exact source identity is already retained.');
      inputs.bundle.sources.push({id,kind:action.kind,text:action.text,dependencies:[]});texts.set(sourceKey(id),action.text);selected=id;break;
    }
    case 'entry': {
      const source=find(action.source);if(source.kind!=='xtext')throw new Error('The entry must select a retained Xtext source.');inputs.grammar=structuredClone(source.id);break;
    }
    case 'dependencies': {
      const source=find(action.source);
      if(!Array.isArray(action.dependencies)||!action.dependencies.every(identity))throw new Error('Invalid dependency identities.');
      source.dependencies=structuredClone(action.dependencies);break;
    }
    case 'remove': {
      const source=find(action.source);
      if(same(source.id,inputs.grammar))throw new Error('Choose another entry grammar before removing this source.');
      const incoming=inputs.bundle.sources.filter(s=>!same(s.id,source.id)&&s.dependencies.some(d=>same(d,source.id)));
      const bindings=inputs.bundle.namespaces.filter(n=>same(n?.source,source.id));
      if(incoming.length||bindings.length)throw new Error('Remove incoming dependency and namespace bindings explicitly before removing this source.');
      inputs.bundle.sources=inputs.bundle.sources.filter(s=>!same(s.id,source.id));texts.delete(sourceKey(source.id));if(same(selected,source.id))selected=inputs.grammar;break;
    }
    case 'namespaceAdd':
    case 'namespaceEdit': {
      opaque(action.namespace,'Namespace');const source=find(action.source);
      const binding={namespace:action.namespace,source:structuredClone(source.id)};
      if(action.type==='namespaceAdd')inputs.bundle.namespaces.push(binding);
      else {if(!Number.isInteger(action.index)||!inputs.bundle.namespaces[action.index])throw new Error('The namespace binding is no longer retained.');inputs.bundle.namespaces[action.index]={...inputs.bundle.namespaces[action.index],...binding};}
      break;
    }
    case 'namespaceRemove':
      if(!Number.isInteger(action.index)||action.index<0||action.index>=inputs.bundle.namespaces.length)throw new Error('The namespace binding is no longer retained.');inputs.bundle.namespaces.splice(action.index,1);break;
    default:throw new Error('Unknown source-graph edit.');
  }
  const result=fromTexts(inputs,texts,selected);
  if(encoder.encode(JSON.stringify(result.record)).length>2000000)throw new Error('Inference source graph exceeds the 2 MB project limit.');
  return result;
}
export function createInferenceGraphHistory(limit=20){
  const undo=[],redo=[];
  const push=(stack,state)=>{stack.push(structuredClone(state));if(stack.length>limit)stack.shift();};
  return {
    get canUndo(){return undo.length>0;},get canRedo(){return redo.length>0;},
    clear(){undo.length=0;redo.length=0;},
    record(before){push(undo,before);redo.length=0;},
    move(direction,current){const from=direction==='undo'?undo:direction==='redo'?redo:null;if(!from?.length)throw new Error('No graph change to restore.');const next=retainInferenceDrafts(from.at(-1),current);from.pop();push(direction==='undo'?redo:undo,current);return next;},
  };
}
