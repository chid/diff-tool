#!/usr/bin/env node

/**
 * End-to-end test: --context CLI flag changes hunk splitting behavior.
 *
 * Verifies that the context value in DiffRequest controls how far-apart
 * changes are grouped into hunks. Uses a 12-line temp fixture with
 * changes at lines 2 and 10 (gap of 7 unchanged lines between them).
 *
 * Default context=3 → threshold 6 → gap 7 > 6 → 2 hunks.
 * --context 10    → threshold 20 → gap 7 ≤ 20 → 1 hunk.
 * --context 0     → tight hunks, no context lines around isolated changes.
 */

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync, unlinkSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';

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

// ── setup temp fixture (12 lines, changes at 2 and 10) ────────────────────

const leftPath = resolve(tmpdir(), 'dt-e2e-context-left.txt');
const rightPath = resolve(tmpdir(), 'dt-e2e-context-right.txt');

// Remove stale files from a previous aborted run.
try { unlinkSync(leftPath); } catch { /* ok */ }
try { unlinkSync(rightPath); } catch { /* ok */ }

// No trailing newline so `split_lines_lossless` produces exactly 12 input lines.
const left = `line 1 unchanged
line 2 changed here
line 3 unchanged
line 4 unchanged
line 5 unchanged
line 6 unchanged
line 7 unchanged
line 8 unchanged
line 9 unchanged
line 10 also changed
line 11 unchanged
line 12 unchanged`;

const right = `line 1 unchanged
line 2 MODIFIED!
line 3 unchanged
line 4 unchanged
line 5 unchanged
line 6 unchanged
line 7 unchanged
line 8 unchanged
line 9 unchanged
line 10 ALSO MODIFIED!
line 11 unchanged
line 12 unchanged`;

writeFileSync(leftPath, left);
writeFileSync(rightPath, right);

check(existsSync(leftPath), `temp fixture created: ${leftPath}`);
check(existsSync(rightPath), `temp fixture created: ${rightPath}`);

// ── helper: run dt diff with --context and return parsed JSON ──────────────

function runDiff(contextValue) {
  const args = [
    'run', '-p', 'diff-cli', '--quiet', '--',
    'diff', leftPath, rightPath,
    '--json',
  ];
  if (contextValue !== undefined) {
    args.push('--context', String(contextValue));
  }

  const result = spawnSync('cargo', args, {
    cwd: PROJECT_ROOT,
    stdio: 'pipe',
    timeout: 30_000,
  });

  return {
    exitCode: result.status,
    stderr: result.stderr.toString(),
    json: result.stdout.length > 0 ? JSON.parse(result.stdout.toString()) : null,
  };
}

// ── Test 1: --context 3 → 2 hunks ────────────────────────────────────────

banner('--context 3: changes at lines 2 and 10 produce 2 hunks');

const result3 = runDiff(3);

check(result3.exitCode === 1, 'exit code is 1 (differences found)');
check(result3.json !== null, 'stdout is valid JSON');
check(result3.stderr.trim() === '', 'stderr is empty');

if (result3.json) {
  check(result3.json.stats.additions === 2, 'stats.additions === 2');
  check(result3.json.stats.deletions === 2, 'stats.deletions === 2');
  check(result3.json.files[0].hunks.length === 2, 'produces 2 hunks with default context=3');

  // First hunk should cover lines ~1-6 (includes context before/after change at line 2)
  const h1 = result3.json.files[0].hunks[0];
  check(h1.foldable_context === true, 'first hunk has foldable_context (hidden lines between hunks)');
  check(h1.old_range.start === 1, 'first hunk old_range starts at 1');

  // Second hunk should cover lines ~7-12
  const h2 = result3.json.files[0].hunks[1];
  check(h2.foldable_context === true, 'second hunk has foldable_context');
  check(h2.old_range.start >= 7, 'second hunk old_range starts at or after line 7');
}

// ── Test 2: --context 10 → 1 merged hunk ──────────────────────────────────

banner('--context 10: far-apart changes merge into 1 hunk');

const result10 = runDiff(10);

check(result10.exitCode === 1, 'exit code is 1 (differences found)');
check(result10.json !== null, 'stdout is valid JSON');
check(result10.stderr.trim() === '', 'stderr is empty');

if (result10.json) {
  check(result10.json.stats.additions === 2, 'stats.additions === 2');
  check(result10.json.stats.deletions === 2, 'stats.deletions === 2');
  check(result10.json.files[0].hunks.length === 1, 'produces 1 merged hunk with --context 10');
  check(
    result10.json.files[0].hunks[0].foldable_context === false,
    'single hunk has foldable_context=false (nothing hidden)',
  );

  // 12 input lines → 14 diff lines: 10 context + 4 change lines (2 del+add pairs)
  const totalLines = result10.json.files[0].hunks[0].lines.length;
  check(totalLines === 14, `merged hunk has 14 diff lines (10 context + 4 changes, got ${totalLines})`);

  // Both changes should be found in the merged hunk
  const texts = result10.json.files[0].hunks[0].lines.map((l) => l.text);
  check(texts.includes('line 2 changed here'), 'merged hunk includes deletion "line 2 changed here"');
  check(texts.includes('line 2 MODIFIED!'), 'merged hunk includes addition "line 2 MODIFIED!"');
  check(texts.includes('line 10 also changed'), 'merged hunk includes deletion "line 10 also changed"');
  check(texts.includes('line 10 ALSO MODIFIED!'), 'merged hunk includes addition "line 10 ALSO MODIFIED!"');
}

// ── Test 3: --context 0 → tight hunks, no context lines ────────────────────

banner('--context 0: tight hunks with no context lines');

const result0 = runDiff(0);

check(result0.exitCode === 1, 'exit code is 1 (differences found)');
check(result0.json !== null, 'stdout is valid JSON');

if (result0.json) {
  // With context=0, two isolated changes should produce 2 hunks
  const hunks = result0.json.files[0].hunks;
  check(hunks.length === 2, 'produces 2 hunks with --context 0 (each change isolated)');

  // Each hunk should contain only change lines (no context lines around them)
  for (const [i, hunk] of hunks.entries()) {
    // Each change is a del+add pair = 2 lines per hunk
    check(hunk.lines.length === 2, `hunk[${i}] has exactly 2 lines (del+add pair, got ${hunk.lines.length})`);
    check(
      hunk.lines.every((l) => l.kind !== 'context'),
      `hunk[${i}] has no context lines`,
    );
    // With context=0 there are 7 hidden lines between hunks, so foldable_context is true.
    check(
      hunk.foldable_context === true,
      `hunk[${i}] has foldable_context=true (${i === 0 ? 7 : 0} hidden lines between hunks)`,
    );
  }

  // First hunk: change at line 2
  const h0 = hunks[0];
  check(h0.lines[0].kind === 'deletion', 'hunk[0] line 0 is deletion');
  check(h0.lines[0].text === 'line 2 changed here', 'hunk[0] deletion text is "line 2 changed here"');
  check(h0.lines[1].kind === 'addition', 'hunk[0] line 1 is addition');
  check(h0.lines[1].text === 'line 2 MODIFIED!', 'hunk[0] addition text is "line 2 MODIFIED!"');

  // Second hunk: change at line 10
  const h1 = hunks[1];
  check(h1.lines[0].kind === 'deletion', 'hunk[1] line 0 is deletion');
  check(h1.lines[0].text === 'line 10 also changed', 'hunk[1] deletion text is "line 10 also changed"');
  check(h1.lines[1].kind === 'addition', 'hunk[1] line 1 is addition');
  check(h1.lines[1].text === 'line 10 ALSO MODIFIED!', 'hunk[1] addition text is "line 10 ALSO MODIFIED!"');
}

// ── instrumentation ────────────────────────────────────────────────────────

banner('Instrumentation');

const instrument = {
  defaultContext3: {
    hunks: result3.json?.files[0].hunks.length,
    totalLines: result3.json?.files[0].hunks.reduce((n, h) => n + h.lines.length, 0),
  },
  context10: {
    hunks: result10.json?.files[0].hunks.length,
    totalLines: result10.json?.files[0].hunks.reduce((n, h) => n + h.lines.length, 0),
  },
  context0: {
    hunks: result0.json?.files[0].hunks.length,
    totalLines: result0.json?.files[0].hunks.reduce((n, h) => n + h.lines.length, 0),
  },
};

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'context-flag-latest.json'),
  JSON.stringify(instrument, null, 2) + '\n',
);
pass('wrote instrumentation to target/e2e/context-flag-latest.json');

// ── cleanup ────────────────────────────────────────────────────────────────

try { unlinkSync(leftPath); } catch { /* ok */ }
try { unlinkSync(rightPath); } catch { /* ok */ }

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
