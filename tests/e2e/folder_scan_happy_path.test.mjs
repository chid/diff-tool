#!/usr/bin/env node

/**
 * End-to-end test: CLI folder scan happy path.
 *
 * Runs `cargo run -p diff-cli -- dir examples/folder-basic/left --json`,
 * parses the JSON array of DirEntryDiff, and asserts the expected shape,
 * status, sizes, and sorting.
 *
 * See docs/e2e-integration-test-process.md for the specification.
 */

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
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

// ── fixture path ───────────────────────────────────────────────────────────

const rootDir = resolve(PROJECT_ROOT, 'examples/folder-basic/left');
check(existsSync(rootDir), `fixture directory exists: ${rootDir}`);

// ── run CLI (JSON mode) ───────────────────────────────────────────────────

banner('Running dt dir --json …');

const result = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'dir',
    rootDir,
    '--json',
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

check(result.status === 0, 'exit code is 0 (success)');

// ── stderr assertions ──────────────────────────────────────────────────────

banner('Stderr');
const stderr = result.stderr.toString();
check(stderr.trim() === '', 'stderr is empty (no warnings or errors)');

// ── stdout / JSON assertions ───────────────────────────────────────────────

banner('JSON parse');
let entries;
try {
  entries = JSON.parse(result.stdout.toString());
  pass('stdout is valid JSON');
} catch {
  fail('stdout is not valid JSON');
  console.error(result.stdout.toString().slice(0, 1000));
  process.exit(1);
}

// ── top-level shape ────────────────────────────────────────────────────────

banner('Top-level shape');
check(Array.isArray(entries), 'root is a JSON array');
// examples/folder-basic/left contains: README.md, src/, src/app.rs
check(entries.length === 3, `exactly 3 entries (got ${entries.length})`);

// ── entry shape assertions ─────────────────────────────────────────────────

banner('Entry shape');

const expectedFields = ['path', 'status', 'size', 'mtime', 'hash', 'children'];
for (const [i, entry] of entries.entries()) {
  for (const field of expectedFields) {
    check(
      Object.prototype.hasOwnProperty.call(entry, field),
      `entry[${i}] (\"${entry.path}\") has field \"${field}\"`,
    );
  }
}

// ── field type assertions ──────────────────────────────────────────────────

banner('Field types');

for (const [i, entry] of entries.entries()) {
  check(
    typeof entry.path === 'string' && entry.path.length > 0,
    `entry[${i}].path is a non-empty string: \"${entry.path}\"`,
  );
  check(
    typeof entry.status === 'string' && entry.status.length > 0,
    `entry[${i}].status is a non-empty string: \"${entry.status}\"`,
  );
  check(
    entry.size === null || typeof entry.size === 'number',
    `entry[${i}].size is null or number: ${entry.size}`,
  );
  check(
    entry.mtime === null || typeof entry.mtime === 'number',
    `entry[${i}].mtime is null or number: ${entry.mtime}`,
  );
  check(
    entry.hash === null || typeof entry.hash === 'string',
    `entry[${i}].hash is null: ${entry.hash}`,
  );
  check(
    Array.isArray(entry.children),
    `entry[${i}].children is an array (length ${entry.children.length})`,
  );
}

// ── status assertions ──────────────────────────────────────────────────────

banner('Status values');

const validStatuses = ['added', 'deleted', 'modified', 'unchanged', 'directory'];
for (const [i, entry] of entries.entries()) {
  check(
    validStatuses.includes(entry.status),
    `entry[${i}].status \"${entry.status}\" is valid`,
  );
}

// Directory entries: status = "directory", size = null
const dirs = entries.filter((e) => e.status === 'directory');
for (const dir of dirs) {
  check(dir.size === null, `directory \"${dir.path}\" has size=null`);
}
check(dirs.length >= 1, `at least 1 directory entry (got ${dirs.length})`);

// File entries: status = "unchanged" (single-tree scan), size > 0
const files = entries.filter((e) => e.status === 'unchanged');
for (const f of files) {
  check(
    typeof f.size === 'number' && f.size >= 0,
    `file \"${f.path}\" has non-negative size: ${f.size}`,
  );
}
check(files.length >= 1, `at least 1 file entry (got ${files.length})`);

// ── specific entry assertions ──────────────────────────────────────────────

banner('Expected entries');

// Find expected entries by path
const readme = entries.find((e) => e.path === 'README.md');
const srcDir = entries.find((e) => e.path === 'src');
const appRs = entries.find((e) => e.path === 'src/app.rs');

check(readme !== undefined, 'README.md entry exists');
if (readme) {
  check(readme.status === 'unchanged', 'README.md status is "unchanged"');
  check(typeof readme.size === 'number' && readme.size > 0, 'README.md has positive size');
  check(readme.hash === null, 'README.md hash is null');
  check(readme.children.length === 0, 'README.md has no children');
}

check(srcDir !== undefined, 'src/ directory entry exists');
if (srcDir) {
  check(srcDir.status === 'directory', 'src/ status is "directory"');
  check(srcDir.size === null, 'src/ size is null');
  check(srcDir.children.length === 0, 'src/ has no children (flat scan)');
}

check(appRs !== undefined, 'src/app.rs entry exists');
if (appRs) {
  check(appRs.status === 'unchanged', 'src/app.rs status is "unchanged"');
  check(typeof appRs.size === 'number' && appRs.size > 0, 'src/app.rs has positive size');
  check(appRs.hash === null, 'src/app.rs hash is null');
  check(appRs.children.length === 0, 'src/app.rs has no children');
}

// ── sorting assertion ──────────────────────────────────────────────────────

banner('Sorting');
const paths = entries.map((e) => e.path);
const sorted = [...paths].sort();
const isSorted = JSON.stringify(paths) === JSON.stringify(sorted);
check(isSorted, 'entries are sorted by path (ascending)');
if (!isSorted) {
  console.error(`  expected: ${sorted.join(', ')}`);
  console.error(`  got:      ${paths.join(', ')}`);
}

// ── entry count assertion ──────────────────────────────────────────────────

banner('Entry count');
// Already asserted above; keep for discoverability.
check(entries.length === 3, `entry count confirmed: ${entries.length}`);

// ── human-readable output (non-JSON) ──────────────────────────────────────

banner('Human-readable output (without --json)');

const resultHuman = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'dir',
    rootDir,
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

const humanStdout = resultHuman.stdout.toString().trim();
const humanStderr = resultHuman.stderr.toString().trim();
check(
  humanStdout === '3 entries',
  `human output matches "3 entries": "${humanStdout}"`,
);
check(
  humanStderr === '',
  `human mode stderr is empty: "${humanStderr}"`,
);

// ── JSON output is pretty-printed ──────────────────────────────────────────

banner('JSON pretty-print');
const jsonString = result.stdout.toString();
check(
  jsonString.includes('\n'),
  'JSON output is pretty-printed (contains newlines)',
);

// ── instrumentation output ─────────────────────────────────────────────────

banner('Instrumentation');

const instrument = {
  exitCode: result.status,
  jsonBytes: result.stdout.length,
  entryCount: entries.length,
  dirCount: dirs.length,
  fileCount: files.length,
  entryPaths: entries.map((e) => e.path),
  fixture: 'examples/folder-basic/left',
};

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'folder-scan-latest.json'),
  JSON.stringify(instrument, null, 2) + '\n',
);

pass(`wrote instrumentation to target/e2e/folder-scan-latest.json`);

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
