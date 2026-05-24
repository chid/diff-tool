# Cross-Platform Diff Tool Implementation Plan

## Product Direction

Build a local-first desktop diff/merge app for macOS, Windows, and Linux. The
app competes with Kaleidoscope, Beyond Compare, Araxis, DeltaWalker, and P4Merge
as a standalone comparison tool, while borrowing Git-context ideas from Sublime
Merge, Tower, Fork, GitKraken, and SmartGit.

`@pierre/diffs` and `@pierre/trees` are Phase 1 build-vs-buy candidates, not
market competitors. The default Phase 1 path is to wrap them for the UI and keep
the core diff, filesystem, Git, merge, and image work in Rust.

## Phases

1. Dependency and UX spike: validate Tauri + React, `@pierre/diffs`, and
   `@pierre/trees` against large diff/tree workloads.
2. Rust core: implement stable backend models for text diffs, folder diffs,
   Git status, conflicts, and image metadata.
3. Desktop product: build file/folder/Git/conflict workflows on top of the
   stable IPC contract.
4. Test corpus and performance: import open fixtures, add golden tests, and
   benchmark against large repositories.
5. AI features: optional BYOK enrichment layer using OpenAI/Anthropic-compatible
   backends for change summaries, merge resolution, risk scoring, intent
   clustering, review questions, commit messages, and agent-review workspaces.
   AI is purely additive — the tool is fully functional without it.

See [ADR 0002](docs/decisions/0002-ai-features-byok.md) for the architecture
decision and [docs/ai-features.md](docs/ai-features.md) for the full ideation
catalog.

## Exit Criteria For Phase 1

- The desktop shell can render a changed-file tree and a split diff.
- The UI wrapper layer can switch between Pierre-backed and fallback rendering.
- A local Git repo can be scanned from Rust and displayed in the app.
- Rust tests and TypeScript checks run from one command.
- Packaging blockers, license issues, and performance limits are documented.

## Documentation System

Use the root docs as the canonical handoff surface:

- `README.md`: project overview, setup, common commands, and current status.
- `TODO.md`: prioritized backlog and known technical debt.
- `CONTRIBUTING.md`: contribution workflow and quality bar.
- `docs/architecture.md`: current system boundaries and data flow.
- `docs/design.md`: the Editorial Diff visual language and prompt pack.
- `docs/development.md`: developer setup and common implementation workflows.
- `docs/testing-and-performance.md`: test commands, fixture rules, and perf
  runner behavior.
- `docs/e2e-integration-test-process.md`: process scaffold for the first
  end-to-end integration test before full instrumentation is added.
- `docs/decisions/`: architecture decision records for choices that should not
  be rediscovered by each new agent.
