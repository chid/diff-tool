#!/usr/bin/env node

/**
 * End-to-end test: CLI `dt config` subcommand (get, set, reset).
 *
 * Tests all config paths using a temporary XDG_CONFIG_HOME to avoid
 * touching the user's real config file.
 *
 * See docs/e2e-integration-test-process.md for the specification.
 */

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
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

// ── setup: isolated config directory ───────────────────────────────────────

const tempConfigHome = mkdtempSync(resolve(tmpdir(), 'diff-tool-e2e-config-'));
const env = {
  ...process.env,
  XDG_CONFIG_HOME: tempConfigHome,
  CARGO_HOME: '/tmp/cargo-home',
};

function runConfig(args) {
  return spawnSync(
    'cargo',
    [
      'run',
      '-p', 'diff-cli',
      '--quiet',
      '--',
      'config',
      ...args,
    ],
    {
      cwd: PROJECT_ROOT,
      env,
      stdio: 'pipe',
      timeout: 30_000,
    },
  );
}

// ── cleanup helper ─────────────────────────────────────────────────────────

function cleanup() {
  try { rmSync(tempConfigHome, { recursive: true, force: true }); } catch {}
}
process.on('exit', cleanup);

// ── Test 1: get with no config ─────────────────────────────────────────────

banner('Test 1: dt config get (no config)');

const r1 = runConfig(['get']);
check(r1.status === 0, 'exit code 0');
const stdout1 = r1.stdout.toString().trim();
check(stdout1 === '(no config set — using built-in defaults)', `stdout: ${stdout1}`);
check(r1.stderr.toString().trim() === '', 'stderr is empty');

// ── Test 2: set algorithm ──────────────────────────────────────────────────

banner('Test 2: dt config set algorithm histogram');

const r2 = runConfig(['set', 'algorithm', 'histogram']);
check(r2.status === 0, 'exit code 0');
const stdout2 = r2.stdout.toString().trim();
check(stdout2.startsWith('Saved config to') && stdout2.endsWith('config.toml'),
  `stdout: ${stdout2}`);
check(r2.stderr.toString().trim() === '', 'stderr is empty');

// ── Test 3: get after set ──────────────────────────────────────────────────

banner('Test 3: dt config get (after set algorithm)');

const r3 = runConfig(['get']);
check(r3.status === 0, 'exit code 0');
const stdout3 = r3.stdout.toString().trim();
check(stdout3 === 'algorithm = histogram', `stdout: ${stdout3}`);

// ── Test 4: set deadline ───────────────────────────────────────────────────

banner('Test 4: dt config set deadline 5000');

const r4 = runConfig(['set', 'deadline', '5000']);
check(r4.status === 0, 'exit code 0');
const stdout4 = r4.stdout.toString().trim();
check(stdout4.startsWith('Saved config to'), `stdout: ${stdout4}`);

// ── Test 5: get shows both keys ────────────────────────────────────────────

banner('Test 5: dt config get (after set both)');

const r5 = runConfig(['get']);
check(r5.status === 0, 'exit code 0');
const stdout5 = r5.stdout.toString().trim();
check(stdout5.includes('algorithm = histogram'), 'contains algorithm = histogram');
check(stdout5.includes('deadline = 5000'), 'contains deadline = 5000');

// Verify the TOML file on disk matches what config get reports.
banner('Test 5b: verify config.toml file contents on disk');
const configFilePath = resolve(tempConfigHome, 'diff-tool', 'config.toml');
check(existsSync(configFilePath), 'config.toml exists on disk');
const tomlContents = readFileSync(configFilePath, 'utf-8');
check(tomlContents.includes('algorithm = "histogram"'),
  'TOML file contains algorithm = "histogram"');
check(tomlContents.includes('deadline = 5000'),
  'TOML file contains deadline = 5000');
check(tomlContents.startsWith('[defaults]'),
  'TOML file has [defaults] section header');

// ── Test 6: set overwrites algorithm ───────────────────────────────────────

banner('Test 6: dt config set algorithm patience (overwrite)');

const r6 = runConfig(['set', 'algorithm', 'patience']);
check(r6.status === 0, 'exit code 0');
const r6Get = runConfig(['get']);
check(r6Get.status === 0, 'exit code 0');
check(r6Get.stdout.toString().trim().startsWith('algorithm = patience'),
  `algorithm was overwritten: ${r6Get.stdout.toString().trim()}`);

// ── Test 7: set invalid key ────────────────────────────────────────────────

banner('Test 7: dt config set invalid_key foo');

const r7 = runConfig(['set', 'invalid_key', 'foo']);
check(r7.status === 2, 'exit code 2 for invalid key');
check(r7.stdout.toString().trim() === '', 'stdout is empty on error');
const stderr7 = r7.stderr.toString().trim();
check(stderr7.includes('unknown key'), `stderr mentions unknown key: ${stderr7}`);
check(stderr7.includes('algorithm') && stderr7.includes('deadline'),
  'stderr lists valid keys');

// ── Test 8: set invalid algorithm value ────────────────────────────────────

banner('Test 8: dt config set algorithm bogus');

const r8 = runConfig(['set', 'algorithm', 'bogus']);
check(r8.status === 2, 'exit code 2 for invalid algorithm');
const stderr8 = r8.stderr.toString().trim();
check(stderr8.includes('unknown algorithm'), `stderr: ${stderr8}`);
check(stderr8.includes('myers') && stderr8.includes('histogram') && stderr8.includes('patience'),
  'stderr lists valid algorithm values');

// ── Test 9: set invalid deadline value ─────────────────────────────────────

banner('Test 9: dt config set deadline abc');

const r9 = runConfig(['set', 'deadline', 'abc']);
check(r9.status === 2, 'exit code 2 for invalid deadline');
const stderr9 = r9.stderr.toString().trim();
check(stderr9.includes('deadline must be a positive integer'), `stderr: ${stderr9}`);

// ── Test 10: set deadline zero ─────────────────────────────────────────────

banner('Test 10: dt config set deadline 0');

const r10 = runConfig(['set', 'deadline', '0']);
check(r10.status === 0, 'exit code 0 (0 is a valid u64)');
const r10Get = runConfig(['get']);
check(r10Get.stdout.toString().includes('deadline = 0'), 'deadline = 0 is stored');

// ── Test 11: reset ─────────────────────────────────────────────────────────

banner('Test 11: dt config reset');

const r11 = runConfig(['reset']);
check(r11.status === 0, 'exit code 0');
const stdout11 = r11.stdout.toString().trim();
check(stdout11 === 'Config reset (file deleted).', `stdout: ${stdout11}`);
check(r11.stderr.toString().trim() === '', 'stderr is empty');

// Verify the config file is actually gone.
check(!existsSync(resolve(tempConfigHome, 'diff-tool', 'config.toml')),
  'config file removed from disk');

// ── Test 12: get after reset ───────────────────────────────────────────────

banner('Test 12: dt config get (after reset)');

const r12 = runConfig(['get']);
check(r12.status === 0, 'exit code 0');
const stdout12 = r12.stdout.toString().trim();
check(stdout12 === '(no config set — using built-in defaults)', `stdout: ${stdout12}`);

// ── Test 13: reset when no config exists ───────────────────────────────────

banner('Test 13: dt config reset (no-op, file already gone)');

const r13 = runConfig(['reset']);
check(r13.status === 0, 'exit code 0 (reset is idempotent)');
check(r13.stdout.toString().trim() === 'Config reset (file deleted).',
  'same success message');

// ── Test 14: get after double reset ────────────────────────────────────────

banner('Test 14: dt config get (after double reset)');

const r14 = runConfig(['get']);
check(r14.status === 0, 'exit code 0');
check(r14.stdout.toString().trim() === '(no config set — using built-in defaults)',
  'still shows defaults');

// ── Test 15: set deadline negative ─────────────────────────────────────────
// Note: `--` is required before the value so clap treats `-1` as positional.

banner('Test 15: dt config set deadline -- -1');

const r15 = runConfig(['set', 'deadline', '--', '-1']);
check(r15.status === 2, 'exit code 2 for negative deadline (not a valid u64)');
const stderr15 = r15.stderr.toString().trim();
check(stderr15.includes('deadline must be a positive integer'), `stderr: ${stderr15}`);

// ── instrumentation ────────────────────────────────────────────────────────

banner('Instrumentation');

mkdirSync(E2E_OUT_DIR, { recursive: true });
writeFileSync(
  resolve(E2E_OUT_DIR, 'config-latest.json'),
  JSON.stringify({ fixture: 'dt config (e2e)', testCount: 15 }, null, 2) + '\n',
);
pass('wrote instrumentation to target/e2e/config-latest.json');

// ── cleanup ────────────────────────────────────────────────────────────────

cleanup();

// ── summary ────────────────────────────────────────────────────────────────

banner('Summary');
if (failures === 0) {
  console.log('\n  All checks passed!');
  process.exit(0);
} else {
  console.error(`\n  ${failures} assertion(s) failed.`);
  process.exit(1);
}
