#!/usr/bin/env node

/**
 * End-to-end test: CLI `dt office diff` subcommand.
 *
 * Tests the Office document diff pipeline (PPTX and XLSX) with both
 * human-readable and JSON output formats.
 *
 * See docs/e2e-integration-test-process.md for the specification.
 */

import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
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

const env = {
  ...process.env,
  CARGO_HOME: '/tmp/cargo-home',
};

function runOffice(args) {
  return spawnSync(
    'cargo',
    [
      'run',
      '-p', 'diff-cli',
      '--quiet',
      '--',
      'office',
      ...args,
    ],
    {
      cwd: PROJECT_ROOT,
      env,
      stdio: 'pipe',
      timeout: 60_000,
    },
  );
}

// ── fixture paths ──────────────────────────────────────────────────────────

const PPTX_LEFT  = resolve(PROJECT_ROOT, 'fixtures/cases/office-pptx-basic/left.pptx');
const PPTX_RIGHT = resolve(PROJECT_ROOT, 'fixtures/cases/office-pptx-basic/right.pptx');
const XLSX_LEFT  = resolve(PROJECT_ROOT, 'fixtures/cases/office-xlsx-basic/left.xlsx');
const XLSX_RIGHT = resolve(PROJECT_ROOT, 'fixtures/cases/office-xlsx-basic/right.xlsx');

// ── Test 1: pptx json diff ─────────────────────────────────────────────────

banner('Test 1: dt office diff pptx --json');

const r1 = runOffice(['diff', PPTX_LEFT, PPTX_RIGHT, '--json']);
check(r1.status === 1, 'exit code 1 (differences found)');
check(r1.stderr.toString().trim() === '', 'stderr is empty');

let pptxResult;
try {
  pptxResult = JSON.parse(r1.stdout.toString());
  pass('stdout is valid JSON');
} catch (e) {
  fail(`stdout is not valid JSON: ${e.message}`);
  pptxResult = null;
}

if (pptxResult) {
  check(pptxResult.format === 'power-point', `format is power-point, got: ${pptxResult.format}`);
  check(pptxResult.summary !== undefined, 'has summary');
  check(pptxResult.summary.sections_total >= 1, `sections_total >= 1, got: ${pptxResult.summary.sections_total}`);
  check(pptxResult.summary.total_atomic_changes >= 1, `total_atomic_changes >= 1, got: ${pptxResult.summary.total_atomic_changes}`);
  check(Array.isArray(pptxResult.sections), 'sections is an array');
  check(pptxResult.sections.length > 0, 'sections is non-empty');
  check(Array.isArray(pptxResult.warnings), 'warnings is an array');
  check(Array.isArray(pptxResult.unsupported_features), 'unsupported_features is an array');

  // Verify section structure
  const s0 = pptxResult.sections[0];
  check(s0.id !== undefined, 'section has id');
  check(typeof s0.id === 'string', 'section id is a string');
  check(s0.kind !== undefined, 'section has kind');
  check(s0.status !== undefined, 'section has status');
  check(['added', 'deleted', 'modified', 'unchanged', 'reordered'].includes(s0.status),
    `status is valid: ${s0.status}`);
  check(Array.isArray(s0.changes), 'section has changes array');

  // Verify atomic change structure (if any)
  if (s0.changes.length > 0) {
    const c0 = s0.changes[0];
    check(c0.path !== undefined, 'change has path');
    check(c0.kind !== undefined, 'change has kind');
    check([
      'text-changed', 'text-added', 'text-deleted',
      'structure-changed', 'formatting-changed',
      'image-changed', 'formula-changed',
      'sheet-renamed', 'slide-reordered',
    ].includes(c0.kind), `change kind is valid: ${c0.kind}`);
  }
}

// ── Test 2: pptx human-readable diff ───────────────────────────────────────

banner('Test 2: dt office diff pptx (human-readable)');

const r2 = runOffice(['diff', PPTX_LEFT, PPTX_RIGHT]);
check(r2.status === 1, 'exit code 1 (differences found)');
check(r2.stderr.toString().trim() === '', 'stderr is empty');

const stdout2 = r2.stdout.toString().trim();
check(stdout2.includes('PowerPoint document'), 'mentions PowerPoint document');
check(stdout2.includes('sections'), 'mentions section count');
check(stdout2.includes('Text changed'), 'mentions Text changed');
check(stdout2.includes('Slide 1'), 'mentions Slide 1');

// ── Test 3: xlsx json diff ─────────────────────────────────────────────────

banner('Test 3: dt office diff xlsx --json');

const r3 = runOffice(['diff', XLSX_LEFT, XLSX_RIGHT, '--json']);
check(r3.status === 1, 'exit code 1 (differences found)');
check(r3.stderr.toString().trim() === '', 'stderr is empty');

let xlsxResult;
try {
  xlsxResult = JSON.parse(r3.stdout.toString());
  pass('stdout is valid JSON');
} catch (e) {
  fail(`stdout is not valid JSON: ${e.message}`);
  xlsxResult = null;
}

if (xlsxResult) {
  check(xlsxResult.format === 'excel', `format is excel, got: ${xlsxResult.format}`);
  check(xlsxResult.summary.sections_total >= 1, `sections_total >= 1, got: ${xlsxResult.summary.sections_total}`);
  check(Array.isArray(xlsxResult.sections), 'sections is an array');
  check(xlsxResult.sections.length > 0, 'sections is non-empty');
  check(xlsxResult.sections[0].kind === 'excel-sheet',
    `section kind is excel-sheet, got: ${xlsxResult.sections[0].kind}`);
}

// ── Test 4: xlsx human-readable diff ───────────────────────────────────────

banner('Test 4: dt office diff xlsx (human-readable)');

const r4 = runOffice(['diff', XLSX_LEFT, XLSX_RIGHT]);
check(r4.status === 1, 'exit code 1 (differences found)');
const stdout4 = r4.stdout.toString().trim();
check(stdout4.includes('Excel document'), 'mentions Excel document');
check(stdout4.includes('Text changed'), 'mentions Text changed');
check(stdout4.includes('Sheet1'), 'mentions Sheet1');

// ── Test 5: nonexistent file ───────────────────────────────────────────────

banner('Test 5: dt office diff with nonexistent file');

const r5 = runOffice(['diff', '/nonexistent/left.pptx', PPTX_RIGHT]);
check(r5.status !== 0, 'nonzero exit code for nonexistent file');
const stderr5 = r5.stderr.toString().trim();
check(stderr5.length > 0 || r5.stdout.toString().trim().length > 0,
  'produces error output');

// ── Test 6: nonexistent file --json ────────────────────────────────────────

banner('Test 6: dt office diff with nonexistent file --json');

const r6 = runOffice(['diff', PPTX_LEFT, '/nonexistent/right.pptx', '--json']);
check(r6.status !== 0, 'nonzero exit code for nonexistent file (json mode)');

// ── Test 7: both files nonexistent ─────────────────────────────────────────

banner('Test 7: dt office diff with both files nonexistent');

const r7 = runOffice(['diff', '/nonexistent/a.pptx', '/nonexistent/b.pptx']);
check(r7.status !== 0, 'nonzero exit code');

// ── Test 8: identical xlsx diff (same file twice) ──────────────────────────

banner('Test 8: dt office diff xlsx (identical files) --json');

const r8 = runOffice(['diff', XLSX_LEFT, XLSX_LEFT, '--json']);
check(r8.status === 0, 'exit code 0 for identical files');
try {
  const idResult = JSON.parse(r8.stdout.toString());
  check(idResult.summary.total_atomic_changes === 0,
    `identical files have 0 atomic changes, got: ${idResult.summary.total_atomic_changes}`);
  check(idResult.summary.sections_unchanged === idResult.summary.sections_total,
    'all sections are unchanged');
} catch (e) {
  fail(`identical files JSON parse: ${e.message}`);
}

// ── Test 9: help output (office diff --help shows --json flag) ────────

banner('Test 9: dt office diff --help');

const r9 = runOffice(['diff', '--help']);
check(r9.status === 0, 'exit code 0');
const stdout9 = r9.stdout.toString();
check(stdout9.includes('office'), 'help mentions office');
check(stdout9.includes('diff'), 'help mentions diff subcommand');
check(stdout9.includes('--json'), 'help mentions --json flag');
check(stdout9.includes('.docx') || stdout9.includes('.pptx') || stdout9.includes('.xlsx') || stdout9.includes('Office'),
  'help mentions Office formats');

// ── instrumentation ────────────────────────────────────────────────────────

banner('Instrumentation');

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'office-cli-latest.json'),
  JSON.stringify({ fixture: 'dt office diff (e2e)', testCount: 9 }, null, 2) + '\n',
);
pass('wrote instrumentation to target/e2e/office-cli-latest.json');

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
