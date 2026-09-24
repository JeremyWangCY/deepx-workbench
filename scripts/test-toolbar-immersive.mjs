import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import vm from 'node:vm';

const source = process.argv[2] === 'baseline'
  ? execFileSync('git', ['show', 'origin/master:src-tauri/src/lib.rs'], { encoding: 'utf8' })
  : readFileSync('src-tauri/src/lib.rs', 'utf8');
const functionSource = source.match(/  function updateFullscreenLayout\(\) \{[\s\S]*?\n  \}/)?.[0];
if (!functionSource) throw new Error('Fullscreen detection is missing');
if (!source.includes('html.deepx-immersive .deepx-toolbar{visibility:hidden!important')) {
  throw new Error('Fullscreen toolbar style is missing');
}

const classes = new Set();
const toolbar = { contains: element => element === toolbar };
const overlay = { getBoundingClientRect: () => ({ top: 10, width: 1180, height: 780 }) };
const smallDialog = { getBoundingClientRect: () => ({ top: 10, width: 400, height: 300 }) };
let elements = [toolbar, overlay];
const context = {
  isHarness: true,
  toolbar,
  window: { innerWidth: 1200, innerHeight: 800 },
  document: {
    documentElement: { classList: { toggle: (name, enabled) => enabled ? classes.add(name) : classes.delete(name) } },
    fullscreenElement: null,
    elementsFromPoint: () => elements,
  },
  getComputedStyle: element => ({ position: element === overlay || element === smallDialog ? 'fixed' : 'static' }),
};
vm.createContext(context);
vm.runInContext(`${functionSource}\nupdateFullscreenLayout();`, context);
if (!classes.has('deepx-immersive')) throw new Error('Large fixed overlay did not hide the toolbar');
elements = [toolbar, smallDialog];
vm.runInContext('updateFullscreenLayout();', context);
if (classes.has('deepx-immersive')) throw new Error('Small dialog hid the toolbar');
elements = [toolbar];
context.document.fullscreenElement = overlay;
vm.runInContext('updateFullscreenLayout();', context);
if (!classes.has('deepx-immersive')) throw new Error('Browser fullscreen did not hide the toolbar');
console.log('Toolbar immersive layout checks passed');
