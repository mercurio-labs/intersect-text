// Handwritten project transport/state only. Native selection and replay own semantics.
import {sourceEol,sourceKey} from './source-drafts.js';
import {validateEntryRule} from './entry-selection.js';
import {validateInferenceProject} from './inference-graph.js';
const encoder=new TextEncoder();
const object=value=>value!==null&&typeof value==='object'&&!Array.isArray(value);
const exact=(value,keys)=>object(value)&&Object.keys(value).every(key=>keys.includes(key));
const own=(value,key)=>Object.hasOwn(value,key);
const identity=value=>exact(value,['uri','sha256'])&&typeof value.uri==='string'&&value.uri.length>0&&!/[\u0000-\u001f\u007f-\u009f]/u.test(value.uri)&&typeof value.sha256==='string'&&/^[a-f0-9]{64}$/.test(value.sha256);
const clone=value=>value===null?null:structuredClone(value);
export function validateProviderDraft(value){
  if(!exact(value,['version','selection'])||value.version!==1||typeof value.selection!=='string'||encoder.encode(value.selection).length>256000)throw new Error('Invalid inferred provider draft (256 KB limit).');
  sourceEol(value.selection);return value;
}
// The original request stays version 1. This validates transport and exact graph
// references, not source digests, language validity or executable provider authority.
export function validateRecoveryInference(inference){
  validateInferenceProject({inputs:inference,drafts:[]});
  const sources=new Map();
  for(const source of inference.bundle.sources){
    if(!exact(source,['id','kind','text','dependencies'])||!identity(source.id)||!source.dependencies.every(identity))throw new Error('Invalid inferred recovery source identity.');
    sources.set(sourceKey(source.id),source);
  }
  if(!identity(inference.grammar))throw new Error('Invalid inferred recovery grammar identity.');
  for(const source of sources.values()){
    const seen=new Set();
    for(const dependency of source.dependencies){
      const key=sourceKey(dependency);
      if(!sources.has(key)||seen.has(key))throw new Error('Missing, stale or duplicate inferred recovery dependency.');
      seen.add(key);
    }
  }
  const namespaces=new Set();
  for(const binding of inference.bundle.namespaces){
    if(!exact(binding,['namespace','source'])||typeof binding.namespace!=='string'||!binding.namespace||!identity(binding.source)||!sources.has(sourceKey(binding.source))||namespaces.has(binding.namespace))throw new Error('Invalid or ambiguous inferred recovery namespace binding.');
    namespaces.add(binding.namespace);
  }
  return inference;
}
export function validateInferenceContext(value,inference){
  if(!exact(value,['version','conversions','applied_conversions','constructors','behavior','computed_settings'])||![1,2].includes(value.version))throw new Error('Unsupported inferred context version or fields.');
  for(const key of ['conversions','applied_conversions','constructors'])if(own(value,key)&&(!Array.isArray(value[key])||!value[key].every(object)))throw new Error('Invalid inferred context catalog.');
  const extended=own(value,'behavior')||own(value,'computed_settings');
  if(value.version===1&&extended||value.version===2&&(!own(value,'behavior')||!own(value,'computed_settings')))throw new Error('Unsupported inferred context version fields.');
  if(value.version===2){
    if(value.behavior!==null&&!identity(value.behavior)||!Array.isArray(value.computed_settings)||!value.computed_settings.every(object))throw new Error('Invalid inferred computed context.');
    if(value.behavior!==null&&inference&&!inference.bundle.sources.some(source=>['rcore','rcore_behavior'].includes(source.kind)&&sourceKey(source.id)===sourceKey(value.behavior)))throw new Error('Inferred recovery selects a missing or stale behavior source.');
  }
  // Nested contracts are carried verbatim. Their declarations can refer to native
  // generated definitions, and only the shared source compiler can qualify them.
  return value;
}
function validateRecoveryRecipe(value,contextRequired){
  if(!exact(value,['inference','inference_context','entryRule'])||!own(value,'entryRule'))throw new Error('Invalid inferred source/provider recovery recipe.');
  validateEntryRule(value.entryRule);
  validateRecoveryInference(value.inference);
  if(contextRequired||own(value,'inference_context'))validateInferenceContext(value.inference_context,value.inference);
  if(encoder.encode(JSON.stringify(value)).length>2000000)throw new Error('Inferred recovery exceeds the 2 MB project limit.');
  return value;
}
// v5 retains its original required-context contract. v6 also carries ordinary
// inference by omitting context entirely; null is never an absent context.
export const validateInferenceRecovery=value=>validateRecoveryRecipe(value,true);
export const validateDurableInferenceRecovery=value=>validateRecoveryRecipe(value,false);
export function createInferenceProviderState(){
  let draft=null,recovery=null;
  return{
    providerSnapshot:()=>clone(draft),
    providerDraft:()=>draft?.selection??null,
    restoreProviders(value){if(value!==null)validateProviderDraft(value);draft=clone(value);},
    setProviderDraft(selection){const value={version:1,selection};validateProviderDraft(value);draft=value;},
    recovery:()=>clone(recovery),
    restoreRecovery(value){if(value!==null)validateDurableInferenceRecovery(value);recovery=clone(value);},
    // Main calls this only after native activation and its project/session fence.
    // Neither draft snapshots nor cached/generated package DTOs are accepted here.
    captureSuccessfulRecovery(activation){
      if(!exact(activation,['ok','inference','inference_context','entryRule'])||activation.ok!==true)throw new Error('A successful native activation is required for inferred recovery.');
      const value={inference:activation.inference,...(own(activation,'inference_context')?{inference_context:activation.inference_context}:{}),entryRule:activation.entryRule};
      validateDurableInferenceRecovery(value);recovery=structuredClone(value);return clone(recovery);
    },
    clear(){draft=null;recovery=null;},
  };
}
