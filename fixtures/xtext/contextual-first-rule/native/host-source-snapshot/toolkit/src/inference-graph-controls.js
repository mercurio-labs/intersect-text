// Handwritten graph editor. Edits source envelopes; it never interprets a DSL.
import {sourceKey} from './source-drafts.js';
export function createInferenceGraphControls({host,read,ensure,edit,move,selectSource,history}){
  const button=document.createElement('button');button.type='button';button.className='text-button';button.textContent='Manage sources';host.append(button);
  const dialog=document.createElement('dialog');dialog.className='inference-graph-dialog';dialog.setAttribute('aria-label','Inference source graph');
  dialog.innerHTML=`<header><h2>Inference sources</h2><button type="button" data-close aria-label="Close inference sources">Close</button></header>
    <p>Original source files and explicit relationships. Compile to activate graph changes.</p>
    <label>Entry grammar<select aria-label="Inference entry grammar" data-entry></select></label>
    <div class="inference-graph-columns"><fieldset><legend>Add or import source</legend>
      <label>Source URI<input aria-label="New inference source URI" data-uri spellcheck="false"></label>
      <label>Kind<select aria-label="New inference source kind" data-kind><option value="xtext">Xtext grammar</option><option value="rcore">Canonical Rcore</option><option value="ecore">Ecore (migration required)</option><option value="rcore_behavior">Rcore behavior (requires provider selection)</option></select></label>
      <label>New source text<textarea aria-label="New inference source text" data-text rows="5" spellcheck="false"></textarea></label>
      <div class="inference-graph-actions"><button type="button" data-add>Add source</button><button type="button" data-import>Import source file</button></div>
      <input type="file" aria-label="Import inference source file" data-file hidden>
    </fieldset><fieldset><legend>Source relationships</legend>
      <label>Selected source<select aria-label="Managed inference source" data-managed></select></label>
      <code aria-label="Selected inference source identity" data-source-identity></code>
      <label>Dependencies<select aria-label="Inference source dependencies" data-dependencies multiple size="5"></select></label>
      <div class="inference-graph-actions"><button type="button" data-apply-dependencies>Apply dependencies</button><button type="button" data-remove>Remove selected source</button></div>
    </fieldset></div>
    <fieldset><legend>Namespace bindings</legend><div data-bindings></div>
      <div class="inference-namespace-row"><label>New namespace<input aria-label="New inference namespace" data-namespace spellcheck="false"></label><label>Source<select aria-label="New inference namespace source" data-namespace-source></select></label><button type="button" data-add-namespace>Add namespace binding</button></div>
    </fieldset>
    <footer><div class="inference-graph-actions"><button type="button" data-undo>Undo graph change</button><button type="button" data-redo>Redo graph change</button></div><p role="status" data-status></p></footer>`;
  document.body.append(dialog);
  const $=key=>dialog.querySelector(`[data-${key}]`),id=node=>node.selectedOptions[0]?.dataset.identity?JSON.parse(node.selectedOptions[0].dataset.identity):null;
  let token=0,busy=false,pendingImport=null;
  const message=(text,error=false)=>{$('status').textContent=text;$('status').classList.toggle('graph-error',error);};
  function label(source,sources){const collision=sources.some(s=>s!==source&&s.id.uri===source.id.uri&&s.id.sha256.slice(0,12)===source.id.sha256.slice(0,12));return `${source.kind} · ${collision?source.id.sha256:source.id.sha256.slice(0,12)} · ${source.id.uri}`;}
  function options(node,sources,selected){node.replaceChildren();for(const source of sources){const option=document.createElement('option');option.value=sourceKey(source.id);option.dataset.identity=JSON.stringify(source.id);option.textContent=label(source,sources);option.title=JSON.stringify(source.id);option.selected=selected===option.value;node.append(option);}}
  function disabled(value){busy=value;button.disabled=value;const unavailable=!read();for(const node of dialog.querySelectorAll('button,input,select,textarea'))if(node!==$('close'))node.disabled=value||unavailable;$('undo').disabled=value||!history.canUndo;$('redo').disabled=value||!history.canRedo;}
  function render(){const state=read();if(!state)return;const sources=state.record.inputs.bundle.sources,selected=sourceKey(state.selectedSource||state.record.inputs.grammar);
    options($('entry'),sources.filter(s=>s.kind==='xtext'),sourceKey(state.record.inputs.grammar));options($('managed'),sources,selected);options($('namespace-source'),sources,selected);
    const source=sources.find(s=>sourceKey(s.id)===selected)||sources[0];$('source-identity').textContent=`${source.id.uri}\nSHA-256: ${source.id.sha256}`;options($('dependencies'),sources,'');
    for(const dependency of source.dependencies){let option=[...$('dependencies').options].find(o=>o.value===sourceKey(dependency));if(!option){option=document.createElement('option');option.value=sourceKey(dependency);option.dataset.identity=JSON.stringify(dependency);option.textContent=`Missing · ${dependency.uri} @ ${dependency.sha256}`;$('dependencies').append(option);}option.selected=true;}
    $('bindings').replaceChildren();for(const [index,binding]of state.record.inputs.bundle.namespaces.entries()){
      const row=document.createElement('div');row.className='inference-namespace-row';row.innerHTML=`<label>Namespace<input aria-label="Inference namespace ${index+1}"></label><label>Source<select aria-label="Inference namespace source ${index+1}"></select></label><button type="button">Apply binding</button><button type="button">Remove binding</button>`;
      const input=row.querySelector('input'),select=row.querySelector('select'),[apply,remove]=row.querySelectorAll('button');input.value=typeof binding?.namespace==='string'?binding.namespace:'';
      const key=binding?.source&&typeof binding.source.uri==='string'&&typeof binding.source.sha256==='string'?sourceKey(binding.source):'';options(select,sources,key);
      if(!sources.some(s=>sourceKey(s.id)===key)){const option=document.createElement('option');option.value=key;option.textContent='Missing or malformed source (retained)';option.selected=true;select.prepend(option);}
      apply.onclick=()=>run(guard=>edit({type:'namespaceEdit',index,namespace:input.value,source:id(select)},guard));remove.onclick=()=>run(guard=>edit({type:'namespaceRemove',index},guard));$('bindings').append(row);
    }
    disabled(busy);
  }
  async function run(action){if(busy)return;const own=++token;const guard=()=>dialog.open&&token===own;disabled(true);message('Updating source graph…');try{await action(guard);if(token!==own)return;render();message('Graph draft updated. Compile to activate it.');}catch(error){if(token===own)message(error.message,true);}finally{if(token===own)disabled(false);}}
  button.onclick=async()=>{if(busy)return;const own=++token;dialog.showModal();disabled(true);message('Loading source graph…');try{await ensure(()=>dialog.open&&token===own);if(token!==own)return;render();message('Select an entry independently of the source editor. Graph undo preserves text in surviving source identities.');}catch(error){if(token===own)message(error.message,true);}finally{if(token===own)disabled(false);}};
  $('close').onclick=()=>dialog.close();dialog.addEventListener('close',()=>{token++;pendingImport=null;disabled(false);});
  $('entry').onchange=()=>{const source=id($('entry'));void run(guard=>edit({type:'entry',source},guard));};
  $('managed').onchange=()=>{selectSource(id($('managed')));render();};
  $('add').onclick=()=>{const action={type:'add',uri:$('uri').value,kind:$('kind').value,text:$('text').value};void run(guard=>edit(action,guard));};
  $('apply-dependencies').onclick=()=>{const action={type:'dependencies',source:id($('managed')),dependencies:[...$('dependencies').selectedOptions].map(o=>JSON.parse(o.dataset.identity))};void run(guard=>edit(action,guard));};
  $('remove').onclick=()=>{const action={type:'remove',source:id($('managed'))};void run(guard=>edit(action,guard));};
  $('add-namespace').onclick=()=>{const action={type:'namespaceAdd',namespace:$('namespace').value,source:id($('namespace-source'))};void run(guard=>edit(action,guard));};
  $('undo').onclick=()=>{void run(guard=>move('undo',guard));};$('redo').onclick=()=>{void run(guard=>move('redo',guard));};
  const stamp=()=>JSON.stringify([read(),$('uri').value,$('kind').value]);
  $('import').onclick=()=>{pendingImport={stamp:stamp(),token};$('file').click();};
  $('file').onchange=event=>{const file=event.target.files[0];event.target.value='';if(!file)return;const before=pendingImport||{stamp:stamp(),token};pendingImport=null;const launchToken=token,action={type:'add',uri:$('uri').value,kind:$('kind').value};void run(async guard=>{if(before.token!==launchToken)throw new Error('The source graph was replaced while selecting the file. Import it again.');if(file.size>256000)throw new Error('Source file exceeds the 256 KB limit.');const bytes=await file.arrayBuffer();if(before.stamp!==stamp()||!guard())throw new Error('Sources changed while opening the file. Import it again.');try{action.text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes);}catch{throw new Error('Source file is not valid UTF-8.');}await edit(action,guard);});};
  return {refresh:render,close(){token++;if(dialog.open)dialog.close();pendingImport=null;disabled(false);}};
}
