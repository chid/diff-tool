# Contributing

This project is intentionally structured so humans and coding agents can work in
parallel. Keep changes small, documented, and easy to verify.

## Workflow

1. Read [README.md](README.md), [docs/architecture.md](docs/architecture.md),
   and [TODO.md](TODO.md).
2. Pick one clearly bounded task.
3. Add or update tests before changing shared behavior.
4. Run `./scripts/check.sh`.
5. For performance-sensitive changes, also run `npm run perf:quick`.
6. Update docs when behavior, commands, public models, or ownership changes.

## Quality Bar

- Preserve deterministic JSON output from Rust models.
- Keep backend crates independent of desktop UI libraries.
- Keep generated artifacts out of commits unless they are required source
  inputs, lockfiles, or Tauri schema files already tracked by convention.
- Avoid broad refactors unless the task explicitly needs them.
- Prefer fixture-driven tests for diff behavior.
- Add examples when a workflow becomes user-facing.

## Commit Guidance

Use focused commits with imperative messages, for example:

```text
Add fixture tests for whitespace diff modes
```

Do not include unrelated local files. Current known local-only paths include
`.antigravitycli/` and any ad-hoc notes unless they are intentionally promoted
to documentation.

## Agent Handoff

When handing work to another agent, include:

- The exact target files or crate.
- Expected command to run.
- Acceptance criteria.
- Known constraints or failing tests.
- Whether generated artifacts should be committed.
