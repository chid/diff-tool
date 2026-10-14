# TODO

Priorities are ordered for efficient implementation. Do these before adding
large new product surfaces.

## P0: Make The Scaffold Production-Shaped

- [x] Add `npm run test:e2e` using the Markdown process in
      `docs/e2e-integration-test-process.md`. (CLI e2e: 6/6 pass; Playwright browser e2e: pass.)
- [x] Make context configurable via `--context` CLI flag (context-aware hunk
      generation already uses the `context` value in `DiffRequest`).
- [x] Add intraline spans for adjacent delete/add pairs.
- [x] Normalize or explicitly model line endings so CRLF/LF behavior is
      intentional and documented.
- [x] Add binary detection before lossy UTF-8 conversion (null byte + >10% non-text byte ratio in 8KB probe).
- [x] Add invalid UTF-8 fixtures and tests (`fixtures/cases/invalid-utf8-left/`, `invalid-utf8-both/`).
- [x] Clean up office-adapter compiler warnings, document as experimental for MVP (see `docs/office-diff-feature.md`).
- [x] Make `dt diff` exit non-zero for unreadable inputs and invalid arguments
      with documented exit codes.
- [x] Add stable CLI output snapshots for human and JSON modes (verified via e2e and insta snapshots).

## P1: Fix The Performance Hot Path

- [x] Investigate `many-small-lines-churn` (~64s Myers) and implement Histogram
      algorithm via `imara-diff` (33s — 1.94× speedup). Myers, Patience, and
      Histogram all available via `--algorithm` CLI flag. Perf suite now benchmarks
      both Myers and Histogram for every case.
- [ ] Add a perf budget file with initial thresholds for quick cases.
- [ ] Fail `npm run perf:quick` only after thresholds are intentionally agreed.
- [ ] Add peak memory measurement for macOS first, then portable support.

## P2: Folder And Git Backend

- [x] Implement true left/right folder comparison instead of single-root scan (`git_adapter::compare_folders`, chunked binary comparison).
- [x] Add added/deleted/modified/unchanged status classification (`FolderDiff`, `DirEntryDiff`).
- [x] Respect `.gitignore`, `.ignore`, and explicit include/exclude filters (`ignore::WalkBuilder` with `.require_git(false)`).
- [ ] Add rename detection behind an explicit option.
- [ ] Add Git repository discovery and working-tree status.
- [ ] Add commit/branch comparison API.
- [ ] Add conflict discovery for unresolved merge markers and Git index stages.

## P3: Desktop UI

- [x] Replace fallback diff pane with a wrapped `@pierre/diffs` spike.
- [x] Replace fallback tree pane with a wrapped `@pierre/trees` spike.
- [x] Preferences system with JSON-driven registry, SettingsContext, SettingsPanel (25 settings, 45 tests).
- [x] Settings panel UI with category icons, modified badges, focus trap, animations.
- [x] TODO: Wire live Tauri IPC so the desktop app calls real Rust backends instead of mock data.
      See [docs/tauri-ipc-wiring-plan.md](docs/tauri-ipc-wiring-plan.md) for the phased plan.
- [x] Phase 1: Add `read_file_text` Tauri command, wire diff content with loading/error states.
- [x] Phase 2: Wire `scan_tree` for live file tree entries.
- [x] Phase 3: File open dialogs (`openFolderDialog`), drag and drop (`tauri://drag-drop`), command-line args (`getLaunchArgs`).
- [x] Phase 4: Decision on Rust-format vs. pierre-format diff output (Rust DiffResult standardized).
- [x] Phase 5: Two-folder comparison integration across desktop UI, Tauri IPC (`compare_folders`), and CLI (`dt dir <left> <right>`).
- [x] TODO: Make Rust `DiffResult` the MVP desktop rendering contract for Tauri two-file comparisons and single-file previews.
  The plain Vite demo still uses `@pierre/diffs` sample data for browser-only examples; the desktop path no longer falls back to hardcoded README/TODO defaults.

## P4: Merge, Image, And Advanced Diff Modes

- [ ] Implement `dt merge` behavior and exit status.
- [ ] Add 3-way merge model and conflict-resolution tests.
- [ ] Add image metadata and pixel/perceptual diff prototype.
- [ ] Add JSON/YAML/TOML semantic diff mode.
- [ ] Add package-lock/dependency diff summaries.

## P5: Product USPs

- [ ] Add a local MCP server interface for agent-facing file and folder diff
      tools. Start with stdio transport and the scoped MVP in
      [docs/mcp-interface-plan.md](docs/mcp-interface-plan.md).
- [x] Research and implement tree-sitter syntax-aware AST diffing.
      — Phases 0-5 complete (deps, parser, matching, hunk production, CLI flags,
        fixtures, perf benchmarks).
      — See [docs/tree-sitter-diffing.html](docs/tree-sitter-diffing.html).
      — Branch: `feat/tree-sitter-syntax-diffing`.
- [ ] Prototype diff triage ranking for risky files.
- [ ] Prototype intent clustering for changed files.
- [ ] Prototype agent-review workspace for comparing outputs from multiple
      coding agents.
- [ ] Add agent live-monitor view: watch agents (Claude Code, Copilot, etc.)
      as they produce diffs, see each file change appear inline in real-time,
      and review/halt/approve per-hunk before the agent moves on.
- [ ] Research: does using tree-sitter for syntax-aware AST diffing improve
      inline detection or perf vs. line-based only? If yes, gate by file
      type and add a `--algorithm syntax` flag.
      — See [docs/tree-sitter-diffing.html](docs/tree-sitter-diffing.html) for
      the full technical plan.
- [ ] Add evidence-linked summaries only after deterministic source anchors are
      available.

## P6: AI Features (BYOK / OpenAI-Anthropic Compat)

The architecture is defined in [ADR 0002](docs/decisions/0002-ai-features-byok.md).
The full ideation catalog is in [docs/ai-features.md](docs/ai-features.md).

### Phase 1: Backend Foundation

- [ ] Create `crates/ai-adapter/` with `LlmBackend` trait.
- [ ] Implement `OpenAiBackend` and `AnthropicBackend`.
- [ ] Add `keyring` crate for OS keychain API key storage.
- [ ] Add cache implementation with hash-keyed file storage.
- [ ] Add settings UI for AI provider, model, and key.

### Phase 2: AI Change Summary (P0)

- [ ] Add `ai_summarize_diff` Tauri command with streaming.
- [ ] Add `<AiSummaryPanel>` React component.
- [ ] Add `dt explain` CLI command.

### Phase 3: AI-Assisted Merge Conflict Resolution (P0)

- [ ] Ensure `diff-core` parses conflict markers into structured hunks.
- [ ] Add `ai_resolve_conflict` Tauri command.
- [ ] Add merge resolution UI with accept/reject per hunk.
- [ ] Add build/lint validation after AI resolution.

### Phase 4: Risk Scoring, Intent Clustering, Triage Ranking (P1)

- [ ] Add risk scoring prompt and Tauri command.
- [ ] Add intent clustering prompt and Tauri command.
- [ ] Add triage ranking combining deterministic + AI signals.
- [ ] Add risk badges to file tree, cluster view toggle.

### Phase 5: Explain Hunk, Review Questions, Commit Messages (P2)

- [ ] Add explain-hunk context menu in diff pane.
- [ ] Add review-questions accordion below diff summary.
- [ ] Add `dt commit-msg` CLI command.

### Phase 6: Agent-Review Workspace (P2)

- [ ] Add diff-of-diffs computation in Rust.
- [ ] Add 3-panel agent comparison view.
- [ ] Add divergence explanation prompt.

## Documentation Debt

- [x] Add an ADR when choosing the AI feature strategy (BYOK / API compat).
- [ ] Add an ADR when choosing the final UI renderer strategy.
- [ ] Add Windows setup notes after validating Tauri and CLI on Windows.
- [x] Tauri bundle config: icons, capabilities, macOS preview build (`DiffTool.app`, `DiffTool_0.1.0_aarch64.dmg`).
- [ ] Add release packaging docs once bundle assets and signing are real.
