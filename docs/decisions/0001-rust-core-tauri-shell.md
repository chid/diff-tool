# ADR 0001: Rust Core With Tauri Desktop Shell

## Status

Accepted for the scaffold.

## Context

The app needs to be cross-platform, local-first, and fast on large diffs and
folder trees. It also needs a rich UI that can iterate quickly while the engine
is still changing.

## Decision

Use Rust for core comparison, filesystem, Git, CLI, and future merge/image work.
Use Tauri 2 with React/Vite for the desktop shell.

## Consequences

- Backend behavior is testable without launching the desktop app.
- CLI and desktop can share the same data models.
- Tauri packaging needs platform-specific validation later.
- React can integrate web-native diff/tree renderers such as `@pierre/diffs`
  and `@pierre/trees`.
