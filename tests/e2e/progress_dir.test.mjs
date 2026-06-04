#!/usr/bin/env node

/**
 * End-to-end test: CLI `--progress` flag for `dt dir` two-folder comparison.
 *
 * Runs `cargo run -p diff-cli -- dir left --right right --progress`,
 * captures stderr, and asserts that per-file progress messages are emitted
 * with `[relative/path]` prefixes. Also verifies exit code and stdout
 * are unchanged from a normal `--progress`-less run.
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

const leftRoot = resolve(PROJECT_ROOT, 'examples/folder-basic/left');
const rightRoot = resolve(PROJECT_ROOT, 'examples/folder-basic/right');

check(existsSync(leftRoot), `left root exists: ${leftRoot}`);
check(existsSync(rightRoot), `right root exists: ${rightRoot}`);

// ── run CLI with --progress ────────────────────────────────────────────────

banner('Running dt dir --right --progress …');

const result = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'dir',
    leftRoot,
    '--right', rightRoot,
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

check(result.status === 0, 'exit code is 0 (success)');

// ── stderr assertions ──────────────────────────────────────────────────────

banner('Stderr (progress messages)');

const stderr = result.stderr.toString();
const stderrLines = stderr.trim().split('\n').map((l) => l.trim()).filter(Boolean);

// At minimum, files that exist on both sides should produce progress.
// The folder fixture has some files in common (README.md, src/app.rs
// exists on both sides, src/new.rs is right-only).
check(
  stderrLines.length >= 1,
  `stderr has at least 1 progress line (${stderrLines.length} lines found)`,
);

// ── per-file progress prefix assertions ───────────────────────────────────

banner('Per-file [path] prefix');

// All progress lines should be prefixed with a [path] label.
// (Lines are trimmed, so no leading spaces.)
const labelPattern = /^\[.+\] /;
for (const [i, line] of stderrLines.entries()) {
  check(
    labelPattern.test(line),
    `progress line ${i + 1} has \"[path]\" prefix: "${line}"`,
  );
}

// Verify we see progress for at least one known shared file (README.md).
// The fixture has README.md on both sides with different content.
const readmeLines = stderrLines.filter((l) => l.includes('[README.md]'));
check(
  readmeLines.length > 0,
  `progress emitted for README.md (found ${readmeLines.length} lines)`,
);

if (readmeLines.length > 0) {
  // README.md should have reading + pipeline stages.
  check(
    readmeLines.some((l) => l.includes('reading')),
    '[README.md] progress includes reading stage',
  );
  check(
    readmeLines.some((l) => l.includes('normalizing')),
    '[README.md] progress includes normalizing stage',
  );
  check(
    readmeLines.some((l) => l.includes('computing diff')),
    '[README.md] progress includes computing diff stage',
  );
}

// ── progress stages per file ───────────────────────────────────────────────

banner('Progress stages within each file');

// Group lines by their [path] prefix.
const groups = new Map();
for (const line of stderrLines) {
  const m = line.match(/^\[(.+?)\]/);
  if (m) {
    const path = m[1];
    if (!groups.has(path)) groups.set(path, []);
    groups.get(path).push(line);
  }
}

// Each file that was actually diffed should have at least the reading stage.
for (const [path, lines] of groups) {
  check(
    lines.some((l) => l.includes('reading')),
    `[${path}] has reading stage`,
  );
  check(
    lines.some((l) => l.includes('building hunks')),
    `[${path}] has building hunks stage`,
  );
}

// ── stdout assertions ──────────────────────────────────────────────────────

banner('Stdout (dir diff output)');

const stdout = result.stdout.toString().trim();
check(stdout.length > 0, 'stdout is non-empty');

// Each line should have a status and a path.
const stdoutLines = stdout.split('\n').filter(Boolean);
check(stdoutLines.length > 0, `stdout has at least 1 line (${stdoutLines.length} lines)`);

for (const [i, line] of stdoutLines.entries()) {
  const parts = line.split(/\s+/, 2);
  check(
    ['added', 'deleted', 'modified', 'unchanged'].includes(parts[0]),
    `line ${i + 1} status "${parts[0]}" is valid`,
  );
  check(
    parts.length >= 2 && parts[1].length > 0,
    `line ${i + 1} has a path: "${line}"`,
  );
}

// ── sanity: same exit code and stdout as without --progress ────────────────

banner('Sanity: same output as without --progress');

const resultNoProgress = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'dir',
    leftRoot,
    '--right', rightRoot,
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

check(
  resultNoProgress.stderr.toString().trim() === '',
  'without --progress, stderr is empty',
);

// Stdout should be deterministic and match between the two runs.
// (Rayon may change internal ordering but Iter → sort_by_key makes it stable.)
check(
  resultNoProgress.stdout.toString() === result.stdout.toString(),
  'same stdout with and without --progress',
);

// ── --json + --progress mode ───────────────────────────────────────────────

banner('JSON mode + --progress');

const resultJson = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'dir',
    leftRoot,
    '--right', rightRoot,
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
  jsonStderr.includes('[') && jsonStderr.includes('reading'),
  '--json + --progress: stderr contains progress messages',
);

let json;
try {
  json = JSON.parse(resultJson.stdout.toString());
  pass('--json + --progress: stdout is valid JSON');
} catch {
  fail('--json + --progress: stdout is not valid JSON');
}

if (json) {
  check(Array.isArray(json), '--json + --progress: JSON root is an array');
  if (json.length > 0) {
    check(
      typeof json[0].path === 'string' && typeof json[0].status === 'string',
      '--json + --progress: entries have path and status',
    );
  }
}

// ── instrumentation output ─────────────────────────────────────────────────

banner('Instrumentation');

const instrument = {
  fixture: 'examples/folder-basic',
  leftRoot,
  rightRoot,
  progressExitCode: result.status,
  progressStderrLineCount: stderrLines.length,
  pathsWithProgress: [...groups.keys()],
  stdoutLineCount: stdoutLines.length,
  stdoutSameAsWithoutProgress:
    resultNoProgress.stdout.toString() === result.stdout.toString(),
};

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'progress-dir-latest.json'),
  JSON.stringify(instrument, null, 2) + '\n',
);

pass(`wrote instrumentation to target/e2e/progress-dir-latest.json`);

// ── single-root mode: --progress should be a no-op (no files to diff) ─────

banner('Single-root --progress (no-op)');

const resultSingle = spawnSync(
  'cargo',
  [
    'run',
    '-p', 'diff-cli',
    '--quiet',
    '--',
    'dir',
    leftRoot,
    '--progress',
  ],
  {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  },
);

check(resultSingle.status === 0, 'single-root --progress: exit code 0');
const singleStderr = resultSingle.stderr.toString().trim();
check(
  singleStderr === '',
  `single-root --progress: stderr is empty (no files to diff), got "${singleStderr}"`,
);
const singleStdout = resultSingle.stdout.toString().trim();
check(
  singleStdout === '3 entries',
  `single-root --progress: stdout is correct, got "${singleStdout}"`,
);

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
