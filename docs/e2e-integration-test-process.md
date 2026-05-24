# End-To-End Integration Test Process

This is the process scaffold for the first end-to-end test. It intentionally
defines the workflow before adding full instrumentation.

## Purpose

Prove that one user-visible flow works from fixture input through CLI/backend
data and into the desktop-facing model shape.

The first E2E test should not try to cover every feature. It should cover one
boring path completely and make failures easy to locate.

## First Scenario: Text Diff Happy Path

Use `examples/text-basic` as the initial fixture.

Flow:

1. Prepare known left/right text files.
2. Run the release or debug `dt diff <left> <right> --json` command.
3. Parse stdout as `DiffResult`.
4. Assert expected stats:
   - additions: `2`
   - deletions: `1`
   - unchanged: `2`
5. Assert at least one file entry and one hunk are present.
6. Assert all line entries include valid `kind`, `old_no`, `new_no`, and `text`
   fields.
7. Optionally feed the parsed result to a desktop fixture-render step once the
   UI harness exists.

The first test should fail loudly if JSON parsing fails, the command exits
non-zero, the schema changes, or stats drift unexpectedly.

## Proposed File Layout

```text
tests/e2e/
  README.md
  text_diff_happy_path.test.mjs
  fixtures/
    text-basic-left.txt
    text-basic-right.txt
```

Alternative if Rust-only is preferred later:

```text
crates/diff-cli/tests/e2e_text_diff.rs
```

Start with the Node script because it can later orchestrate CLI runs, dev
server startup, Playwright, screenshots, and instrumentation output in one
place.

## Runner Contract

Add a future npm script:

```json
{
  "scripts": {
    "test:e2e": "node tests/e2e/text_diff_happy_path.test.mjs"
  }
}
```

The runner should:

- Build or locate the `dt` binary.
- Use repository-relative fixture paths.
- Write transient logs to `target/e2e/`.
- Exit `0` on pass and non-zero on failure.
- Print a short summary to stdout.

## Initial Assertions

For the text diff happy path, assert:

- CLI command exits successfully.
- stdout is parseable JSON.
- top-level keys include `files`, `stats`, `warnings`, and `truncated`.
- `stats.additions`, `stats.deletions`, and `stats.unchanged` match expected
  values.
- `truncated` is `false`.
- every line has one of `context`, `addition`, or `deletion`.
- deleted lines have `old_no` and no `new_no`.
- added lines have `new_no` and no `old_no`.
- context lines have both line numbers.

## Instrumentation To Add Later

Do not block the first E2E test on full instrumentation. Add these in follow-up
steps:

- command duration
- JSON byte size
- input byte size
- input line counts
- peak memory, platform-specific at first
- spawned command and args
- binary version or git commit
- fixture name
- screenshot path once UI rendering is included
- trace path once Playwright is included

Instrumentation output should be JSON under:

```text
target/e2e/latest.json
```

## Future Scenarios

Add scenarios in this order:

1. Text diff happy path.
2. Whitespace-ignore text diff. ✅ (`npm run test:e2e:whitespace-ignore`)
2b. Whitespace-and-case-ignore text diff. ✅ (`npm run test:e2e:whitespace-case-ignore`)
3. Folder scan happy path. ✅ (`npm run test:e2e:folder-scan`)
4. CLI error path for missing file. ✅ (`npm run test:e2e:error-path`)
5. Desktop render smoke test using a static `DiffResult`. ✅ (`npm run test:e2e` — `desktop_render_smoke.spec.ts`)
6. Desktop render test backed by CLI output. ✅ (`npm run test:e2e:cli-smoke`)
7. Merge placeholder behavior.
8. Real merge behavior once implemented.

## Acceptance Criteria For The First PR

- `npm run test:e2e` exists.
- It runs from a clean checkout after `npm install`.
- It does not require the Tauri desktop window.
- It writes only under `target/e2e/`.
- It passes on macOS and should be portable to Linux/Windows.
- It is documented in `README.md` and `docs/testing-and-performance.md`.
