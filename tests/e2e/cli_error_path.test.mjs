#!/usr/bin/env node

/**
 * End-to-end test: CLI error paths.
 *
 * Runs `dt diff` against nonexistent files and binary files,
 * asserting exit code 2 (EXIT_ERROR) and error messages on stderr.
 *
 * Covers the error paths from docs/e2e-integration-test-process.md
 * scenario 4: CLI error path for missing file.
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

const validLeft = resolve(PROJECT_ROOT, 'examples/text-basic/left.txt');
const validRight = resolve(PROJECT_ROOT, 'examples/text-basic/right.txt');
const nonexistent = '/nonexistent/path/definitely_not_a_real_file.txt';
const binaryPath = '/tmp/dt-e2e-binary-test.bin';

// Prepare a binary file for testing (null byte triggers binary detection).
writeFileSync(binaryPath, Buffer.from([0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f]));
check(existsSync(validLeft), `valid left fixture exists: ${validLeft}`);
check(existsSync(validRight), `valid right fixture exists: ${validRight}`);
check(existsSync(binaryPath), `binary test file created: ${binaryPath}`);

// ── helper: run diff and assert error ──────────────────────────────────────

/**
 * Run `dt diff <left> <right> --json`, assert exit 2 with error on stderr
 * and empty stdout (JSON mode) or non-empty stderr (human mode).
 */
function assertError({ label, left, right, flags = [], expectStderrContains }) {
  banner(label);

  const result = spawnSync(
    'cargo',
    [
      'run',
      '-p', 'diff-cli',
      '--quiet',
      '--',
      'diff',
      left,
      right,
      ...flags,
    ],
    {
      cwd: PROJECT_ROOT,
      stdio: 'pipe',
      timeout: 30_000,
    },
  );

  const commandWorked = result.status !== null && result.error === undefined;
  check(commandWorked, `${label}: command spawned without OS error`);
  if (!commandWorked) {
    console.error(result.error?.message ?? 'unknown spawn error');
    return;
  }

  // Exit code must be 2 (EXIT_ERROR)
  check(result.status === 2, `${label}: exit code is 2 (EXIT_ERROR), got ${result.status}`);

  // Stderr must contain the expected error text
  const stderr = result.stderr.toString();
  check(
    stderr.includes('error:'),
    `${label}: stderr contains "error:" prefix`,
  );
  for (const substr of expectStderrContains) {
    check(
      stderr.includes(substr),
      `${label}: stderr contains "${substr}"`,
    );
  }

  // Stdout should be empty — errors happen before any output is produced.
  const stdout = result.stdout.toString().trim();
  check(stdout === '', `${label}: stdout is empty`);

  return { status: result.status, stderr };
}

// ── error cases ────────────────────────────────────────────────────────────

// 1. Missing left file (JSON mode)
assertError({
  label: 'Missing left file (--json)',
  left: nonexistent,
  right: validRight,
  flags: ['--json'],
  expectStderrContains: ['failed to read'],
});

// 2. Missing left file (human mode)
assertError({
  label: 'Missing left file (human)',
  left: nonexistent,
  right: validRight,
  flags: [],
  expectStderrContains: ['failed to read'],
});

// 3. Missing right file (JSON mode)
assertError({
  label: 'Missing right file (--json)',
  left: validLeft,
  right: nonexistent,
  flags: ['--json'],
  expectStderrContains: ['failed to read'],
});

// 4. Missing right file (human mode)
assertError({
  label: 'Missing right file (human)',
  left: validLeft,
  right: nonexistent,
  flags: [],
  expectStderrContains: ['failed to read'],
});

// 5. Both files missing — check we report the first failure
assertError({
  label: 'Both files missing',
  left: '/nonexistent/a.txt',
  right: '/nonexistent/b.txt',
  flags: ['--json'],
  expectStderrContains: ['failed to read', 'a.txt'],
});

// 6. Binary file detection (left)
assertError({
  label: 'Binary file (left)',
  left: binaryPath,
  right: validRight,
  flags: ['--json'],
  expectStderrContains: ['appears to be a binary file'],
});

// 7. Binary file detection (right)
assertError({
  label: 'Binary file (right)',
  left: validLeft,
  right: binaryPath,
  flags: ['--json'],
  expectStderrContains: ['appears to be a binary file'],
});

// ── dt dir error path ──────────────────────────────────────────────────────

banner('dt dir nonexistent path');

const resultDirNonexistent = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'dir',
    '/nonexistent/dir/path',
    '--json',
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

check(
  resultDirNonexistent.status !== null && resultDirNonexistent.error === undefined,
  'dt dir nonexistent: command spawned without OS error',
);
// CLI now returns exit 2 for nonexistent paths with an error on stderr
// (see crates/diff-cli/src/main.rs Command::Dir).
check(
  resultDirNonexistent.status === 2,
  `dt dir nonexistent: exit code is 2 (EXIT_ERROR), got ${resultDirNonexistent.status}`,
);

const dirStderr = resultDirNonexistent.stderr.toString();
check(
  dirStderr.includes('error:') && dirStderr.includes('does not exist'),
  `dt dir nonexistent: stderr reports path does not exist, got "${dirStderr.trim()}"`,
);

// Stdout should be empty — errors happen before any output.
const dirStdout = resultDirNonexistent.stdout.toString().trim();
check(dirStdout === '', `dt dir nonexistent: stdout is empty (error on stderr), got "${dirStdout.slice(0, 200)}"`);

// ── sanity: valid files still work ────────────────────────────────────────

banner('Sanity: valid files still work');

const resultValid = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'diff',
    validLeft,
    validRight,
    '--json',
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

check(resultValid.status !== null, 'valid diff: command spawned');
check(resultValid.status === 1, 'valid diff: exit code 1 (differences found)');
check(
  resultValid.stderr.toString().trim() === '',
  'valid diff: stderr is empty',
);
const validStdout = resultValid.stdout.toString().trim();
check(
  validStdout.length > 0,
  `valid diff: stdout is non-empty (${validStdout.length} bytes)`,
);

// ── instrumentation output ─────────────────────────────────────────────────

banner('Instrumentation');

const instrument = {
  fixture: 'cli error paths',
  scenarios: [
    { name: 'missing-left-json', exitCode: 2 },
    { name: 'missing-left-human', exitCode: 2 },
    { name: 'missing-right-json', exitCode: 2 },
    { name: 'missing-right-human', exitCode: 2 },
    { name: 'both-missing', exitCode: 2 },
    { name: 'binary-left', exitCode: 2 },
    { name: 'binary-right', exitCode: 2 },
    { name: 'valid-files-sanity', exitCode: 1 },
  ],
};

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'cli-error-path-latest.json'),
  JSON.stringify(instrument, null, 2) + '\n',
);

pass(`wrote instrumentation to target/e2e/cli-error-path-latest.json`);

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
