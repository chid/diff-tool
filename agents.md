# Agent Handoff And Work Packages

This file is a compaction-style handoff for future agents working in
`/Users/charley/dev/diff-tool`. Treat it as the quick context restore point:
read this first, then read `README.md`, `TODO.md`, and the relevant crate docs.

## Current Project State

- The repo is a cross-platform diff/merge tool prototype targeting MVP desktop ship.
- Current commit scaffold: `0d3ece2 Scaffold diff tool prototype and tests`.
- The `./scripts/check.sh` script (npm run check + cargo test --workspace) is green (150 tests, 0 failures).
- The `npm run perf:quick` harness now correctly handles `dt diff` exit codes 0 and 1 as successful.
- The workspace test suite was stabilized: diff-core fixture discovery now excludes Office-adapter fixtures.

### Just-Completed Fixes (2025-05-25)

These fixes addressed the immediate blockers in `PLAN.md`:

1. **Test layout stabilized** — `crates/diff-core/tests/fixture_cases.rs` now filters fixture
   directories by checking for `"ignore"` in `case.json` (text-diff schema), ignoring
   Office fixtures (which use `"format"` instead). The office-adapter already had the
   inverse filter. Both crates now coexist in `fixtures/cases/` without interference.

2. **Perf harness fixed** — `scripts/measure-performance.mjs` now treats `dt diff` exit
   codes 0 (identical) and 1 (differences) as successful. JSON is parsed for both.
   Console display shows `ok` for both.

3. **CLI `dt dir` rejects bad roots** — Before calling `scan_tree`, the CLI now checks
   `root.exists()` and `root.is_dir()`, returning exit 2 with a clear error message for
   nonexistent paths or files-passed-as-directories. WalkBuilder's silent empty-result
   behavior is avoided.

4. **Unimplemented commands return exit 2** — `dt merge` and `dt git install` now print
   `error: not implemented` to stderr and return `EXIT_ERROR` (exit 2) instead of exit 0.
   Tests added for both.

5. **Tests added** for dir-exit-2-when-file-not-dir, merge-exit-2-not-implemented,
   git-install-exit-2-not-implemented.

## Remaining PLAN.md Items (Ordered By Priority)

The following items from `PLAN.md` still need to be addressed. Items marked ✅ are done.

- [✅] Stabilize the test layout — office fixtures don't break text fixture discovery
- [✅] Fix perf harness to treat dt diff exit codes 0 and 1 as successful
- [✅] Clean CLI release behavior — dt dir rejects nonexistent roots, unimplemented commands return exit 2
- [ ] Make Rust DiffResult the MVP desktop rendering contract (see Phase 3 below)
- [ ] Finish desktop open flow: no hardcoded defaults in Tauri mode; support left/right file dialogs,
  folder picker for scan, loading/error/empty states, and binary-file errors from Rust
- [ ] Keep MVP folder support scoped to single-root scan (done in code; label UI and docs accordingly)
- [ ] Park Office work: keep office-adapter green, clean warning noise, document as experimental
- [ ] Prepare packaging: commit Tauri capabilities/icons, set bundle config, produce macOS local preview build
- [ ] Add external Git and folder-diff benchmarks at realistic scales (linux, vscode)
- [ ] Fuzz diff and scanning paths with large random inputs and assertions

## Next Work Priority

In order:

1. **Desktop open flow (Phase 3)** — Wire live Tauri IPC so the desktop app calls real Rust backends.
   See `docs/tauri-ipc-wiring-plan.md`. Phase 1 (read_file_text) and Phase 2 (scan_tree) are done.
   Phase 3: file open dialogs, drag and drop, command-line args. Phase 4: Rust-format vs pierre-format
   decision.

2. **Desktop rendering contract** — Decide whether the shipped renderer uses Rust's `FileDiffMetadata`
   (native) or converts to `@pierre/diffs` format. The conversion layer in TypeScript
   (`parseDiffFromFile`) should be replaced by Rust outputting the target format directly.

3. **Office cleanup** — Suppress the ~100 compiler warnings in office-adapter, ensure it
   doesn't appear in desktop/CLI public surface for MVP, and document as experimental.

4. **Packaging prep** — Set Tauri bundle config intentionally, commit icons/capabilities,
   produce a macOS preview build.

## Known Performance Baseline

Perf harness is green. Quick mode baseline (latest run):

- `equal-1k-lines`: ok, ~179ms
- `sparse-edits-1k-lines`: ok, ~2.7ms
- `whitespace-only-churn`: ok, ~99.9ms

The `many-small-lines-churn` case (full suite, not quick) is still ~68 seconds.
This remains a regression guard for future algorithm exploration.

## Commands That Should Work

```bash
./scripts/check.sh                          # npm run check + cargo test --workspace
npm run perf:quick                          # Performance harness (quick mode)
npm run perf                                # Full performance suite
cargo test -p diff-core                     # Text-diff unit + fixture tests (13 tests)
cargo test -p diff-cli                      # CLI integration tests (19 tests)
cargo test -p git-adapter                   # Filesystem scan tests
cargo test -p office-adapter                # Office fixture tests (should remain green)
INSTA_UPDATE=no cargo test --workspace       # All tests with snapshot checks
```

## Architecture Rules

- Backend crates must not depend on desktop/UI packages.
- Keep JSON models deterministic and serializable.
- Prefer fixture-driven tests for diff behavior.
- Keep generated perf data under `target/perf/`; do not commit it.
- Do not commit unrelated `.bak`, handoff HTML, local tool state, or user notes.
- If a file has existing local changes, inspect and preserve them rather than overwriting or reverting.

## Agent Work Packages

| Agent | Role | Target | Responsibilities | Status |
|---|---|---|---|---|
| Agent A | Market and spec researcher | `docs/` | Competitive matrix, pricing, feature gaps, dependency licensing. | Not started |
| Agent B | Pierre UI spike owner | `apps/desktop` | Integrate `@pierre/diffs` and `@pierre/trees`, test Shadow DOM styling, workers, virtualization, packaging. | In progress (spikes done, renderer ADR pending) |
| Agent C | Rust diff core developer | `crates/diff-core` | Hunk generation, intraline spans, binary/UTF-8 handling, diff algorithm experiments. | Most P0 items done. Perf improvements remain. |
| Agent D | Git and filesystem developer | `crates/git-adapter` | Left/right folder diff, ignore rules, Git status, conflict discovery. | Single-root scan only. |
| Agent E | CLI and integration engineer | `crates/diff-cli` | `dt diff`, `dt dir`, `dt merge`, `dt git install`, exit codes, JSON/human output. | Core commands solid. merge/git stub properly. |
| Agent F | QA, fixtures, and benchmarks | `fixtures/`, `examples/`, `scripts/`, `benches/` | Fixtures, perf budgets, Playwright/UI smoke tests, large-repo benchmark path. | Perf harness fixed. |

## Acceptance Pattern For Future Tasks

For any nontrivial change:

1. Add or update a fixture or test first.
2. Implement the smallest behavior needed.
3. Run `./scripts/check.sh`.
4. Run `npm run perf:quick` if the diff engine, CLI, filesystem scan, or UI rendering path changed.
5. Update `TODO.md` and `agents.md` when a task changes status or scope.
