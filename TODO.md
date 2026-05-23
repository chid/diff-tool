# TODO

Priorities are ordered for efficient implementation. Do these before adding
large new product surfaces.

## P0: Make The Scaffold Production-Shaped

- [ ] Add `npm run test:e2e` using the Markdown process in
      `docs/e2e-integration-test-process.md`.
- [ ] Replace single full-file hunk output with real hunk generation using the
      `context` value in `DiffRequest`.
- [ ] Add intraline spans for adjacent delete/add pairs.
- [ ] Normalize or explicitly model line endings so CRLF/LF behavior is
      intentional and documented.
- [ ] Add binary detection before lossy UTF-8 conversion.
- [ ] Add invalid UTF-8 fixtures and tests.
- [ ] Make `dt diff` exit non-zero for unreadable inputs and invalid arguments
      with documented exit codes.
- [ ] Add stable CLI output snapshots for human and JSON modes.

## P1: Fix The Performance Hot Path

- [ ] Investigate `many-small-lines-churn`, currently about 68 seconds in the
      synthetic full perf suite.
- [ ] Compare `similar`, `imara-diff`, and a histogram strategy on the existing
      perf cases.
- [ ] Add a perf budget file with initial thresholds for quick cases.
- [ ] Fail `npm run perf:quick` only after thresholds are intentionally agreed.
- [ ] Add peak memory measurement for macOS first, then portable support.

## P2: Folder And Git Backend

- [ ] Implement true left/right folder comparison instead of single-root scan.
- [ ] Add added/deleted/modified/unchanged status classification.
- [ ] Respect `.gitignore`, `.ignore`, and explicit include/exclude filters.
- [ ] Add rename detection behind an explicit option.
- [ ] Add Git repository discovery and working-tree status.
- [ ] Add commit/branch comparison API.
- [ ] Add conflict discovery for unresolved merge markers and Git index stages.

## P3: Desktop UI

- [x] Replace fallback diff pane with a wrapped `@pierre/diffs` spike.
- [x] Replace fallback tree pane with a wrapped `@pierre/trees` spike.
- [x] Preferences system with JSON-driven registry, SettingsContext, SettingsPanel (25 settings, 45 tests).
- [x] Settings panel UI with category icons, modified badges, focus trap, animations.
- [ ] TODO: Wire live Tauri IPC so the desktop app calls real Rust backends instead of mock data.
- [ ] TODO: Update Rust diff-cli to output `FileDiffMetadata`-compatible JSON directly, eliminating the TypeScript `parseDiffFromFile` converter layer.

## P4: Merge, Image, And Advanced Diff Modes

- [ ] Implement `dt merge` behavior and exit status.
- [ ] Add 3-way merge model and conflict-resolution tests.
- [ ] Add image metadata and pixel/perceptual diff prototype.
- [ ] Add JSON/YAML/TOML semantic diff mode.
- [ ] Add package-lock/dependency diff summaries.

## P5: Product USPs

- [ ] Prototype diff triage ranking for risky files.
- [ ] Prototype intent clustering for changed files.
- [ ] Prototype agent-review workspace for comparing outputs from multiple
      coding agents.
- [ ] Add evidence-linked summaries only after deterministic source anchors are
      available.

## Documentation Debt

- [ ] Add an ADR when choosing the final UI renderer strategy.
- [ ] Add an ADR when choosing the final diff algorithm strategy.
- [ ] Add Windows setup notes after validating Tauri and CLI on Windows.
- [ ] Add release packaging docs once bundle assets and signing are real.
