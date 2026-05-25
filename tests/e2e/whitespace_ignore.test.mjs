#!/usr/bin/env node

/**
 * End-to-end test: CLI text diff with --ignore-whitespace.
 *
 * Runs `cargo run -p diff-cli -- diff fixtures/cases/whitespace-ignored/left.txt
 * fixtures/cases/whitespace-ignored/right.txt --json --ignore-whitespace`,
 * parses the JSON, and asserts zero differences (identical after whitespace
 * normalisation).
 *
 * The fixture has "one   two" (multiple spaces) on the left and "one two" on
 * the right. With --ignore-whitespace these are treated as identical.
 *
 * See docs/e2e-integration-test-process.md for the specification.
 */

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

// ── helpers ────────────────────────────────────────────────────────────────

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = resolve(__dirname, '..', '..');
const E2E_OUT_DIR = resolve(PROJECT_ROOT, 'target', 'e2e');

function banner(text) {
  console.log(`\n── ${text}`);
}

let failures = 0;
function fail(msg) {
  failures++;
  console.error(`  FAIL  ${msg}`);
}
function pass(msg) {
  console.log(`  PASS  ${msg}`);
}
function check(cond, msg) {
  if (cond) {
    pass(msg);
  } else {
    fail(msg);
  }
}

// ── fixture paths ──────────────────────────────────────────────────────────

const fixtureDir = resolve(PROJECT_ROOT, 'fixtures/cases/whitespace-ignored');
const leftPath = resolve(fixtureDir, 'left.txt');
const rightPath = resolve(fixtureDir, 'right.txt');

check(existsSync(leftPath), `fixture exists: ${leftPath}`);
check(existsSync(rightPath), `fixture exists: ${rightPath}`);

// ── run CLI ────────────────────────────────────────────────────────────────

banner('Running dt diff --json --ignore-whitespace …');

const result = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'diff',
    leftPath,
    rightPath,
    '--json',
    '--ignore-whitespace',
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

// ── exit code assertions ───────────────────────────────────────────────────

banner('Exit code');
const commandWorked = result.status !== null && result.error === undefined;
check(commandWorked, 'command spawned without OS error');
if (!commandWorked) {
  console.error(result.error?.message ?? 'unknown spawn error');
  process.exit(1);
}

// With --ignore-whitespace the files are identical → exit code 0.
check(result.status === 0, 'exit code is 0 (files identical after whitespace ignore)');

// ── stderr assertions ──────────────────────────────────────────────────────

banner('Stderr');
const stderr = result.stderr.toString();
check(stderr.trim() === '', 'stderr is empty (no warnings or errors)');

// ── stdout / JSON assertions ───────────────────────────────────────────────

banner('JSON parse');
let json;
try {
  json = JSON.parse(result.stdout.toString());
  pass('stdout is valid JSON');
} catch {
  fail('stdout is not valid JSON');
  console.error(result.stdout.toString().slice(0, 1000));
  process.exit(1);
}

// ── top-level shape ────────────────────────────────────────────────────────

banner('Top-level keys');
check(Array.isArray(json.files), 'root.files is an array');
check(typeof json.stats === 'object' && json.stats !== null, 'root.stats is an object');
check(Array.isArray(json.warnings), 'root.warnings is an array');
check(json.truncated === false, 'root.truncated is false');

const topKeys = Object.keys(json).sort();
const expectedKeys = ['files', 'stats', 'truncated', 'warnings'].sort();
check(
  JSON.stringify(topKeys) === JSON.stringify(expectedKeys),
  `top-level keys match expected: ${topKeys.join(', ')}`,
);

// ── stats assertions (zero differences) ────────────────────────────────────

banner('Stats (zero differences)');
check(json.stats.additions === 0, 'stats.additions === 0 (no additions with whitespace ignore)');
check(json.stats.deletions === 0, 'stats.deletions === 0 (no deletions with whitespace ignore)');
check(json.stats.unchanged === 2, 'stats.unchanged === 2 (both lines are identical)');

// ── files assertions ───────────────────────────────────────────────────────

banner('File entries');
check(json.files.length === 1, 'exactly 1 file entry');

const file = json.files[0];
check(typeof file.left === 'string' && file.left.length > 0, 'file.left is a non-empty string');
check(typeof file.right === 'string' && file.right.length > 0, 'file.right is a non-empty string');
check(Array.isArray(file.hunks), 'file.hunks is an array');
check(
  file.hunks.length === 0,
  `hunks is empty (0 hunks for identical files), got ${file.hunks.length}`,
);

// ── non-whitespace-ignored comparison still shows differences ──────────────

banner('Sanity: diff without --ignore-whitespace shows differences');

const resultNoIgnore = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'diff',
    leftPath,
    rightPath,
    '--json',
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

check(
  resultNoIgnore.status !== null && resultNoIgnore.status === 1,
  'without --ignore-whitespace, exit code is 1 (differences found)',
);

let jsonNoIgnore;
try {
  jsonNoIgnore = JSON.parse(resultNoIgnore.stdout.toString());
  pass('without --ignore-whitespace, stdout is valid JSON');
} catch {
  fail('without --ignore-whitespace, stdout is not valid JSON');
}

if (jsonNoIgnore) {
  check(
    jsonNoIgnore.stats.additions >= 1 || jsonNoIgnore.stats.deletions >= 1,
    'without --ignore-whitespace, at least one addition or deletion exists',
  );
  check(
    jsonNoIgnore.stats.additions + jsonNoIgnore.stats.deletions > 0,
    `without --ignore-whitespace, total changes > 0 (got additions=${jsonNoIgnore.stats.additions}, deletions=${jsonNoIgnore.stats.deletions})`,
  );
  check(
    jsonNoIgnore.files[0].hunks.length >= 1,
    'without --ignore-whitespace, at least 1 hunk present',
  );
}

// ── instrumentation output ─────────────────────────────────────────────────

banner('Instrumentation');

const leftContents = readFileSync(leftPath);
const rightContents = readFileSync(rightPath);

const instrument = {
  exitCode: result.status,
  jsonBytes: result.stdout.length,
  leftBytes: leftContents.length,
  rightBytes: rightContents.length,
  leftLines: leftContents.toString().split('\n').filter(Boolean).length,
  rightLines: rightContents.toString().split('\n').filter(Boolean).length,
  fileCount: json.files.length,
  hunkCount: file.hunks.length,
  lineCount: 0,
  intraSpanCount: 0,
  fixture: 'fixtures/cases/whitespace-ignored',
  ignoreMode: 'whitespace',
};

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'whitespace-ignore-latest.json'),
  JSON.stringify(instrument, null, 2) + '\n',
);

pass(`wrote instrumentation to target/e2e/whitespace-ignore-latest.json`);

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
