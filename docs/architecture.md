# Architecture

The product is split into a deterministic Rust core and a desktop shell. The
desktop shell should never own expensive diff, Git, merge, image, or filesystem
logic.

## System Shape

```text
Desktop UI
  React components
  Tauri command calls
  Pierre wrappers or fallback renderers

Tauri Bridge
  IPC request/response models
  command handlers

Rust Core
  diff-core
  git-adapter
  diff-cli
```

## Rust Crates

### `diff-core`

Owns text comparison models and algorithms.

Current responsibilities:

- `DiffRequest`
- `DiffResult`
- `FileDiff`
- `Hunk`
- `LineChange`
- ignore modes
- basic file comparison

Planned responsibilities:

- multi-hunk output
- intraline spans
- binary detection
- encoding and line-ending policy
- merge model support shared by CLI and desktop

### `git-adapter`

Owns filesystem and Git-facing data. Despite the name, it currently implements a
single-root file tree scan only.

Planned responsibilities:

- left/right folder comparison
- ignore rules
- Git status and commit comparison
- conflict discovery
- progress events for long scans

### `diff-cli`

Owns headless workflows and Git-tool integration.

Current commands:

- `dt diff <left> <right> [--json]`
- `dt dir <root> [--json]`
- placeholder `dt merge`
- placeholder `dt git install`

## Desktop App

`apps/desktop` is a Tauri 2 app with a React/Vite frontend.

Current UI:

- Pierre-backed diff and tree wrappers (live in `integrate-pierre-libs`).
- JSON-driven preferences system with 25 settings and 45 tests.
- Mock data fallback for Vite dev mode (no Rust backend).

Target UI:

- Live Tauri IPC wiring — see [Tauri IPC Wiring Plan](tauri-ipc-wiring-plan.md).
- Fallback renderers retained for testability and packaging risk.
- Typed IPC boundary between React and Rust.

## Data Flow

1. User opens files, folder, repository, or conflict.
2. React calls a Tauri command with typed request data.
3. Rust reads data, computes comparison output, and returns JSON-serializable
   models.
4. React renders the result using Pierre components or fallback components.
5. CLI commands use the same Rust crates and model shape.

## Design Constraints

- Keep public models serializable and stable.
- Avoid UI dependencies in backend crates.
- Prefer fixture-backed tests for behavior.
- Prefer synthetic perf cases before large external benchmark repositories.
- Treat generated files as disposable unless they are required source inputs.
