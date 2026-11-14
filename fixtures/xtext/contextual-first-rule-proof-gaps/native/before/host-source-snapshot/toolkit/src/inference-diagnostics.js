// Handwritten host projection of native producer metadata; no semantic inference.
import {sourceKey} from './source-drafts.js';
const same=(a,b)=>a&&b&&sourceKey(a)===sourceKey(b);
export function projectInferenceDiagnostics(result,inputs){
  const sources=inputs?.bundle.sources||[];
  const retained=id=>sources.some(s=>same(s.id,id));
  return (result.diagnostics||[]).map((d,index)=>{
    const origin=result.diagnostic_origins?.[index];
    const base={...d,inferredDiagnostic:true,origin,locations:[]};
    if(origin){
      if(origin.kind==='source'&&retained(origin.source)){
        const span=origin.span;
        if(span&&!same(span.source,origin.source))return {...base,scope:'unavailable'};
        const location={source:origin.source,start:span?.start??d.start,end:span?.end??d.end};
        return {...base,scope:'source',sourceId:origin.source,start:location.start,end:location.end,source:origin.source.uri,locations:[location]};
      }
      if(origin.kind==='generated'){
        const locations=origin.originals||[];
        return {...base,scope:'generated',locations};
      }
      return {...base,scope:origin.kind==='context'?'context':'unavailable'};
    }
    const candidates=sources.filter(s=>s.id.uri===d.source);
    if(candidates.length===1){const sourceId=candidates[0].id;return {...base,scope:'legacy',legacySourceId:sourceId,locations:[{source:sourceId,start:d.start,end:d.end,legacy:true}]};}
    return {...base,scope:'legacy',ambiguousSource:candidates.length>1};
  });
}
export function inferenceDiagnosticLocations(d){return d.locations||[];}
export function inferenceDiagnosticLabel(d){
  if(d.ambiguousSource)return `${d.source} · multiple retained revisions`;
  if(d.scope==='legacy')return `URI-only diagnostic · ${d.source}`;
  if(d.scope==='generated')return `Generated source · ${d.origin.generated.source.uri} · ${d.locations.length} authored locations`;
  if(d.scope==='context')return `Request context · ${d.origin.source.uri}`;
  if(d.scope==='unavailable')return 'Source ownership unavailable';
  return d.sourceId?`${d.sourceId.sha256.slice(0,12)} · ${d.sourceId.uri}`:d.source;
}
export function inferenceDiagnosticRows(diagnostics){
  return diagnostics.flatMap(d=>d.scope!=='generated'?[{...d,label:inferenceDiagnosticLabel(d)}]:[
    {...d,label:inferenceDiagnosticLabel(d)},
    ...d.locations.map((location,index)=>({...d,scope:'correspondence',sourceId:location.source,source:location.source.uri,start:location.start,end:location.end,locations:[location],label:`Authored location ${index+1} · ${location.source.sha256.slice(0,12)} · ${location.source.uri} · bytes ${location.start}–${location.end}`})),
  ]);
}
