// Editor presentation only. Every document still uses the native model/query
// runtime or the existing transactional Python host.
export function validateWindows(state) {
  if (state === undefined) return;
  if (!state || state.version !== 1 || !Array.isArray(state.documents) || state.documents.length < 4 || state.documents.length > 32 || typeof state.split !== 'boolean') throw new Error('Invalid editor windows.');
  const ids = new Set(), names = new Set();
  for (const doc of state.documents) {
    if (!doc || !['model', 'python', 'query'].includes(doc.type) || !/^(catalog|loans|script|query|document-[0-9]{1,12})$/.test(doc.id) || ids.has(doc.id) || !/^[a-zA-Z0-9_-][a-zA-Z0-9_.-]{0,79}$/.test(doc.name) || names.has(doc.name.toLowerCase()) || ![0, 1].includes(doc.group)) throw new Error('Invalid or duplicate editor window.');
    ids.add(doc.id); names.add(doc.name.toLowerCase());
    if (doc.type !== 'query') {
      if (typeof doc.source !== 'string' || new TextEncoder().encode(doc.source).length > (doc.type === 'python' ? 128000 : 256000)) throw new Error('Invalid or oversized window source.');
      const remainder = doc.source.replaceAll('\r\n', '');
      if (remainder.includes('\r') || (doc.source.includes('\r\n') && remainder.includes('\n'))) throw new Error('Mixed or standalone CR line endings in window source.');
      if (doc.type === 'python' && !doc.name.endsWith('.py')) throw new Error('Python windows require a .py filename.');
    } else if (!doc.query || typeof doc.query !== 'object' || Array.isArray(doc.query) || JSON.stringify(doc.query).length > 128000 || Object.entries(doc.query).some(([key, value]) => key !== 'customFilter' && typeof value !== 'string')) throw new Error('Invalid query window.');
  }
  for (const [id, type, name] of [['catalog', 'model', 'catalog.library'], ['loans', 'model', 'loans.library'], ['script', 'python', 'script.py'], ['query', 'query', 'Queries']]) {
    if (!state.documents.some(doc => doc.id === id && doc.type === type && doc.name === name)) throw new Error('Missing standard editor window.');
  }
  if (!state.split && state.documents.some(doc => doc.group !== 0)) throw new Error('Invalid merged editor groups.');
  if (!Array.isArray(state.selected) || state.selected.length !== 2 || state.selected.some((id, group) => id !== null && !state.documents.some(doc => doc.id === id && doc.group === group))) throw new Error('Invalid selected editor window.');
}

export function createEditorWindows({ host, initial, create, onActivate, extension }) {
  const documents = [...initial];
  documents.forEach(doc => doc.group = 0);
  let split = false, focus = 0, selected = ['catalog', null], mode = 'language', sequence = 0;
  host.classList.add('editor-windows');
  const toolbar = document.createElement('div'); toolbar.className = 'window-toolbar';
  toolbar.innerHTML = '<button id="new-window">＋ New window</button><span class="window-hint">Model files · Python · Queries</span><button id="split-windows" aria-pressed="true">Side-by-side</button><button id="move-window">Move to other group</button>';
  const groups = [0, 1].map(index => {
    const group = document.createElement('section'); group.className = 'editor-group';
    group.innerHTML = `<nav class="window-tabs" role="tablist" aria-label="Editor group ${index + 1}"></nav><div class="window-content"></div>`;
    for (const event of ['pointerdown', 'focusin']) group.addEventListener(event, () => { focus = index; activateContext(); });
    return group;
  });
  const body = document.createElement('div'); body.className = 'window-groups'; body.append(...groups);
  const dialog = document.createElement('dialog'); dialog.id = 'new-window-dialog';
  dialog.innerHTML = '<form><h2>New window</h2><label>Window type<select name="type"><option value="model">Model file</option><option value="python">Python script</option><option value="query">Query</option></select></label><label>Name<input name="name" required maxlength="80" autocomplete="off"></label><p class="window-error" role="status"></p><div><button type="button" data-cancel>Cancel</button><button type="submit" class="primary">Create window</button></div></form>';
  host.append(toolbar, body, dialog);
  const form = dialog.querySelector('form'), type = form.elements.type, name = form.elements.name;
  const available = doc => mode !== 'structure' || doc.type !== 'model';
  function activateContext() { const doc = documents.find(d => d.id === selected[focus]); if (doc) onActivate(doc); }
  function render() {
    body.classList.toggle('is-split', split);
    toolbar.querySelector('#split-windows').setAttribute('aria-pressed', String(split));
    toolbar.querySelector('#move-window').hidden = !split;
    for (const [index, group] of groups.entries()) {
      group.hidden = index === 1 && !split;
      const docs = documents.filter(d => (split ? d.group === index : index === 0) && available(d));
      group.hidden = !docs.length;
      if (!docs.some(d => d.id === selected[index])) selected[index] = docs[0]?.id || null;
      const nav = group.firstElementChild; nav.replaceChildren();
      for (const doc of docs) {
        const button = document.createElement('button'); button.className = 'tab'; button.textContent = doc.displayName || doc.name;
        button.id = 'window-tab-' + doc.id; button.dataset.window = doc.id;
        if (doc.type === 'model') button.dataset.model = doc.id;
        button.setAttribute('role', 'tab'); button.setAttribute('aria-selected', String(doc.id === selected[index]));
        button.classList.toggle('selected', doc.id === selected[index]); button.tabIndex = doc.id === selected[index] ? 0 : -1;
        button.setAttribute('aria-controls', 'window-' + doc.id);
        button.onclick = () => open(doc.id, index);
        nav.append(button);
        doc.panel.id = 'window-' + doc.id; doc.panel.setAttribute('role', 'tabpanel'); doc.panel.setAttribute('aria-labelledby', button.id);
        doc.panel.removeAttribute('data-usage'); doc.panel.hidden = doc.id !== selected[index];
        group.lastElementChild.append(doc.panel);
      }
    }
    body.classList.toggle('is-split', split && groups.every(group => !group.hidden));
    for (const doc of documents.filter(d => !available(d))) doc.panel.hidden = true;
    activateContext();
  }
  groups.forEach(group => group.firstElementChild.addEventListener('keydown', event => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    const tabs = [...group.querySelectorAll('[role="tab"]')], index = tabs.indexOf(event.target);
    if (index < 0) return;
    event.preventDefault(); const next = event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 : (index + (event.key === 'ArrowLeft' ? -1 : 1) + tabs.length) % tabs.length;
    tabs[next].click(); document.getElementById(tabs[next].id)?.focus();
  }));
  function open(id, group) {
    const doc = documents.find(d => d.id === id); if (!doc || !available(doc)) return;
    focus = group ?? (split ? doc.group : 0); selected[focus] = id; render();
  }
  function suggested() {
    const suffix = type.value === 'python' ? '.py' : type.value === 'query' ? '.query' : '.' + extension();
    let number = 1; while (documents.some(d => d.name.toLowerCase() === `${type.value}${number}${suffix}`.toLowerCase())) number += 1;
    name.value = `${type.value}${number}${suffix}`;
  }
  toolbar.querySelector('#new-window').onclick = () => {
    type.querySelector('[value="model"]').disabled = mode === 'structure';
    if (mode === 'structure' && type.value === 'model') type.value = 'python';
    suggested(); dialog.querySelector('.window-error').textContent = ''; dialog.showModal(); name.focus(); name.select();
  };
  type.onchange = suggested;
  dialog.querySelector('[data-cancel]').onclick = () => dialog.close();
  form.onsubmit = event => {
    event.preventDefault();
    try {
      if (!/^[a-zA-Z0-9_-][a-zA-Z0-9_.-]{0,79}$/.test(name.value)) throw new Error('Use letters, numbers, underscores, dots or hyphens in the name.');
      if (documents.length >= 32) throw new Error('This preview supports up to 32 windows per project.');
      if (documents.some(d => d.name.toLowerCase() === name.value.toLowerCase())) throw new Error('A window with this name already exists.');
      if (type.value === 'python' && !name.value.endsWith('.py')) throw new Error('Use a .py filename for a Python script.');
      if (type.value === 'model' && !name.value.endsWith('.' + extension())) throw new Error('Use .' + extension() + ' for this language.');
      let id; do { id = 'document-' + ++sequence; } while (documents.some(d => d.id === id));
      const doc = create({ id, name: name.value, type: type.value, group: focus, source: '' });
      documents.push(doc); selected[focus] = id; dialog.close(); render();
    } catch (error) { dialog.querySelector('.window-error').textContent = error.message; }
  };
  toolbar.querySelector('#split-windows').onclick = () => {
    if (split) { selected[0] = selected[focus]; documents.forEach(d => d.group = 0); focus = 0; split = false; }
    else { split = true; const other = documents.find(d => d.id !== selected[0] && available(d)); if (other) { other.group = 1; selected[1] = other.id; } }
    render();
  };
  toolbar.querySelector('#move-window').onclick = () => {
    const doc = documents.find(d => d.id === selected[focus]); if (!doc) return;
    doc.group = 1 - focus; selected[focus] = null; focus = doc.group; selected[focus] = doc.id; render();
  };
  render();
  return {
    documents, open,
    setModelLabels(labels) {
      for (const doc of documents.filter(d => d.type === 'model')) {
        doc.displayName = labels[doc.id] || doc.name;
        const path = doc.panel.querySelector('.file-path');
        if (path) (path.querySelector('#model-file') || path).textContent = doc.displayName;
        doc.editor.updateOptions({ ariaLabel: 'Model source: ' + doc.displayName });
      }
      render();
    },
    setMode(value) { mode = value; render(); },
    snapshot() { return { version: 1, split, selected: [...selected], documents: documents.map(doc => ({ id: doc.id, name: doc.name, type: doc.type, group: doc.group, ...(doc.model ? { source: doc.model.getValue() } : { query: doc.query.snapshot() }) })) }; },
    restore(state) {
      validateWindows(state);
      onActivate(initial.find(doc => doc.python));
      for (const doc of documents.splice(initial.length)) doc.dispose?.();
      const defaults = initial.map(doc => ({ ...doc, group: 0 }));
      const saved = state?.documents || defaults;
      for (const value of saved) {
        let doc = documents.find(d => d.id === value.id);
        if (!doc) { doc = create(value); documents.push(doc); }
        doc.group = value.group;
        if (doc.query && value.query && !value.query.snapshot) doc.query.restore(value.query);
      }
      split = state?.split ?? false; selected = state ? [...state.selected] : ['catalog', null]; focus = 0; render();
    },
  };
}
