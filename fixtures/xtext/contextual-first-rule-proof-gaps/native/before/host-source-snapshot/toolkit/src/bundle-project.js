import {validateEntryRule} from './entry-selection.js';
// Handwritten source-envelope transport only. Rust validates identities and meaning.
import {sourceKey} from './source-drafts.js';
const encoder=new TextEncoder();
export function validateBundleProject(compilation) {
 const bundle=compilation?.bundle;
 const identity=id=>id&&typeof id.uri==='string'&&typeof id.sha256==='string'&&/^[a-f0-9]{64}$/.test(id.sha256);
 if(!bundle||bundle.version!==1||!Array.isArray(bundle.sources)||!Array.isArray(bundle.namespaces)||!['ecore','rcore_behavior'].every(key=>identity(compilation[key]))||(compilation.xtext!==null&&!identity(compilation.xtext))||typeof compilation.namespace!=='string')throw new Error('Invalid retained definition bundle.');
 for(const source of bundle.sources){
  if(!identity(source.id)||!['rcore','xtext','rcore_behavior'].includes(source.kind)||typeof source.text!=='string'||encoder.encode(source.text).length>256_000||!Array.isArray(source.dependencies)||!source.dependencies.every(identity))throw new Error('Invalid retained definition source.');
 }
 // Preserve all fields. Native compilation rejects stale/unsupported metadata.
 return compilation;
}
export function validateBundleRecovery(record,profile,allowEntry=false) {
 if(!record||typeof record!=='object'||Array.isArray(record)||Object.keys(record).some(k=>!['compilation','compilerProfile',...(allowEntry?['entryRule']:[])].includes(k))||record.compilerProfile!==profile)throw new Error('Invalid ordinary source recovery recipe.');
 if(Object.hasOwn(record,'entryRule'))validateEntryRule(record.entryRule);
 validateBundleProject(record.compilation);
 if(!record.compilation.xtext)throw new Error('Ordinary language recovery requires an Xtext source.');
 return record;
}
// Call only after Rust successfully rebuilt this original source bundle.
export function bundleSourceSnapshot(compilation) {
 const text=role=>{
  const id=compilation[role];if(!id)return '';
  const source=compilation.bundle.sources.find(s=>sourceKey(s.id)===sourceKey(id));
  if(!source)throw new Error('Rebuilt bundle is missing a selected source.');
  return source.text;
 };
 return {ecore:'',rcore:text('ecore'),xtext:text('xtext'),rcore_behavior:sourceKey(compilation.ecore)===sourceKey(compilation.rcore_behavior)?'':text('rcore_behavior')};
}
