#!/usr/bin/env node

/**
 * End-to-end test: CLI text diff happy path.
 *
 * Runs `cargo run -p diff-cli -- diff examples/text-basic/left.txt
 * examples/text-basic/right.txt --json`, parses the JSON, and asserts
 * the expected shape, stats, and line-level fields.
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

// ── run CLI ────────────────────────────────────────────────────────────────

const leftPath = resolve(PROJECT_ROOT, 'examples/text-basic/left.txt');
const rightPath = resolve(PROJECT_ROOT, 'examples/text-basic/right.txt');

check(existsSync(leftPath), `fixture exists: ${leftPath}`);
check(existsSync(rightPath), `fixture exists: ${rightPath}`);

banner('Running dt diff --json …');

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

check(result.status === 1, 'exit code is 1 (differences found)');

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

// ── stats assertions ───────────────────────────────────────────────────────

banner('Stats');
check(json.stats.additions === 2, 'stats.additions === 2');
check(json.stats.deletions === 1, 'stats.deletions === 1');
check(json.stats.unchanged === 2, 'stats.unchanged === 2');

// ── files assertions ───────────────────────────────────────────────────────

banner('File entries');
check(json.files.length === 1, 'exactly 1 file entry');

const file = json.files[0];
check(typeof file.left === 'string' && file.left.length > 0, 'file.left is a non-empty string');
check(typeof file.right === 'string' && file.right.length > 0, 'file.right is a non-empty string');
check(Array.isArray(file.hunks), 'file.hunks is an array');
check(file.hunks.length >= 1, 'at least 1 hunk');

// ── hunk assertions ────────────────────────────────────────────────────────

banner('Hunks');

const hunk = file.hunks[0];
check(
  typeof hunk.old_range?.start === 'number' && typeof hunk.old_range?.len === 'number',
  'hunk.old_range has start and len',
);
check(
  typeof hunk.new_range?.start === 'number' && typeof hunk.new_range?.len === 'number',
  'hunk.new_range has start and len',
);
check(typeof hunk.foldable_context === 'boolean', 'hunk.foldable_context is boolean');
check(Array.isArray(hunk.lines), 'hunk.lines is an array');
check(hunk.lines.length >= 1, 'hunk has at least 1 line');

// ── line-level assertions ──────────────────────────────────────────────────

banner('Line entries');

const kinds = hunk.lines.map((l) => l.kind);
check(kinds.includes('context'), 'lines include context kind');
check(kinds.includes('addition'), 'lines include addition kind');
check(kinds.includes('deletion'), 'lines include deletion kind');

for (const [i, line] of hunk.lines.entries()) {
  const validKinds = ['context', 'addition', 'deletion'];
  check(
    validKinds.includes(line.kind),
    `line[${i}] kind \"${line.kind}\" is valid`,
  );

  check(
    typeof line.text === 'string' && line.text.length >= 0,
    `line[${i}] has text string`,
  );

  check(
    Array.isArray(line.intra_spans),
    `line[${i}] intra_spans is an array`,
  );

  // Context lines have both line numbers
  if (line.kind === 'context') {
    check(
      typeof line.old_no === 'number' && line.old_no > 0,
      `line[${i}] (context) has old_no`,
    );
    check(
      typeof line.new_no === 'number' && line.new_no > 0,
      `line[${i}] (context) has new_no`,
    );
  }

  // Deletion lines have old_no and no new_no
  if (line.kind === 'deletion') {
    check(
      typeof line.old_no === 'number' && line.old_no > 0,
      `line[${i}] (deletion) has old_no`,
    );
    check(
      line.new_no === null,
      `line[${i}] (deletion) new_no is null`,
    );
  }

  // Addition lines have new_no and no old_no
  if (line.kind === 'addition') {
    check(
      line.old_no === null,
      `line[${i}] (addition) old_no is null`,
    );
    check(
      typeof line.new_no === 'number' && line.new_no > 0,
      `line[${i}] (addition) has new_no`,
    );
  }
}

// ── intra_span assertions (specific to this fixture) ───────────────────────

banner('Intraline spans');

// The "welcome" → "hello" line pair should have intra_spans.
const delLine = hunk.lines.find((l) => l.kind === 'deletion');
const addLine = hunk.lines.find((l) => l.kind === 'addition');

check(delLine !== undefined, 'found a deletion line');
check(addLine !== undefined, 'found an addition line');

if (delLine) {
  check(delLine.intra_spans.length >= 1, 'deletion line has intra_spans');
  if (delLine.intra_spans.length > 0) {
    const s = delLine.intra_spans[0];
    check(typeof s.start === 'number' && s.start >= 0, 'intra_span.start is non-negative integer');
    check(typeof s.len === 'number' && s.len > 0, 'intra_span.len is positive integer');
  }
}
if (addLine) {
  check(addLine.intra_spans.length >= 1, 'addition line has intra_spans');
  if (addLine.intra_spans.length > 0) {
    const s = addLine.intra_spans[0];
    check(typeof s.start === 'number' && s.start >= 0, 'intra_span.start is non-negative integer');
    check(typeof s.len === 'number' && s.len > 0, 'intra_span.len is positive integer');
  }
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
  lineCount: hunk.lines.length,
  intraSpanCount: hunk.lines.reduce((n, l) => n + (l.intra_spans?.length ?? 0), 0),
  fixture: 'examples/text-basic',
};

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'latest.json'),
  JSON.stringify(instrument, null, 2) + '\n',
);

pass(`wrote instrumentation to target/e2e/latest.json`);

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
const total = failures;
if (total === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${total} assertion(s) failed.`);
  process.exit(1);
}
