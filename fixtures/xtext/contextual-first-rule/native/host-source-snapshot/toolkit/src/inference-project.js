import {inferenceDiagnosticLocations} from './inference-diagnostics.js';
import {createInferenceProviderState} from './inference-provider-state.js';
// Handwritten editor/project adapter. No inference or model semantics in JS.
import {sourceEol, sourceKey, sourceText, sourceIdentity, withSourceEdits} from './source-drafts.js';
export {validateInferenceProject} from './inference-graph.js';
import {validateInferenceProject,inferenceGraphState,mutateInferenceGraph,createInferenceGraphHistory} from './inference-graph.js';
import {createInferenceGraphControls} from './inference-graph-controls.js';
import examples from '../../fixtures/xtext/inference-fragment-runtime/cases.json' with {type:'json'};
export const inferenceStarter=examples.find(c=>c.id==='typed_basic').sources[0].source;
const encoder=new TextEncoder();
let nextEditor=0;
export function createInferenceProject({monaco,primary,editor,host,generatedHost,changed,onGenerated}){
  host.innerHTML='<label>Inference source <select aria-label="Inference source file"></select></label><span class="small-label">Original sources · selected file is editable</span>';
  generatedHost.innerHTML='<label>Generated Rcore preview <select aria-label="Generated Rcore file"></select></label><span class="small-label">Read-only · rebuilt from original sources</span>';
  const picker=host.querySelector('select'),generatedPicker=generatedHost.querySelector('select');
  const generatedModel=monaco.editor.createModel('','rcore',monaco.Uri.parse(`file:///inference/generated-${++nextEditor}.rcore`));
  let inputs=null,rows=[],active=false,generatedSources=[],generatedInputs=null,generatedContext=null,preview=null,generation=0;
  const history=createInferenceGraphHistory(),providers=createInferenceProviderState();
  const select=index=>{picker.value=String(index);editor.setModel(rows[index]?.model||primary);};
  picker.onchange=()=>select(Number(picker.value));
  generatedPicker.onchange=()=>{generatedModel.setValue(generatedSources[Number(generatedPicker.value)]?.text||'');onGenerated();};
  function clearGenerated(){generatedSources=[];generatedInputs=null;generatedContext=null;preview=null;generatedPicker.replaceChildren();generatedModel.setValue('');}
  function disposeRows(){for(const row of rows)if(row.model!==primary)row.model.dispose();rows=[];picker.replaceChildren();editor.setModel(primary);}
  function snapshot(){return !inputs?null:{inputs:structuredClone(inputs),drafts:rows.filter(r=>r.model!==primary&&sourceText(r.model)!==r.source.text).map(r=>({source:structuredClone(r.source.id),text:sourceText(r.model)}))};}
  function capture(){return !inputs?null:{record:snapshot(),entryText:sourceText(primary),selectedSource:rows[Number(picker.value)]?.source.id||inputs.grammar};}
  const stamp=()=>JSON.stringify([generation,capture(),primary.getVersionId()]);
  function fill(state,replacePrimary){
    validateInferenceProject(state.record);const previous=rows,previousEntry=inputs?.grammar;generation++;inputs=structuredClone(state.record.inputs);picker.replaceChildren();const retained=new Set([primary]);
    const drafts=new Map(state.record.drafts.map(d=>[sourceKey(d.source),d.text]));
    if(replacePrimary&&(!previousEntry||sourceKey(previousEntry)!==sourceKey(inputs.grammar)||sourceText(primary)!==state.entryText)){primary.setValue(state.entryText);primary.setEOL(sourceEol(state.entryText));}
    rows=inputs.bundle.sources.map((source,index)=>{
      const entry=sourceKey(source.id)===sourceKey(inputs.grammar);
      const language=source.kind==='xtext'?'xtext':source.kind==='ecore'?'plaintext':'rcore';
      const existing=!entry&&previous.find(r=>r.model!==primary&&sourceKey(r.source.id)===sourceKey(source.id));
      const text=drafts.get(sourceKey(source.id))??source.text;
      const model=entry?primary:existing?.model||monaco.editor.createModel(text,language,monaco.Uri.parse(`file:///inference/source-${++nextEditor}.${source.kind}`));
      if(!entry&&!existing)model.onDidChangeContent(changed);
      if(!entry&&existing&&sourceText(model)!==text){model.setValue(text);model.setEOL(sourceEol(text));}
      retained.add(model);
      const option=document.createElement('option');option.value=String(index);option.textContent=`${entry?'Entry · ':''}${source.id.sha256.slice(0,12)} · ${source.id.uri}`;option.title=JSON.stringify(source.id);picker.append(option);
      return {source,model};
    });
    const selected=rows.findIndex(r=>sourceKey(r.source.id)===sourceKey(state.selectedSource||inputs.grammar));select(selected<0?rows.findIndex(r=>r.model===primary):selected);
    for(const row of previous)if(!retained.has(row.model))row.model.dispose();
  }
  function clear(){generation++;controls.close();history.clear();disposeRows();inputs=null;providers.clear();clearGenerated();}
  function restore(record){const text=sourceText(primary);clear();if(record)fill(inferenceGraphState(record,text),false);controls.refresh();}
  async function ensure(guard=()=>true){
    if(inputs)return;const before=stamp(),text=sourceText(primary),id=await sourceIdentity('memory:language.xtext',text);
    if(before!==stamp()||!guard())throw new Error('Sources changed while preparing the graph. Open it again.');
    fill(inferenceGraphState({inputs:{version:1,bundle:{version:1,sources:[{id,kind:'xtext',text,dependencies:[]}],namespaces:[]},grammar:id},drafts:[]},text),false);
  }
  async function edit(action,guard){
    const before=capture(),version=stamp(),next=await mutateInferenceGraph(before,action);
    if(version!==stamp()||!guard())throw new Error('Sources changed during the graph edit. Retry without replacing newer drafts.');
    if(JSON.stringify(before)===JSON.stringify(next))return;
    history.record(before);fill(next,true);changed();controls.refresh();
  }
  function move(direction,guard){if(!guard())throw new Error('Source graph editor was closed.');const next=history.move(direction,capture());fill(next,true);changed();controls.refresh();}
  const controls=createInferenceGraphControls({host,read:capture,ensure,edit,move,history,selectSource:id=>{const index=rows.findIndex(r=>sourceKey(r.source.id)===sourceKey(id));if(index>=0)select(index);}});
  return {
    generatedModel,clear,restore,clearGenerated,
    providerSnapshot:providers.providerSnapshot,providerDraft:providers.providerDraft,
    restoreProviders(value){providers.restoreProviders(value);generation++;},
    setProviderDraft(selection){if(providers.providerDraft()===selection)return;providers.setProviderDraft(selection);generation++;changed();},
    clearProviders(){if(providers.providerSnapshot()===null)return;providers.restoreProviders(null);generation++;changed();},
    recovery:providers.recovery,restoreRecovery:providers.restoreRecovery,captureSuccessfulRecovery:providers.captureSuccessfulRecovery,
    selectEntry(){select(rows.findIndex(r=>r.model===primary));},
    setActive(value){active=value;generation++;if(!active)controls.close();host.hidden=!active;generatedHost.hidden=!active;editor.updateOptions({ariaLabel:active?'Inference source':'Xtext source'});if(!active)editor.setModel(primary);else select(Number(picker.value)||0);},
    snapshot,
    async prepare(text,{providerMode=false}={}){
      if(!inputs){
        const before=stamp(),id=await sourceIdentity('memory:language.xtext',text);
        const initial={version:1,bundle:{version:1,sources:[{id,kind:'xtext',text,dependencies:[]}],namespaces:[]},grammar:id};
        // A compiling snapshot may finish after a source/project change. It can
        // still compile, but cannot replace that newer editor graph.
        if(before!==stamp())return initial;
        fill(inferenceGraphState({inputs:initial,drafts:[]},sourceText(primary)),false);
      }
      if(providerMode&&providers.providerSnapshot()===null)throw new Error('Explicit provider project mode is required before preparing behavior sources.');
      if(!providerMode&&rows.some(r=>r.source.kind==='rcore_behavior'&&sourceText(r.model).trim()))throw new Error('Separate behavior sources are not supported in inferred mode. Their drafts are retained.');
      const envelope=inputs,version=generation;
      const edits=new Map(rows.map(r=>[sourceKey(r.source.id),r.model===primary?text:sourceText(r.model)]));
      const next=await withSourceEdits(envelope,edits);
      if(inputs===envelope&&generation===version){
        if(JSON.stringify(next)!==JSON.stringify(envelope))history.clear();
        inputs=next;rows.forEach((r,i)=>{r.source=next.bundle.sources[i];});
        for(const [index,option]of [...picker.options].entries()){const source=rows[index].source;option.textContent=`${sourceKey(source.id)===sourceKey(inputs.grammar)?'Entry · ':''}${source.id.sha256.slice(0,12)} · ${source.id.uri}`;option.title=JSON.stringify(source.id);}
        controls.refresh();
      }
      return structuredClone(next);
    },
    showGenerated(report,originalInputs,inferenceContext=null){
      clearGenerated();generatedInputs=structuredClone(originalInputs);generatedContext=inferenceContext===null?null:structuredClone(inferenceContext);generatedSources=report?.sources||[];
      for(const [index,source]of generatedSources.entries()){const option=document.createElement('option');option.value=String(index);option.textContent=source.id.uri;generatedPicker.append(option);}
      const entry=Math.max(0,generatedSources.findIndex(s=>sourceKey(s.id)===sourceKey(report.entry)));generatedPicker.value=String(entry);generatedModel.setValue(generatedSources[entry]?.text||'');onGenerated();
    },
    previewRequest(){return generatedInputs&&generatedSources.length?{inference:structuredClone(generatedInputs),generated_source:generatedSources[Number(generatedPicker.value)]?.id,...(generatedContext===null?{}:{inference_context:structuredClone(generatedContext)})}:null;},
    cachePreview(value){preview=value;},
    previewDocument(source){if(!preview||preview.lowering.generated.text!==source)throw new Error('Generated preview is stale. Compile the original inference sources.');return preview.document;},
    hasPendingEdits(snapshot){return JSON.stringify(this.snapshot())!==JSON.stringify(snapshot);},
    diagnostics(diagnostics){for(const row of rows){monaco.editor.setModelMarkers(row.model,'compiler',diagnostics.flatMap(d=>inferenceDiagnosticLocations(d).map(location=>({...location,message:d.message}))).filter(location=>sourceKey(location.source)===sourceKey(row.source.id)).map(location=>{const pos=n=>row.model.getPositionAt(new TextDecoder().decode(encoder.encode(sourceText(row.model)).slice(0,n)).length);const a=pos(location.start),b=pos(location.end);return {severity:monaco.MarkerSeverity.Error,message:location.message,startLineNumber:a.lineNumber,startColumn:a.column,endLineNumber:b.lineNumber,endColumn:b.lineNumber===a.lineNumber?Math.max(a.column+1,b.column):b.column};}));}},
    focusDiagnostic(d){const selected=d.sourceId||d.legacySourceId;if(!selected)return false;const index=rows.findIndex(r=>sourceKey(r.source.id)===sourceKey(selected));if(index<0)return false;select(index);const model=rows[index].model;const p=model.getPositionAt(new TextDecoder().decode(encoder.encode(sourceText(model)).slice(0,d.start)).length);editor.setPosition(p);editor.revealPositionInCenter(p);editor.focus();return true;},
  };
}
