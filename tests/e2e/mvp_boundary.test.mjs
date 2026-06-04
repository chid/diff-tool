#!/usr/bin/env node

/**
 * MVP boundary contract.
 *
 * This is intentionally static and fast. It protects the release test surface:
 * desktop/CLI text diff and single-root folder scan are MVP; Office/AI/image,
 * merge, Git, and true folder comparison stay experimental or post-MVP.
 */

import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = resolve(__dirname, '..', '..');
const E2E_OUT_DIR = resolve(PROJECT_ROOT, 'target', 'e2e');

const read = (path) => readFileSync(resolve(PROJECT_ROOT, path), 'utf8');
const json = (path) => JSON.parse(read(path));

let failures = 0;
function check(cond, msg) {
  if (cond) {
    console.log(`  PASS  ${msg}`);
  } else {
    failures++;
    console.error(`  FAIL  ${msg}`);
  }
}

console.log('\n-- MVP package script contract');
const pkg = json('package.json');
const scripts = pkg.scripts ?? {};
for (const name of [
  'test:mvp',
  'test:mvp:package',
  'test:e2e:cli-all',
  'test:e2e:progress-diff',
  'test:e2e:progress-dir',
  'test:e2e:config',
  'test:e2e:mvp-boundary',
  'test:e2e:office',
]) {
  check(typeof scripts[name] === 'string' && scripts[name].length > 0, `${name} script exists`);
}
check(scripts['test:mvp'].includes('test:e2e:cli-all'), 'test:mvp runs all MVP CLI e2e tests');
check(scripts['test:mvp'].includes('test:e2e:mvp-boundary'), 'test:mvp runs the MVP boundary contract');
check(scripts['test:mvp'].includes('perf:quick'), 'test:mvp runs the quick perf gate');
check(!scripts['test:mvp'].includes('test:e2e:office'), 'Office e2e is available but not MVP-blocking');

console.log('\n-- Desktop MVP claim contract');
const app = read('apps/desktop/src/App.tsx');
check(app.includes('Open folder to scan'), 'folder UI is labeled as scan, not compare');
check(app.includes('Folder → scan tree'), 'drop overlay describes folder scan');
check(app.includes('AI chat test — accessed via ?test=ai'), 'AI surface is test-only');
check(app.includes('Office document diff test — accessed via ?test=office'), 'Office surface is test-only');
check(!app.includes('Office Document Diff Test') || app.indexOf('Office Document Diff Test') > app.indexOf("testMode === 'office'"), 'Office UI is not a primary mode claim');
const loader = read('apps/desktop/src/useFileDiffLoader.ts');
check(!loader.includes("const DEMO_LEFT = 'README.md'"), 'Tauri loader has no hardcoded README default');
check(loader.includes("setState({ phase: 'idle' })"), 'Tauri loader idles when no file is selected');
check(loader.includes('const rustResult = await compareFiles(leftPath, rightPath, compareOpts)'), 'two-file Tauri load uses Rust DiffResult directly');
check(loader.includes('buildSingleFilePreviewDiff'), 'single-file previews use native-shaped DiffResult');

console.log('\n-- Documentation MVP boundary contract');
const plan = read('PLAN.md');
const todo = read('TODO.md');
const testing = read('docs/testing-and-performance.md');
check(plan.includes('Ship target: MVP Desktop'), 'PLAN keeps MVP desktop target');
check(plan.includes('Office comparison is parked as experimental'), 'PLAN parks Office for MVP');
check(todo.includes('Phase 4: Decision on Rust-format vs. pierre-format diff output'), 'TODO tracks renderer contract decision');
check(testing.includes('npm run test:mvp'), 'testing docs mention the MVP gate');
check(testing.includes('npm run test:mvp:package'), 'testing docs mention the packaging smoke gate');

console.log('\n-- Playwright output contract');
const playwright = read('playwright.config.ts');
check(playwright.includes("outputFolder: 'target/e2e/report'"), 'Playwright HTML report stays under target/e2e');
check(playwright.includes('PLAYWRIGHT_HEADED'), 'headed Playwright runs are opt-in');

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'mvp-boundary-latest.json'),
  JSON.stringify({
    fixture: 'mvp-boundary',
    assertionsFailed: failures,
    scriptsChecked: 8,
  }, null, 2) + '\n',
);

console.log('\n-- Summary');
if (failures === 0) {
  console.log('  All checks passed!');
  process.exit(0);
}

console.error(`  ${failures} assertion(s) failed.`);
process.exit(1);
