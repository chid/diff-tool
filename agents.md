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

## Tree-Sitter Syntax-Aware Diffing — Implemented on `feat/tree-sitter-syntax-diffing`

A detailed implementation plan and research doc is at
[docs/tree-sitter-diffing.html](docs/tree-sitter-diffing.html).

### What's built (Phases 0-3)

- **`crates/diff-core/Cargo.toml`** — Added `tree-sitter`, `tree-sitter-rust`,
  `tree-sitter-typescript`, `tree-sitter-python` dependencies.
- **`crates/diff-core/src/parser.rs`** — `Language` enum (Rust/TypeScript/Python),
  `language_from_extension()`, `parse_file()` → `Vec<AstNode>`, `build_child_links()`.
  Statement-depth node filtering with whitespace-normalized content hashing.
- **`crates/diff-core/src/ast_diff.rs`** — `compute_ast_diff()` with 4-pass
  matching: hash-based (unique), iterative bottom-up (parent-child chain),
  parent-child propagation, positional heuristic (moves/renames). 8 `ChangeClass`
  variants: Identical, Modified, Moved, Renamed, Added, Deleted, Reformatted,
  TypeChanged.
- **`crates/diff-core/src/ast_to_hunks.rs`** — `ast_diff_to_hunks_full()` converts
  AST diff to `Vec<LineChange>` with line-level event mapping.
- **`crates/diff-core/src/lib.rs`** — `DiffAlgorithm::Syntax` variant. Pipeline
  branching in `run_diff()` and `run_diff_with_deadline()`. Size gate (10K lines),
  language detection, silent fallback to `imara-diff` for unsupported languages.

### Completed (Phases 4-5)

- **Phase 4: CLI flags** — `--algorithm syntax`, `--language <lang>`, `--syntax-depth <level>`
  wired in `crates/diff-cli/src/main.rs`. Added `Syntax` to `AlgorithmArg` enum.
  Propagated `language` and `syntax_depth` through `DiffRequest` → `compare_texts` →
  `run_diff` → `try_run_syntax_diff`.
- **Phase 5: Fixtures + perf benchmarks** — 8 syntax fixture cases under
  `fixtures/cases/syntax-*` (identical, rename, add, delete, modify, move, type-change,
  mixed). Added `SyntaxDepth` enum (`Function`/`Statement`/`Token`) to `parser.rs`.
  Excluded `source_file` root wrapper from statement nodes. Added `has_unmatched_ancestor`
  dedup in `ast_to_hunks.rs` to avoid nested duplicate events. Updated perf harness
  with `syntax-equal-1k-fns` and `syntax-sparse-fn-edit` scenarios.
- `Syntax` diffing is ~25% slower than Myers for 100-function files (~4ms vs ~3ms).

### Status

Feature is **experimental** — emits a warning on use. Known limitations documented
in `ROUGH_EDGES.md`.

### Next steps

- **SemanticTag on Hunk** — Add `ChangeClass` to `Hunk` struct for per-hunk
  classification badges in the desktop UI.

## Next Work Priority

In order:

1. **Desktop open flow (Phase 3)** — Wire live Tauri IPC so the desktop app calls real Rust backends.
   See `docs/tauri-ipc-wiring-plan.md`. Phase 1 and 2 are done.
2. **Desktop rendering contract** — Decide Rust's `FileDiffMetadata` vs `@pierre/diffs` format.
3. **Office cleanup** — Suppress compiler warnings, document as experimental.
4. **Packaging prep** — Bundle config, icons, macOS preview build.
5. **SemanticTag on Hunk** — Add `ChangeClass` to Hunk for per-hunk badges in desktop UI.

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
