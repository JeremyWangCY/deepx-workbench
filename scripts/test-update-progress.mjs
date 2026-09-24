import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const toolbarSource = readFileSync('src-tauri/src/lib.rs', 'utf8');
const runAction = toolbarSource.match(/  async function runAction\(command, key\) \{[\s\S]*?\n  \}/)?.[0];
if (!runAction) throw new Error('Toolbar update action not found');
let destination = '';
let invokedFromHarness = false;
const toolbarContext = {
  busy: false,
  invoke: () => { invokedFromHarness = true; },
  isLocal: false,
  location: { assign: value => { destination = value; } },
};
vm.createContext(toolbarContext);
await vm.runInContext(`${runAction}\nrunAction('update_harness', 'harness')`, toolbarContext);
if (destination !== 'http://tauri.localhost/?update=harness' || invokedFromHarness) {
  throw new Error('Harness update did not move to the persistent DeepX page');
}

const calls = [];
const fields = new Map();
const main = { innerHTML: '', querySelector: selector => {
  if (!fields.has(selector)) fields.set(selector, { textContent: '', style: {} });
  return fields.get(selector);
} };
const root = { querySelector: () => null, appendChild: () => {} };
const source = readFileSync('frontend/main.js', 'utf8').replace(/^import .*;\r?\n/gm, '');
const context = {
  document: { querySelector: () => root, createElement: () => main },
  location: { search: '?update=harness' },
  URLSearchParams,
  invoke: async command => { calls.push(command); },
  listen: async event => { calls.push(`listen:${event}`); },
};
vm.createContext(context);
vm.runInContext(source, context);
await new Promise(resolve => setImmediate(resolve));
if (calls.join(',') !== 'listen:runtime-progress,update_harness') {
  throw new Error(`Progress listener was not ready before update: ${calls.join(',')}`);
}
console.log('Harness update progress flow checks passed');
