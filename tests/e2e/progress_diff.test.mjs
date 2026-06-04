#!/usr/bin/env node

/**
 * End-to-end test: CLI `--progress` flag for `dt diff`.
 *
 * Runs `cargo run -p diff-cli -- diff left right --progress`,
 * captures stderr, and asserts all 6 progress stages fire in the
 * correct order. Also verifies exit code and stdout are unchanged
 * from a normal --progress-less run.
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

// ── fixture paths ──────────────────────────────────────────────────────────

const leftPath = resolve(PROJECT_ROOT, 'examples/text-basic/left.txt');
const rightPath = resolve(PROJECT_ROOT, 'examples/text-basic/right.txt');

check(existsSync(leftPath), `fixture exists: ${leftPath}`);
check(existsSync(rightPath), `fixture exists: ${rightPath}`);

// ── run CLI with --progress ────────────────────────────────────────────────

banner('Running dt diff --progress …');

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
    '--progress',
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

banner('Stderr (progress messages)');

const stderr = result.stderr.toString();
const stderrLines = stderr.trim().split('\n').map((l) => l.trim()).filter(Boolean);

check(stderrLines.length >= 6, `at least 6 progress lines (got ${stderrLines.length})`);

// Verify all 6 stages appear in order: reading (×2), normalizing,
// computing-diff, computing-intraline, building-hunks.
const expectedStages = [
  'reading',
  'reading',
  'normalizing',
  'computing diff',
  'computing intraline spans',
  'building hunks',
];

for (let i = 0; i < expectedStages.length; i++) {
  const expected = expectedStages[i];
  const actual = stderrLines[i] || '';
  check(
    actual.includes(expected),
    `progress line ${i + 1} contains "${expected}" — got "${actual}"`,
  );
}

// Reading lines should include the file path and KiB.
// (Don't mandate left/right order — it's an implementation detail.)
const firstTwoReading = stderrLines.slice(0, 2).join(' ');
check(
  firstTwoReading.includes('left.txt'),
  'first two reading lines mention left.txt',
);
check(
  firstTwoReading.includes('right.txt'),
  'first two reading lines mention right.txt',
);
check(
  stderrLines[0].includes('KiB'),
  'reading line includes KiB suffix',
);

// The normalizing/compuring/etc. lines should end with "..."
for (let i = 2; i < 6; i++) {
  check(
    stderrLines[i].endsWith('...'),
    `progress line ${i + 1} ends with \"...\": "${stderrLines[i]}"`,
  );
}

// ── stdout assertions ──────────────────────────────────────────────────────

banner('Stdout (diff output, unchanged by --progress)');

const stdout = result.stdout.toString().trim();
check(stdout.length > 0, 'stdout is non-empty');
check(
  stdout.includes('additions') && stdout.includes('deletions'),
  `stdout contains diff summary: "${stdout}"`,
);

// ── sanity: --progress does not change exit code vs non-progress run ───────

banner('Sanity: same exit code as without --progress');

const resultNoProgress = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'diff',
    leftPath,
    rightPath,
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

check(
  resultNoProgress.status === result.status,
  `same exit code with and without --progress: ${resultNoProgress.status} === ${result.status}`,
);

const stdoutNoProgress = resultNoProgress.stdout.toString().trim();
check(
  stdoutNoProgress === stdout,
  'same stdout with and without --progress',
);
check(
  resultNoProgress.stderr.toString().trim() === '',
  'without --progress, stderr is empty',
);

// ── also test with --json + --progress (progress on stderr, JSON on stdout)

banner('JSON mode + --progress');

const resultJson = spawnSync(
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
    '--progress',
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

const jsonStderr = resultJson.stderr.toString();
check(
  jsonStderr.includes('reading') && jsonStderr.includes('building hunks'),
  '--json + --progress: stderr contains progress messages',
);
check(
  !jsonStderr.includes('error:'),
  '--json + --progress: stderr contains no error messages',
);

let json;
try {
  json = JSON.parse(resultJson.stdout.toString());
  pass('--json + --progress: stdout is valid JSON');
} catch {
  fail('--json + --progress: stdout is not valid JSON');
}
check(
  json && json.stats && json.files,
  '--json + --progress: JSON has expected shape (files + stats)',
);

// ── instrumentation output ─────────────────────────────────────────────────

banner('Instrumentation');

const instrument = {
  fixture: 'examples/text-basic',
  progressExitCode: result.status,
  progressStderrLineCount: stderrLines.length,
  progressStages: expectedStages,
  stdoutSameAsWithoutProgress: stdoutNoProgress === stdout,
};

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'progress-diff-latest.json'),
  JSON.stringify(instrument, null, 2) + '\n',
);

pass(`wrote instrumentation to target/e2e/progress-diff-latest.json`);

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
