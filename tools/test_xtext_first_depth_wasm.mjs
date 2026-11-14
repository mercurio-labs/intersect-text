// Verification-only host; all semantic operations and assertions execute in Rust/WASM.
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
const root = fileURLToPath(new URL('../', import.meta.url));
const build = spawnSync('cargo', ['build', '-p', 'intersect-text-acceptance', '--example', 'first_depth_probe', '--target', 'wasm32-unknown-unknown', '--locked'], {cwd:root,stdio:'inherit'});
assert.equal(build.status, 0, build.error?.message);
const target = resolve(root, process.env.CARGO_TARGET_DIR || 'target');
const bytes = readFileSync(resolve(target, 'wasm32-unknown-unknown/debug/examples/first_depth_probe.wasm'));
const {instance} = await WebAssembly.instantiate(bytes, {});
assert.equal(instance.exports.verify_first_depth(), 1);
console.log('Deep rule FIRST analysis and rejection controls passed in real WASM.');
