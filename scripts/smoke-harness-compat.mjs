import { spawn } from 'node:child_process';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const [runtime, node = process.execPath] = process.argv.slice(2);
if (!runtime) throw new Error('Usage: node scripts/smoke-harness-compat.mjs <runtime> [node]');

const home = await mkdtemp(join(tmpdir(), 'deepx-harness-smoke-'));
const patch = join(home, 'deepx-no-open.yml');
await writeFile(patch, '- id: web-runtime\n  config:\n    openBrowser: false\n    printUrl: true\n    surfaceContext: true\n    trustedHosts: []\n');

const child = spawn(node, [join(runtime, 'node_modules/@deepseek-ai/dsh/lib/bin.js'), '--profile', 'web', '--patch', patch, '--no-open', '--port', '0'], {
  env: { ...process.env, DSH_HOME: home },
  windowsHide: true,
});
let output = '';
let errors = '';
child.stdout.on('data', chunk => { output += chunk; });
child.stderr.on('data', chunk => { errors += chunk; });

try {
  const deadline = Date.now() + 30000;
  let ready = false;
  while (Date.now() < deadline && child.exitCode === null) {
    const match = output.match(/dsh web: (http:\/\/127\.0\.0\.1:\d+\/\?token=[^\s]+)/);
    if (match) {
      const response = await fetch(match[1]).catch(() => null);
      if (response) {
        ready = true;
        break;
      }
    }
    await new Promise(resolve => setTimeout(resolve, 500));
  }
  if (!ready) throw new Error(`Harness did not become ready: ${errors.slice(0, 1000)}`);
  console.log('Harness web smoke test passed');
} finally {
  child.kill();
}
