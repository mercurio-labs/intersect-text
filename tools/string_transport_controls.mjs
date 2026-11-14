// Test fixture codec only. Native source compilation remains execution authority.
import {readFileSync} from 'node:fs';
import {join} from 'node:path';
export function expandReferences(value,catalog){
  if(Array.isArray(value))return value.map(v=>expandReferences(v,catalog));
  if(value&&typeof value==='object'){
    if(Object.hasOwn(value,'$ref')){
      if(Object.keys(value).length!==1||typeof value.$ref!=='string'||!Object.hasOwn(catalog,value.$ref))throw new Error('Unknown or augmented fixture reference');
      return structuredClone(catalog[value.$ref]);
    }
    return Object.fromEntries(Object.entries(value).map(([k,v])=>[k,expandReferences(v,catalog)]));
  }
  return value;
}
export function loadControls(root){
  const folder=join(root,'fixtures','rcore-string-transport');
  const plan=JSON.parse(readFileSync(join(folder,'plan.json'),'utf8'));
  const packed=JSON.parse(readFileSync(join(folder,'public-controls.json'),'utf8'));
  if(plan.version!==1||packed.version!==1)throw new Error('Unsupported String control fixture version');
  const refs={};
  for(const c of plan.cases){
    const f=JSON.parse(readFileSync(join(folder,c.id+'.json'),'utf8'));
    Object.assign(refs,{[c.id+'/request']:f.request,[c.id+'/bundle']:f.request.bundle,[c.id+'/arguments']:f.request.arguments,[c.id+'/contexts']:f.request.reference_contexts,[c.id+'/selection']:f.request.selection,[c.id+'/program']:c.complete.program,[c.id+'/complete']:c.complete,[c.id+'/evaluation']:c.complete.evaluation});
  }
  return {plan,packed,groups:packed.groups.map(g=>({id:g.id,requests:expandReferences(g.requests,refs),expected:expandReferences(g.expected,refs)}))};
}
