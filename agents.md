# Agent Handoff And Work Packages

This file is a compaction-style handoff for future agents working in
`/Users/charley/dev/diff-tool`. Treat it as the quick context restore point:
read this first, then read `README.md`, `TODO.md`, and the relevant crate docs.

## Current Project State

- The repo is a cross-platform diff/merge tool prototype.
- The last committed scaffold commit is `0d3ece2 Scaffold diff tool prototype and tests`.
- The committed scaffold includes:
  - Rust workspace with `diff-core`, `git-adapter`, and `diff-cli`.
  - Tauri 2 + React/Vite desktop shell in `apps/desktop`.
  - Example inputs under `examples/`.
  - Fixture cases under `fixtures/cases/`.
  - CLI, adapter, and diff-core tests.
  - Performance runner at `scripts/measure-performance.mjs`.
- The desktop UI is still a fallback/mock workbench; it does not yet use
  `@pierre/diffs` or `@pierre/trees` in the live components.
- The current Rust diff implementation emits one full-file hunk and uses
  `similar` for line matching.
- The current `git-adapter` is a single-root file tree scanner, not a real
  left/right folder diff or Git backend.

## Current Dirty/Local State To Preserve

At the time this handoff was written, the working tree had local changes and
untracked files. Do not revert or delete them unless the user explicitly asks.

- `.gitignore` has local ignore additions such as `.antigravitycli` and
  `/handoff.html`.
- Documentation files were being added or updated:
  - `README.md`
  - `CONTRIBUTING.md`
  - `TODO.md`
  - `docs/architecture.md`
  - `docs/development.md`
  - `docs/testing-and-performance.md`
  - `docs/decisions/0001-rust-core-tauri-shell.md`
  - `docs/implementation-plan.md`
- `agents.md` itself is untracked unless a later agent stages it.
- Other local/untracked files seen during handoff:
  - `apps/desktop/src/App.tsx.bak`
  - `apps/desktop/src/components/DiffPane.tsx.bak`
  - `apps/desktop/src/components/FileTreePane.tsx.bak`
  - `handoff.agy.html`
- `apps/desktop/src/components/FileTreePane.tsx` had a local modification not
  made as part of this handoff. Inspect it before editing UI code.

## Commands That Should Work

Use these before handing work back:

```bash
./scripts/check.sh
npm run perf:quick
```

Useful direct commands:

```bash
npm_config_cache=/tmp/npm-cache npm install
npm run dev
npm run build
npm run test
cargo run -p diff-cli -- diff examples/text-basic/left.txt examples/text-basic/right.txt --json
cargo run -p diff-cli -- dir examples/folder-basic/left --json
```

Use `CARGO_HOME=/tmp/cargo-home` if Cargo cannot write to the default global
cache. Use `npm_config_cache=/tmp/npm-cache` if npm cannot write to `~/.npm`.

## Known Performance Baseline

The full synthetic perf suite writes `target/perf/latest.json`.

Known baseline from the initial run:

- `sparse-edits-100k-lines`: about 62 ms.
- `tail-edit-10mb`: about 233 ms.
- `many-small-lines-churn`: about 68 seconds.

The many-small-lines case is intentionally important. It exposes the current
engine's pathological behavior and should remain a regression guard while
testing alternative algorithms.

## Prioritized Next Work

Start with `TODO.md`, but the practical order is:

1. Add `npm run test:e2e` following
   `docs/e2e-integration-test-process.md`.
2. Replace the single full-file hunk with real context-aware hunk generation.
3. Add intraline spans for adjacent delete/add lines.
4. Add binary and invalid UTF-8 detection before lossy text conversion.
5. Investigate `many-small-lines-churn` and compare `similar`, `imara-diff`,
   and histogram-style strategies.
6. Implement true left/right folder comparison in `git-adapter`.
7. Spike `@pierre/diffs` and `@pierre/trees` behind wrappers in `apps/desktop`.
8. Add UI smoke tests after the renderer strategy is clearer.

## Architecture Rules

- Backend crates must not depend on desktop/UI packages.
- Keep JSON models deterministic and serializable.
- Prefer fixture-driven tests for diff behavior.
- Keep generated perf data under `target/perf/`; do not commit it.
- Do not commit unrelated `.bak`, handoff HTML, local tool state, or user notes.
- If a file has existing local changes, inspect and preserve them rather than
  overwriting or reverting.

## Agent Work Packages

| Agent | Role | Target | Responsibilities |
|---|---|---|---|
| Agent A | Market and spec researcher | `docs/` | Competitive matrix, pricing, feature gaps, dependency licensing. |
| Agent B | Pierre UI spike owner | `apps/desktop` | Integrate `@pierre/diffs` and `@pierre/trees`, test Shadow DOM styling, workers, virtualization, packaging. |
| Agent C | Rust diff core developer | `crates/diff-core` | Hunk generation, intraline spans, binary/UTF-8 handling, diff algorithm experiments. |
| Agent D | Git and filesystem developer | `crates/git-adapter` | Left/right folder diff, ignore rules, Git status, conflict discovery. |
| Agent E | CLI and integration engineer | `crates/diff-cli` | `dt diff`, `dt dir`, `dt merge`, `dt git install`, exit codes, JSON/human output. |
| Agent F | QA, fixtures, and benchmarks | `fixtures/`, `examples/`, `scripts/`, `benches/` | Fixtures, perf budgets, Playwright/UI smoke tests, large-repo benchmark path. |

## Acceptance Pattern For Future Tasks

For any nontrivial change:

1. Add or update a fixture or test first.
2. Implement the smallest behavior needed.
3. Run `./scripts/check.sh`.
4. Run `npm run perf:quick` if the diff engine, CLI, filesystem scan, or UI
   rendering path changed.
5. Update `TODO.md` or docs when a task changes status or scope.
