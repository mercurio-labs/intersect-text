// All semantic operations and assertions execute in shared Rust/WASM.
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../', import.meta.url));
const build = spawnSync('cargo', ['build', '-p', 'intersect-text-acceptance', '--example', 'terminal_parents_probe', '--target', 'wasm32-unknown-unknown', '--locked'], {cwd:root,stdio:'inherit'});
assert.equal(build.status, 0, build.error?.message);
const target = resolve(root, process.env.CARGO_TARGET_DIR || 'target');
const bytes = readFileSync(resolve(target, 'wasm32-unknown-unknown/debug/examples/terminal_parents_probe.wasm'));
const {instance} = await WebAssembly.instantiate(bytes, {});
assert.equal(instance.exports.verify_terminal_parents(), 8);
console.log('8 terminal_parents groups passed in real WASM, including V52 STRING conversion and retained V51 disagreement controls.');
