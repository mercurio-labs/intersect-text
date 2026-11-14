import {validateEntryRule} from './entry-selection.js';
// Handwritten browser project I/O. Source definitions are authoritative; layout
// is separate presentation state, and model semantics remain in Rust.
import { validateWindows } from './editor-windows.js';
import { validateBindings } from './binding-state.js';
import {sourceEol, sourceText} from './source-drafts.js';
import {validateInferenceProject, inferenceStarter} from './inference-project.js';
import {validateBundleProject,validateBundleRecovery} from './bundle-project.js';
import {validateProviderDraft,validateInferenceRecovery,validateDurableInferenceRecovery} from './inference-provider-state.js';

const FORMAT = 'intersect.toolkit.project.v2';
// Older readers must reject source recovery they cannot preserve.
const RECOVERY_FORMAT = 'intersect.toolkit.project.v3';
const ENTRY_FORMAT = 'intersect.toolkit.project.v4';
const PROVIDER_FORMAT = 'intersect.toolkit.project.v5';
const INFERRED_RECOVERY_FORMAT = 'intersect.toolkit.project.v6';
const LIMIT = 2_000_000;
const encoder = new TextEncoder();
export const emptyRcore = `rcore 1;
package \`model\` version "1";
metamodel "urn:example:model" prefix model {
  class Element {}
}
`;
export const emptyEcore = `<?xml version="1.0" encoding="UTF-8"?>
<ecore:EPackage xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xmlns:ecore="http://www.eclipse.org/emf/2002/Ecore"
    name="model" nsURI="urn:example:model" nsPrefix="model">
  <eClassifiers xsi:type="ecore:EClass" name="Element"/>
</ecore:EPackage>
`;

function download(name, source, type) {
  const url = URL.createObjectURL(new Blob([source], { type }));
  const anchor = document.createElement('a'); anchor.href = url; anchor.download = name; anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function validateProject(value) {
  if (!value || ![FORMAT,RECOVERY_FORMAT,ENTRY_FORMAT,PROVIDER_FORMAT,INFERRED_RECOVERY_FORMAT].includes(value.format) || !['language', 'structure', 'inferred'].includes(value.mode)) throw new Error('Unsupported Toolkit project format or definition mode.');
  const providerProject=value.format===PROVIDER_FORMAT,durableInference=value.format===INFERRED_RECOVERY_FORMAT;
  if(providerProject||durableInference){
    if(durableInference&&value.definitionBundle!==undefined)throw new Error('Conflicting definition source recipes.');
    if(value.mode!=='inferred'||value.definitionInference===undefined||value.definitionBundle!==undefined||value.definitionRecovery!==undefined)throw new Error(providerProject?'Provider projects require separate original inferred source recipes.':'Inferred recovery requires separate original source recipes.');
    const allowed=['format','mode','profile','compilerProfile','structureSource','entryRule','definitionInference','definitionProviderDraft','definitionInferenceRecovery','definitions','models','python','layout','windows','bindings'];
    if(Object.keys(value).some(key=>!allowed.includes(key)))throw new Error('Unsupported provider project field.');
    if(providerProject||Object.hasOwn(value,'definitionProviderDraft'))validateProviderDraft(value.definitionProviderDraft);
    if(durableInference)validateDurableInferenceRecovery(value.definitionInferenceRecovery);
    else if(Object.hasOwn(value,'definitionInferenceRecovery'))validateInferenceRecovery(value.definitionInferenceRecovery);
  }else if(Object.hasOwn(value,'definitionProviderDraft')||Object.hasOwn(value,'definitionInferenceRecovery'))throw new Error('Inferred provider drafts and recovery require the v5 or v6 project format.');
  if((value.format===ENTRY_FORMAT||providerProject||durableInference)!==Object.hasOwn(value,'entryRule'))throw new Error('Parser entry selection requires the v4, v5 or v6 project format.');
  if(value.format===ENTRY_FORMAT||providerProject||durableInference){validateEntryRule(value.entryRule);if(value.mode==='structure'&&value.entryRule!==null)throw new Error('Structure projects have no parser entry.');}
  if((value.format===RECOVERY_FORMAT&&value.definitionRecovery===undefined)||(value.format===FORMAT&&value.definitionRecovery!==undefined))throw new Error('Ordinary source recovery requires the v3 or v4 project format.');
  if (value.structureSource !== 'rcore') throw new Error('Unsupported structure source.');
  if (encoder.encode(JSON.stringify(value)).length>LIMIT) throw new Error('Project exceeds the 2 MB limit.');
  if (value.definitionInference !== undefined) {
    if (value.mode!=='inferred'||value.definitionBundle!==undefined) throw new Error('Conflicting definition source recipes.');
    validateInferenceProject(value.definitionInference);
  }
  if(value.mode==='inferred'&&(value.definitionBundle!==undefined||value.compilerProfile!=='intersect.xtext.native.v56'||value.profile!=='default')) throw new Error('Inferred projects require their fixed compiler profile and original inference inputs.');
  if (!['default','library-implementations','rcore','xtext-kerml','xtext-dispatch','xtext-fragments','xtext-punctuation','xtext-datatypes','xtext-actions','xtext-terminals','xtext-lexical','xtext-prediction','xtext-inheritance','xtext-overrides','xtext-showcase','xtext-inference','xtext-current','xtext-predicates','xtext-enums','xtext-converters','xtext-recursive','xtext-double','xtext-arguments','xtext-library-advanced','xtext-expressions-advanced','xtext-library-composed','xtext-expressions-composed','xtext-projected','xtext-nary','xtext-filtered','xtext-discovery'].includes(value.profile)) throw new Error('Unsupported language profile.');
  if (!['intersect.workbench.v1','intersect.xtext.native.v1','intersect.xtext.native.v2','intersect.xtext.native.v3','intersect.xtext.native.v4','intersect.xtext.native.v5','intersect.xtext.native.v6','intersect.xtext.native.v7','intersect.xtext.native.v8','intersect.xtext.native.v9','intersect.xtext.native.v10','intersect.xtext.native.v11','intersect.xtext.native.v27','intersect.xtext.native.v42','intersect.xtext.native.v52','intersect.xtext.native.v53','intersect.xtext.native.v54','intersect.xtext.native.v55','intersect.xtext.native.v56','intersect.xtext.native.v57','intersect.xtext.native.v58','intersect.xtext.native.v59', 'intersect.xtext.native.v60','intersect.xtext.native.v61'].includes(value.compilerProfile)) throw new Error('Unsupported Xtext compiler profile.');
  for (const [group, keys] of [['definitions', ['ecore', 'rcore', 'xtext', 'rcore_behavior']], ['models', ['catalog', 'loans']]]) {
    for (const key of keys) {
      if (typeof value[group]?.[key] !== 'string' || encoder.encode(value[group][key]).length > 256_000) throw new Error(`Invalid or oversized ${key} source (256 KB limit).`);
      sourceEol(value[group][key]);
    }
  }
  if (value.definitionBundle !== undefined) validateBundleProject(value.definitionBundle);
  if (value.definitionRecovery !== undefined) {
    if(value.mode!=='language'||value.definitionBundle===undefined||value.definitionInference!==undefined)throw new Error('Conflicting ordinary source recovery recipe.');
    validateBundleRecovery(value.definitionRecovery,value.compilerProfile,value.format===ENTRY_FORMAT);
  }
  if (typeof value.python !== 'string' || encoder.encode(value.python).length > 128_000) throw new Error('Invalid or oversized Python script (128 KB limit).');
  sourceEol(value.python);
  validateWindows(value.windows);
  validateBindings(value.bindings);
  const layout = value.layout;
  if (!layout || layout.version !== 1 || !layout.positions || Array.isArray(layout.positions) || typeof layout.positions !== 'object') throw new Error('Unsupported diagram layout.');
  if (Object.keys(layout.positions).length > 2048) throw new Error('Diagram layout has too many positions.');
  for (const [name, position] of Object.entries(layout.positions)) {
    if (name.length > 1024 || !Number.isFinite(position?.x) || !Number.isFinite(position?.y) || position.x < 0 || position.y < 0 || position.x > 100_000 || position.y > 100_000) throw new Error('Invalid diagram position.');
  }
  if (!Number.isFinite(layout.zoom) || layout.zoom < 0.5 || layout.zoom > 1.5) throw new Error('Invalid diagram zoom.');
  return value;
}

export function createProjectControls({ beforeReplace = () => {}, loaded = () => {}, getProviderDraft = () => null, setProviderDraft = () => {}, getInferenceRecovery = () => null, setInferenceRecovery = () => {}, getEntryRule = () => null, setEntryRule = () => {}, getBindings = () => undefined, restoreBindings = () => {}, getBundle = () => null, setBundle = () => {}, getInference = () => null, setInference = () => {}, getRecovery = () => null, setRecovery = () => {}, getGeneratedRcore = () => '', host, definitionModels, modelModels, python, getMode, setMode, structure, request, changed, getWindows = () => undefined, restoreWindows = () => {}, cancelScripts = () => python.cancel(), sample, kerml, created, getCompilerProfile = () => 'intersect.workbench.v1', setCompilerProfile = () => {}, getProfile = () => 'default', setProfile = () => {}, getStructureSource = () => 'rcore', setStructureSource = () => {} }) {
  host.innerHTML = `<div class="toolkit-file-actions"><button type="button" id="new-empty-project">New Blank Project</button><button type="button" id="new-sample-project">New Library Project</button><button type="button" id="new-kerml-project" title="Evolving KerML specialization demo; bounded Rcore structure, Xtext grammar and behavior">New KerML Project</button><button type="button" id="new-inferred-project">New inferred language</button><button type="button" id="open-project">Open project</button><button type="button" id="save-project">Save project</button><button type="button" id="open-rcore">Open Rcore</button><button type="button" id="save-rcore">Save Rcore</button></div><span id="project-status" role="status">Sources, Python, and diagram layout travel together in a project.</span><input type="file" id="project-file" accept=".json,.intersect" hidden><input type="file" id="rcore-file" accept=".rcore" hidden>`;
  const $ = id => host.querySelector(`#${id}`);
  const statusNode = $('project-status');
  const models = { ...definitionModels, ...modelModels, python: python.model };
  // Ecore is the read-only projection of authored Rcore, not a source edit.
  // Background diagram refresh must not cancel an otherwise unchanged file load.
  const stamp = () => JSON.stringify([getMode(), ...Object.entries(models).filter(([name])=>name!=='ecore'||getStructureSource()!=='rcore').map(([,model]) => model.getVersionId()), structure.getLayout(), getWindows(), getBindings(), getBundle(), getInference(), getRecovery(), getEntryRule(), getProviderDraft(), getInferenceRecovery()]);
  let pendingStamp;
  const status = (message, error = false) => { statusNode.textContent = message; statusNode.classList.toggle('project-error', error); };
  const snapshot = () => {
    const providerDraft=getProviderDraft()??null,inferenceRecovery=getInferenceRecovery()??null,providerProject=providerDraft!==null,durableInference=inferenceRecovery!==null&&(providerDraft===null||!Object.hasOwn(inferenceRecovery,'inference_context'));
    return {
    format: durableInference?INFERRED_RECOVERY_FORMAT:providerProject?PROVIDER_FORMAT:getEntryRule()!==null||getRecovery()?.entryRule!=null?ENTRY_FORMAT:getRecovery()?RECOVERY_FORMAT:FORMAT, mode: getMode(), profile: getProfile(), compilerProfile: getCompilerProfile(), structureSource: getStructureSource(),
    ...(providerProject||durableInference||getEntryRule()!==null||getRecovery()?.entryRule!=null?{entryRule:getEntryRule()}:{}),
    ...(getBundle() ? {definitionBundle:structuredClone(getBundle())} : {}),
    ...(getInference() ? {definitionInference:structuredClone(getInference())} : {}),
    ...(getRecovery() ? {definitionRecovery:structuredClone(getRecovery())} : {}),
    ...(providerDraft!==null?{definitionProviderDraft:structuredClone(providerDraft)}:{}),
    ...(inferenceRecovery!==null?{definitionInferenceRecovery:structuredClone(inferenceRecovery)}:{}),
    definitions: Object.fromEntries(Object.entries(definitionModels).map(([key, model]) => [key, key==='ecore'&&getMode()==='inferred'?'':sourceText(model)])),
    models: Object.fromEntries(Object.entries(modelModels).filter(([key]) => ['catalog', 'loans'].includes(key)).map(([key, model]) => [key, model.getValue()])),
    python: python.model.getValue(), layout: structure.getLayout(), windows: getWindows(), bindings: getBindings(),
    };
  };
  const replace = (model, text) => {
    const eol = sourceEol(text);
    model.pushStackElement(); model.pushEOL(eol); model.pushEditOperations([], [{ range: model.getFullModelRange(), text }], () => null); model.pushStackElement();
  };
  function load(value) {
    validateProject(value);
    beforeReplace(value,{reason:'load'});
    // Validation finishes before any editor is changed. Loading preserves draft
    // sources, including definitions that do not currently compile.
    cancelScripts();
    setProfile(value.profile);
    setCompilerProfile(value.compilerProfile);
    setMode(value.mode);
    setStructureSource('rcore');
    for (const [key, model] of Object.entries(definitionModels)) replace(model, value.definitions[key]);
    setStructureSource('rcore');
    restoreWindows(value.windows);
    restoreBindings(value.bindings);
    replace(python.model, value.python);
    for (const key of ['catalog', 'loans']) replace(modelModels[key], value.models[key]);
    structure.setLayout(value.layout);
    changed();
    setBundle(value.definitionBundle ? structuredClone(value.definitionBundle) : null);
    setInference(value.definitionInference ? structuredClone(value.definitionInference) : null);
    setRecovery(value.definitionRecovery ? structuredClone(value.definitionRecovery) : null);
    setProviderDraft(value.definitionProviderDraft ? structuredClone(value.definitionProviderDraft) : null);
    setInferenceRecovery(value.definitionInferenceRecovery ? structuredClone(value.definitionInferenceRecovery) : null);
    setEntryRule(value.entryRule??null);
    status(value.mode==='inferred'?'Project opened. Compile to rebuild its inferred language from original sources.':'Project opened. Compile to activate its Rcore definitions; test instances are recreated by Python.');
    return loaded(value,{reason:'load'});
  }
  $('save-project').onclick = () => {
    try { const source = JSON.stringify(validateProject(snapshot()), null, 2) + '\n'; if (encoder.encode(source).length > LIMIT) throw new Error('Project exceeds the 2 MB limit.'); download('model.intersect.json', source, 'application/json'); status('Saved definition drafts, model text, Python, and diagram layout. Instance overlays are exported separately.'); }
    catch (error) { status(error.message, true); }
  };
  $('save-rcore').onclick = () => { if(getMode()==='inferred'){download('inferred.generated.rcore',getGeneratedRcore(),'text/plain');status('Saved the generated Rcore preview. Save project retains its original inference sources.');}else{download('model.rcore',sourceText(definitionModels.rcore),'text/plain');status('Saved the exact Rcore source. Diagram positions are saved with the project.');} };
  for (const kind of ['project', 'rcore']) {
    $(`open-${kind}`).onclick = () => { pendingStamp = stamp(); $(`${kind}-file`).click(); };
    $(`${kind}-file`).onchange = async event => {
      const file = event.target.files[0]; event.target.value = ''; if (!file) return;
      const before = pendingStamp ?? stamp();
      try {
        if (file.size > (kind === 'project' ? LIMIT : 256_000)) throw new Error('Selected file exceeds the supported size limit.');
        const source = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(await file.arrayBuffer());
        if (before !== stamp()) throw new Error('The project changed while opening the file. Open it again to replace the current draft.');
        if (kind === 'project') await load(JSON.parse(source));
        else {
          sourceEol(source);
          const lowered = await request('intersect/lowerRcoreStructure', {uri:'memory:structure.rcore',text:source,generated_uri:'memory:ecore'});
          if (!lowered.ok) throw new Error((lowered.diagnostics || []).map(d=>d.message).join(' '));
          if (before !== stamp()) throw new Error('Definitions changed during import. Open the Rcore file again.');
          beforeReplace({format:FORMAT,mode:'structure'},{reason:'openRcore'});
          cancelScripts(); setProviderDraft(null);setInferenceRecovery(null);setEntryRule(null); setProfile('default'); setStructureSource('rcore'); setMode('structure'); replace(definitionModels.ecore, lowered.lowering.generated.text); replace(definitionModels.rcore_behavior, ''); replace(definitionModels.xtext, ''); replace(definitionModels.rcore, source);
          structure.setLayout({ version: 1, positions: {}, zoom: 1 }); changed();
          status('Rcore opened with its source preserved. Compile to create and test instances.');
          await loaded(snapshot(),{reason:'openRcore'});
        }
      } catch (error) { status(`Could not open file: ${error.message}`, true); }
    };
  }
  let creating = false;
  async function create(kind) {
    if (creating) return;
    creating = true; for(const id of ['new-empty-project','new-sample-project','new-kerml-project','new-inferred-project'])$(id).disabled=true;
    try {
      beforeReplace({format:FORMAT,mode:kind==='empty'?'structure':kind==='inferred'?'inferred':'language'},{reason:'create'});
      await cancelScripts();
      setProviderDraft(null);setInferenceRecovery(null);
      restoreBindings();
      setEntryRule(null);
      restoreWindows(); setProfile(kind === 'kerml' ? 'rcore' : 'default'); setStructureSource('rcore'); setMode(kind === 'empty' ? 'structure' : kind==='inferred'?'inferred':'language');
      const template = kind === 'sample' ? sample : kind === 'kerml' ? kerml : kind==='inferred'?{definitions:{ecore:'',rcore:'',xtext:inferenceStarter,rcore_behavior:''},models:{catalog:'alpha',loans:'alpha'},python:''}:null;
      for (const [key, model] of Object.entries(definitionModels)) replace(model, template ? (template.definitions[key] ?? '') : key === 'rcore' ? emptyRcore : '');
      setStructureSource('rcore');
      for (const [key, model] of Object.entries(modelModels)) replace(model, template ? template.models[key] : '');
      replace(python.model, template ? template.python : '');
      structure.setLayout({ version: 1, positions: {}, zoom: 1 }); changed();
      await created(kind);
      await loaded(snapshot(),{reason:'create'});
      status(kind==='inferred'?'New inferred language. Edit its Xtext source; the shared compiler generates Rcore and runs both model documents.':kind === 'sample' ? 'New sample project. The Library example is ready.' : kind === 'kerml' ? 'New KerML project. The evolving specialization demo is ready; run reachability in Test.' : 'New empty project. Edit Element or add classes in the Rcore source, then compile.');
    } catch (error) { status('Could not create project: ' + error.message, true); }
    finally { creating = false; for(const id of ['new-empty-project','new-sample-project','new-kerml-project','new-inferred-project'])$(id).disabled=false; }
  }
  $('new-inferred-project').onclick = () => create('inferred');
  $('new-empty-project').onclick = () => create('empty');
  $('new-sample-project').onclick = () => create('sample');
  $('new-kerml-project').onclick = () => create('kerml');
  return { snapshot, load };
}
