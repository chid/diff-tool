# Diff Tool

[![Live Demo](https://img.shields.io/badge/demo-live%20site-brightgreen)](https://chid.github.io/diff-tool/)

Cross-platform, local-first diff and merge tool prototype powered by a high-performance Rust core and Pierre UI.

👉 **[Try the Live Web Demo](https://chid.github.io/diff-tool/)** (Interactive live diff playground, syntax-aware tree-sitter diffing, and CLI simulator).

The current repository is a working scaffold, not a finished product. It has a
Rust backend, a Tauri/React desktop shell, fixture-driven tests, example inputs,
and a synthetic performance harness.

## Goals

- Fast text, folder, Git, merge, and image comparison on macOS, Windows, and
  Linux.
- Local-first behavior with deterministic, testable core data models.
- A desktop UI that can use `@pierre/diffs` and `@pierre/trees` where they fit,
  while keeping the expensive comparison work in Rust.
- Developer ergonomics good enough for multiple agents or contributors to work
  in parallel without stepping on each other.

## Repository Layout

```text
apps/desktop/        Tauri 2 + React desktop shell
crates/diff-core/    Text diff models and comparison logic
crates/diff-tui/     Terminal diff preview renderer and TUI scaffold
crates/git-adapter/  Filesystem and future Git/folder integration
crates/diff-cli/     `dt` command-line entrypoint
docs/                Architecture, implementation, and process docs
examples/            Small runnable examples for humans and agents
fixtures/            Fixture cases for regression tests
scripts/             Test and performance helper scripts
benches/             Benchmark notes and future large-repo benchmark work
```

## Prerequisites

- Node.js 22+
- npm 10+
- Rust 1.95+
- A platform supported by Tauri 2

On this machine, use a temp npm cache until `~/.npm` ownership is fixed:

```bash
npm_config_cache=/tmp/npm-cache npm install
```

## Common Commands

```bash
npm run dev
npm run build
npm run test
npm run perf:quick
cargo run -p diff-cli -- diff examples/text-basic/left.txt examples/text-basic/right.txt --json
cargo run -p diff-cli -- tui examples/text-basic/left.txt examples/text-basic/right.txt
```

Use `./scripts/check.sh` before handing work to another agent or opening a PR.
It runs TypeScript checks and the Rust test suite with a writable Cargo cache.

## Current Status

Implemented:

- Workspace scaffold with Rust crates and desktop app.
- Initial diff JSON model and `dt diff` CLI.
- Deterministic `dt tui` split-pane scaffold for terminal diff UI work.
- Basic folder scan model and `dt dir` CLI.
- Tauri command bridge for file compare and tree scan.
- **`@pierre/diffs` integration** — `FileDiff` renders in split view with
  Shiki syntax highlighting preloaded via `main.tsx`.
- **`@pierre/trees` integration** — `FileTree` web component renders the
  changed-file list with Git status badges and selection callbacks.
- **Preferences system** — JSON-driven settings registry (`settings-registry.json`),
  `SettingsContext` with localStorage persistence, and an elegant `SettingsPanel`
  modal with category icons, modified-count badges, focus trap, and animations.
  Covers 25 settings across 5 categories: Display, Algorithm, Performance,
  Interaction, File Display.
- **Unit tests** — 45 vitest tests (SettingsContext + SettingsPanel) using
  `@testing-library/react` and `jsdom`.
- Fixture cases, CLI tests, adapter tests, examples, and performance runner.

Not implemented yet:

- Multi-hunk/context-aware diff output.
- Live Tauri bridge wiring (desktop currently runs mock data in Vite dev mode).
- Real Git status, commit comparison, blame, or conflict discovery.
- Real folder comparison between left/right trees.
- Merge resolution.
- Image/table/archive comparison.

See [TODO.md](TODO.md) for the prioritized backlog.

## Documentation

- [Architecture](docs/architecture.md)
- [Design Language](docs/design.md)
- [Development Guide](docs/development.md)
- [Testing and Performance](docs/testing-and-performance.md)
- [End-To-End Integration Test Process](docs/e2e-integration-test-process.md)
- [Implementation Plan](docs/implementation-plan.md)
- [MCP Interface Plan](docs/mcp-interface-plan.md)
- [Agent Work Packages](docs/agent-work-packages.md)
- [Contributing](CONTRIBUTING.md)

## Performance Baseline

Run:

```bash
npm run perf
```

The latest report is written to `target/perf/latest.json`. Generated perf
fixtures live under `target/perf/fixtures` and are intentionally not committed.

The current baseline has one known pathological case:
`many-small-lines-churn` takes about 68 seconds on the first measured machine.
That is expected to drive the next diff-engine improvement.
