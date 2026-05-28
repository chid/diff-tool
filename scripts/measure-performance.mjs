#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const quick = process.argv.includes('--quick');
const perfRoot = path.join(root, 'target', 'perf');
const fixtureRoot = path.join(perfRoot, 'fixtures');
const reportPath = path.join(perfRoot, 'latest.json');

rmSync(fixtureRoot, { recursive: true, force: true });
mkdirSync(fixtureRoot, { recursive: true });

const cases = [
  {
    name: 'equal-1k-lines',
    quick: true,
    generate: () => pair(lines(1_000, (i) => `line ${i}`), lines(1_000, (i) => `line ${i}`)),
  },
  {
    name: 'sparse-edits-1k-lines',
    quick: true,
    generate: () =>
      pair(
        lines(1_000, (i) => `line ${i}`),
        lines(1_000, (i) => (i % 100 === 0 ? `changed ${i}` : `line ${i}`)),
      ),
  },
  {
    name: 'sparse-edits-100k-lines',
    quick: false,
    generate: () =>
      pair(
        lines(100_000, (i) => `line ${i}`),
        lines(100_000, (i) => (i % 5_000 === 0 ? `changed ${i}` : `line ${i}`)),
      ),
  },
  {
    name: 'tail-edit-10mb',
    quick: false,
    generate: () => {
      const base = lines(250_000, (i) => `payload ${i.toString().padStart(8, '0')} xxxxxxxxxxxxxxxxxxxxx`);
      return pair(base, `${base}tail changed\n`);
    },
  },
  {
    name: 'whitespace-only-churn',
    quick: true,
    generate: () =>
      pair(
        lines(5_000, (i) => `call(${i}, "value")`),
        lines(5_000, (i) => `call( ${i},   "value" )`),
      ),
  },
  {
    name: 'many-small-lines-churn',
    quick: false,
    generate: () =>
      pair(
        lines(200_000, (i) => `${i}`),
        lines(200_000, (i) => (i % 2 === 0 ? `${i}` : `${i}-x`)),
      ),
  },
].filter((testCase) => !quick || testCase.quick);

const build = spawnSync('cargo', ['build', '--release', '-q', '-p', 'diff-cli'], {
  cwd: root,
  encoding: 'utf8',
  env: { ...process.env, CARGO_HOME: process.env.CARGO_HOME || '/tmp/cargo-home' },
});
if (build.status !== 0) {
  console.error(build.stderr);
  process.exit(build.status ?? 1);
}

const binary = path.join(root, 'target', 'release', process.platform === 'win32' ? 'dt.exe' : 'dt');
const results = [];
for (const testCase of cases) {
  const dir = path.join(fixtureRoot, testCase.name);
  mkdirSync(dir, { recursive: true });
  const { left, right } = testCase.generate();
  const leftPath = path.join(dir, 'left.txt');
  const rightPath = path.join(dir, 'right.txt');
  writeFileSync(leftPath, left);
  writeFileSync(rightPath, right);

  const started = performance.now();
  const run = spawnSync(binary, ['diff', leftPath, rightPath, '--json'], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 512 * 1024 * 1024,
  });
  const durationMs = performance.now() - started;
  const leftBytes = Buffer.byteLength(left);
  const rightBytes = Buffer.byteLength(right);
  const totalMb = (leftBytes + rightBytes) / (1024 * 1024);

  // dt diff returns exit 0 when files are identical, exit 1 when differences
  // are found, and exit 2 on error.  Both 0 and 1 are successful executions.
  let stats = null;
  if (run.status === 0 || run.status === 1) {
    stats = JSON.parse(run.stdout).stats;
  }

  results.push({
    name: testCase.name,
    left_bytes: leftBytes,
    right_bytes: rightBytes,
    left_lines: countLines(left),
    right_lines: countLines(right),
    duration_ms: Math.round(durationMs * 100) / 100,
    throughput_mb_s: Math.round((totalMb / (durationMs / 1000)) * 100) / 100,
    exit_code: run.status,
    stats,
    stderr: (run.status === 0 || run.status === 1) ? undefined : run.stderr.slice(0, 4000),
  });
}

const report = {
  generated_at: new Date().toISOString(),
  tool_version: readPackageVersion(),
  platform: {
    os: os.platform(),
    arch: os.arch(),
    cpus: os.cpus().length,
    node: process.version,
  },
  quick,
  cases: results,
};

mkdirSync(perfRoot, { recursive: true });
writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);

for (const result of results) {
  const status = (result.exit_code === 0 || result.exit_code === 1) ? 'ok' : `exit ${result.exit_code}`;
  console.log(
    `${result.name}: ${status}, ${result.duration_ms}ms, ${result.throughput_mb_s} MB/s`,
  );
}
console.log(`Wrote ${path.relative(root, reportPath)}`);

function pair(left, right) {
  return { left, right };
}

function lines(count, render) {
  let output = '';
  for (let i = 0; i < count; i += 1) {
    output += `${render(i)}\n`;
  }
  return output;
}

function countLines(input) {
  return input.length === 0 ? 0 : input.split('\n').length - 1;
}

function readPackageVersion() {
  const pkg = JSON.parse(readFileSync(path.join(root, 'package.json'), 'utf8'));
  return pkg.version;
}
