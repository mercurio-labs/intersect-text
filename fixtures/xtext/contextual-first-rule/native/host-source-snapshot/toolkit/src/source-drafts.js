// Host source ownership/identity transport only. Rust validates language meaning.
export const sourceKey = id => JSON.stringify([id.uri, id.sha256]);
export const sourceText = model => model.getValue(undefined, true);
export function sourceEol(source) {
  const crlf = source.includes('\r\n');
  const remainder = source.replaceAll('\r\n', '');
  if (remainder.includes('\r') || (crlf && remainder.includes('\n'))) throw new Error('Mixed or standalone CR line endings cannot be preserved by this editor. Normalize the file explicitly before opening it.');
  return crlf ? 1 : 0;
}
export async function sourceIdentity(uri, text) {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
  return {uri, sha256:[...new Uint8Array(digest)].map(byte=>byte.toString(16).padStart(2,'0')).join('')};
}
export async function withSourceEdits(inputs, edits) {
  const copy=structuredClone(inputs), replacements=new Map();
  for (const source of copy.bundle.sources) {
    const text=edits.get(sourceKey(source.id));
    // An unchanged invalid digest is rejected by Rust, never silently repaired.
    if (text === undefined || text === source.text) continue;
    const previous=sourceKey(source.id);source.text=text;
    const fresh=await sourceIdentity(source.id.uri,text);
    // Preserve unsupported identity fields for native rejection, independently
    // on each owner/reference. Editing text never repairs unknown metadata.
    source.id={...source.id,...fresh};replacements.set(previous,fresh);
  }
  const replace=value=>{
    if(Array.isArray(value))return value.map(replace);
    if(!value||typeof value!=='object')return value;
    if(typeof value.uri==='string'&&typeof value.sha256==='string'){const fresh=replacements.get(sourceKey(value));return fresh?{...value,...fresh}:value;}
    return Object.fromEntries(Object.entries(value).map(([key,item])=>[key,replace(item)]));
  };
  return replace(copy);
}
