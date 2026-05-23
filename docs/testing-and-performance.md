# Testing And Performance

## Standard Check

Run:

```bash
./scripts/check.sh
```

This runs:

- TypeScript checks for the desktop app.
- Rust tests for all workspace crates.

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
