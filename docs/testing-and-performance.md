# Testing And Performance

## Standard Check

Run:

```bash
./scripts/check.sh
```

This runs:

- TypeScript checks for the desktop app.
- Rust tests for all workspace crates.

## MVP Gate

Run this before treating a branch as MVP-ready:

```bash
npm run test:mvp
```

This runs the TypeScript and Rust checks, the MVP boundary contract, all
scripted CLI E2E tests, Playwright desktop/browser tests, and the quick
performance suite. It is intentionally scoped to the MVP product: local text
diff, CLI behavior, single-root folder scan, desktop rendering, and packaging
readiness signals.

Run the package smoke separately when you need a local macOS artifact:

```bash
npm run test:mvp:package
```

That command builds the Tauri app bundle. It is separated from `test:mvp`
because it is slower and platform-specific.

## Rust Tests

```bash
CARGO_HOME=/tmp/cargo-home cargo test --workspace
```

Test coverage currently includes:

- inline `diff-core` unit tests
- fixture-driven `diff-core` cases
- `git-adapter` scan tests
- `dt` CLI integration tests

## Fixtures

Fixture cases live under `fixtures/cases`.

The test harness reads every directory with a `case.json`, so adding a new case
does not require modifying test code unless the schema changes.

## Performance

Quick suite:

```bash
npm run perf:quick
```

Full synthetic suite:

```bash
npm run perf
```

Reports are written to:

```text
target/perf/latest.json
```

Generated input data is written to:

```text
target/perf/fixtures/
```

## Current Perf Cases

- `equal-1k-lines`
- `sparse-edits-1k-lines`
- `sparse-edits-100k-lines`
- `tail-edit-10mb`
- `whitespace-only-churn`
- `many-small-lines-churn`

## Known Performance Risk

`many-small-lines-churn` is intentionally pathological for the current engine.
It should remain in the suite as the main regression guard while evaluating
alternate diff algorithms.

## Future Test Work

- Add the first E2E integration runner using
  [the E2E process](e2e-integration-test-process.md).
- Add UI smoke tests once the desktop harness is stable.
- Add large-repo optional benchmarks for Rust, CPython, LLVM, and Linux.
- Add platform-specific memory measurement.
- Add snapshot tests for CLI human output after output style stabilizes.

## E2E Tests

Node-based E2E tests exercise the CLI from end to end:

```bash
npm run test:e2e:cli                     # Text diff happy path
npm run test:e2e:whitespace-ignore       # --ignore-whitespace (zero diffs)
npm run test:e2e:whitespace-case-ignore  # --ignore-whitespace-and-case (zero diffs)
npm run test:e2e:folder-scan             # Folder scan happy path
npm run test:e2e:error-path             # CLI error paths (exit code 2)
npm run test:e2e:context               # --context flag hunk grouping
npm run test:e2e:progress-diff          # dt diff --progress stderr/stdout split
npm run test:e2e:progress-dir           # dt dir --right --progress behavior
npm run test:e2e:config                 # isolated CLI config get/set/reset
npm run test:e2e:mvp-boundary           # MVP claim and script contract
npm run test:e2e:office                 # experimental Office CLI, not in MVP gate
```

Playwright-based E2E tests verify the desktop renderer:

```bash
npm run test:e2e                         # Browser harness + smoke tests
npm run test:e2e:cli-smoke              # CLI-backed desktop render smoke test
```

Each test:

- Builds the `dt` binary from source (`cargo run -p diff-cli`).
- Runs a known fixture through the CLI.
- Asserts JSON output shape, exit codes, and expected stats.
- Writes instrumentation to `target/e2e/*.json`.
- Exits `0` on pass and non-zero on failure.

Office, AI, image, merge, Git, and true left/right folder comparison tests are
post-MVP unless explicitly included in `npm run test:mvp`. If those crates or
routes remain in the workspace, they still need to stay green under the Rust
workspace tests and their own explicit scripts.
