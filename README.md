# Diff Tool

[![Live Demo](https://img.shields.io/badge/demo-live%20site-brightgreen)](https://chid.github.io/diff-tool/)

Cross-platform, local-first diff and merge tool prototype powered by a high-performance Rust core and Pierre UI.

👉 **[Try the Live Web Demo](https://chid.github.io/diff-tool/)** (Interactive diff playground, GitHub PR & commit review, syntax-aware tree-sitter diffing, and CLI simulator).

### Compare documents

Open **Documents** to compare Word, PDF, Excel, and PowerPoint files on your device. Choose **Download review report** for a printable summary with supporting passages or highlighted PDF pages. The self-contained HTML file opens offline. For editable Word revisions, use **Download tracked Word report**. [Example supplier report](https://chid.github.io/diff-tool/document-demos/supplier-report.html) · [Document comparison and benchmark](docs/document-comparison.md)

Try a fictional pair in the live app. Each example includes both source files.

- Follow a moved packing clause and a shorter payment deadline in the [supplier agreement (Word)](https://chid.github.io/diff-tool/?tab=documents&example=supplier).
- Review a larger first shipment and new bottle artwork in the [coffee launch deck (PowerPoint)](https://chid.github.io/diff-tool/?tab=documents&example=launch).
- Find the aisle that opened between two tables in the [reading room floor plan (PDF)](https://chid.github.io/diff-tool/?tab=documents&example=reading-room).

The [document feature roadmap](docs/document-feature-roadmap.md) describes 15 proposed features with delivery order and acceptance tests.

### Review AI governance changes with a client

Three cases pair dated company publications with analyst notes and a question to investigate with a client. Explore how Google described appeals, how Anthropic prioritized human oversight, or how OpenAI addressed emotional reliance on an assistant. [Google](https://chid.github.io/diff-tool/?tab=documents&example=google) · [Anthropic](https://chid.github.io/diff-tool/?tab=documents&example=anthropic) · [OpenAI](https://chid.github.io/diff-tool/?tab=documents&example=openai) · [Case dates and printable reports](docs/document-comparison.md#review-ai-governance-changes)

### Review Public GitHub PRs & Commits
You can inspect and review any public GitHub pull request or commit URL in the live demo:
- **Direct PR link**: [`https://chid.github.io/diff-tool/?pr=facebook/react#28000`](https://chid.github.io/diff-tool/?pr=facebook/react#28000)
- **Shorthand targets**: `owner/repo#prNumber` (e.g. `facebook/react#28000`, `tailwindlabs/tailwindcss#14000`) or `owner/repo@sha`
- **Full URLs**: Paste any `https://github.com/.../pull/...` or `.../commit/...` URL
- **Side-by-side split & unified view**: Fast syntax highlighting, intraline character spans, and file list navigation powered by `@pierre/diffs`.

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

## Original scaffold status

The lists below describe the initial scaffold. The architecture guide traces the implemented browser, desktop, and CLI paths. The developer guide walks through the same Word comparison in two runtimes. [Architecture](docs/architecture.md) · [Developer walkthrough](docs/development.md)

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
