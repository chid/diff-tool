#!/usr/bin/env node

/**
 * End-to-end test: CLI text diff with --ignore-whitespace-and-case.
 *
 * Runs `cargo run -p diff-cli -- diff fixtures/cases/whitespace-case-ignored/left.txt
 * fixtures/cases/whitespace-case-ignored/right.txt --json --ignore-whitespace-and-case`,
 * parses the JSON, and asserts zero differences (identical after whitespace
 * normalisation and case folding).
 *
 * The fixture has "One   Two\nTHREE\n" on the left and "one two\nthree\n" on
 * the right. With --ignore-whitespace-and-case these are treated as identical.
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

const fixtureDir = resolve(PROJECT_ROOT, 'fixtures/cases/whitespace-case-ignored');
const leftPath = resolve(fixtureDir, 'left.txt');
const rightPath = resolve(fixtureDir, 'right.txt');

check(existsSync(leftPath), `fixture exists: ${leftPath}`);
check(existsSync(rightPath), `fixture exists: ${rightPath}`);

// ── run CLI ────────────────────────────────────────────────────────────────

banner('Running dt diff --json --ignore-whitespace-and-case …');

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
    '--ignore-whitespace-and-case',
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

// With --ignore-whitespace-and-case the files are identical → exit code 0.
check(result.status === 0, 'exit code is 0 (files identical after whitespace+case ignore)');

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
check(json.stats.additions === 0, 'stats.additions === 0');
check(json.stats.deletions === 0, 'stats.deletions === 0');
check(json.stats.unchanged === 2, 'stats.unchanged === 2');

// ── files assertions ───────────────────────────────────────────────────────

banner('File entries');
check(json.files.length === 1, 'exactly 1 file entry');

const file = json.files[0];
check(typeof file.left === 'string' && file.left.length > 0, 'file.left is a non-empty string');
check(typeof file.right === 'string' && file.right.length > 0, 'file.right is a non-empty string');
check(Array.isArray(file.hunks), 'file.hunks is an array');
check(file.hunks.length === 0, 'hunks is empty (0 hunks for identical files)');

// ── sanity: whitespace-only ignore still shows case differences ────────────

banner('Sanity: --ignore-whitespace (without case) still shows differences');

const resultWsOnly = spawnSync(
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

check(
  resultWsOnly.status !== null && resultWsOnly.status === 1,
  'with --ignore-whitespace only, exit code is 1 (case differences remain)',
);

let jsonWsOnly;
try {
  jsonWsOnly = JSON.parse(resultWsOnly.stdout.toString());
  pass('with --ignore-whitespace only, stdout is valid JSON');
} catch {
  fail('with --ignore-whitespace only, stdout is not valid JSON');
}

if (jsonWsOnly) {
  check(
    jsonWsOnly.stats.additions >= 1 || jsonWsOnly.stats.deletions >= 1,
    'with --ignore-whitespace only, at least one addition or deletion exists (case differs)',
  );
  check(
    jsonWsOnly.files[0].hunks.length >= 1,
    'with --ignore-whitespace only, at least 1 hunk present',
  );
}

// ── sanity: conflict detection works ───────────────────────────────────────

banner('Sanity: --ignore-whitespace-and-case conflicts with --ignore-whitespace');

const resultConflict = spawnSync(
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
    '--ignore-whitespace-and-case',
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

check(
  resultConflict.status !== null && resultConflict.status === 2,
  'passing both --ignore-whitespace and --ignore-whitespace-and-case fails with exit code 2',
);

const conflictStderr = resultConflict.stderr.toString();
check(
  conflictStderr.includes('ignore-whitespace') || conflictStderr.includes('cannot be used'),
  `stderr indicates conflict: ${conflictStderr.slice(0, 200).trim()}`,
);

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
  hunkCount: 0,
  lineCount: 0,
  intraSpanCount: 0,
  fixture: 'fixtures/cases/whitespace-case-ignored',
  ignoreMode: 'whitespace-and-case',
};

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'whitespace-case-ignore-latest.json'),
  JSON.stringify(instrument, null, 2) + '\n',
);

pass(`wrote instrumentation to target/e2e/whitespace-case-ignore-latest.json`);

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
