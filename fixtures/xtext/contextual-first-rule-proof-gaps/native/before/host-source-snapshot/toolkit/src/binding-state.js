// Project draft validation only. Rust remains the authority for contracts/types.
export function validateBindings(state) {
  if (state === undefined) return;
  const bounded = value => typeof value === 'string' && new TextEncoder().encode(value).length <= 262144;
  if (state?.version === 1) throw new Error('This project contains a legacy implementation-dialog draft. Export its Rhai source before importing it into the file-based editor.');
  if (!state || state.version !== 2 || typeof state.present !== 'boolean' || !bounded(state.source) || !bounded(state.arguments)) {
    throw new Error('Invalid Rhai implementation file.');
  }
  const remainder = state.source.replaceAll('\r\n', '');
  if (remainder.includes('\r') || (state.source.includes('\r\n') && remainder.includes('\n'))) throw new Error('Mixed or standalone CR line endings in Rhai source.');
}
