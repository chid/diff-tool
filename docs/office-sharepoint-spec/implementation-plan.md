# Office & SharePoint Implementation Plan

## Phased Approach

The implementation is broken into six phases. Phases 1–3 cover offline Office
diff and can proceed independently. Phases 4–6 add SharePoint connectivity
and depend on the Office adapter being complete.

## Phase 1: Office Adapter Foundation

**Goal:** Format detection, safe zip reading, and structured text extraction
for all three formats.

**Depends on:** nothing (new crate).

### Tasks

- [ ] Create `crates/office-adapter/` with Cargo.toml and module skeleton
- [ ] Implement `format::detect_format()` — extension + zip signature check
- [ ] Implement `unpack::read_zip_safe()` — zip reading with size/part limits
- [ ] Implement Word XML extraction: paragraphs, headings, tables, comments
- [ ] Implement PowerPoint XML extraction: slides, titles, notes, text boxes
- [ ] Implement Excel XML extraction: sheets, cells, formulas, shared strings
- [ ] Add `OfficeError` type with clear error variants
- [ ] Add unit tests for format detection (valid, corrupt, wrong extension)
- [ ] Add unit tests for zip safety guards (oversized, too many parts)
- [ ] Add integration tests with hand-authored minimal `.docx`, `.pptx`, `.xlsx`

### Exit Criteria

- `cargo test -p office-adapter` passes
- Can extract structured text from all three formats
- Malformed files produce clean errors, not panics

## Phase 2: Office Comparison Engine

**Goal:** Compare extracted documents and produce `OfficeDiffResult`.

**Depends on:** Phase 1.

### Tasks

- [ ] Implement section matching by ID (paragraph index, slide ID, sheet name)
- [ ] Implement text comparison using `diff-core` for extracted text blobs
- [ ] Implement structural comparison (added/deleted/reordered sections)
- [ ] Implement Excel-specific: formula comparison, named range comparison
- [ ] Implement image metadata comparison (presence, dimensions, hash)
- [ ] Add `OfficeDiffResult` model with all types from the spec
- [ ] Add fixture-driven tests: `fixtures/office/word-basic/`,
      `fixtures/office/powerpoint-basic/`, `fixtures/office/excel-basic/`
- [ ] Add test: deterministic output (same inputs → identical JSON)
- [ ] Add test: expected section/change counts match fixture metadata
- [ ] Add performance test: large Excel workbook with range filter

### Exit Criteria

- `cargo test -p office-adapter` passes with fixture-based comparison tests
- Office diff output matches expected JSON for all basic fixtures
- Large Excel workbook comparison completes in under 5 seconds with range filter

## Phase 3: CLI and Desktop Integration

**Goal:** `dt office` CLI command and desktop Office compare route.

**Depends on:** Phase 2.

### Tasks

- [ ] Add `dt office` subcommand to `diff-cli` with `--json` and human modes
- [ ] Add `--sheet` and `--range` flags for Excel
- [ ] Add human-readable output format (compact summary with section details)
- [ ] Add CLI snapshot tests for JSON and human output
- [ ] Add `compare_office_files` Tauri command
- [ ] Add `<OfficeDiffPane>` React component
  - Word: heading sidebar + paragraph change cards
  - PowerPoint: slide list + per-slide change badges
  - Excel: sheet picker + grid-like change summary
- [ ] Add Office file type to the file open dialog
- [ ] Add Office-specific settings to settings registry
- [ ] Add CLI integration tests with example files
- [ ] Add desktop UI smoke test for Office diff rendering

### Exit Criteria

- `dt office examples/word/left.docx examples/word/right.docx --json` works
- Desktop app renders Office diffs with format-specific navigation
- Snapshot tests pass for CLI output

## Phase 4: SharePoint Adapter Foundation

**Goal:** OAuth authentication, Graph API client, and site/library browsing.

**Depends on:** Phase 2 (needs `office-adapter` for diff computation).

### Tasks

- [ ] Create `crates/sharepoint-adapter/` with Cargo.toml and module skeleton
- [ ] Register app in Microsoft Entra ID for OAuth (dev tenant)
- [ ] Implement OAuth 2.0 PKCE flow with system browser launch
- [ ] Implement token storage and refresh using `keyring` crate
- [ ] Implement Microsoft Graph API HTTP client with auth header injection
- [ ] Implement `sharepoint_list_sites()` — site discovery
- [ ] Implement `sharepoint_list_drives()` — document library listing
- [ ] Implement `sharepoint_list_items()` — file/folder browsing
- [ ] Implement `sharepoint_search()` — search within libraries
- [ ] Add Tauri commands for all browsing functions
- [ ] Add `<SharePointBrowser>` React component
- [ ] Add "Connect SharePoint" flow in settings
- [ ] Add unit tests with mock Graph API responses
- [ ] Add integration test with real tenant (opt-in, gated by env var)

### Exit Criteria

- User can connect SharePoint account from settings
- User can browse sites, libraries, and folders in the SharePoint browser
- Token refresh works transparently
- `dt sharepoint sites` and `dt sharepoint ls` CLI commands work

## Phase 5: SharePoint Version Diff

**Goal:** Version history, version-to-version diff, and local-vs-server diff.

**Depends on:** Phase 4.

### Tasks

- [ ] Implement `sharepoint_list_versions()` — version history
- [ ] Implement file download for specific versions
- [ ] Implement local file cache with hash-keyed storage and TTL
- [ ] Implement `sharepoint_diff_versions()` — diff two server versions
- [ ] Implement `sharepoint_diff_local_vs_server()` — local vs server diff
- [ ] Add cache eviction (LRU, size limit)
- [ ] Add "Clear SharePoint cache" button in settings
- [ ] Add Tauri commands for all version and diff functions
- [ ] Add version picker UI in SharePoint browser
- [ ] Add diff result rendering (reuse `<OfficeDiffPane>` from Phase 3)
- [ ] Add CLI: `dt sharepoint diff`, `dt sharepoint diff-local`
- [ ] Add integration tests with fixture SharePoint files

### Exit Criteria

- User can view version history for a SharePoint file
- User can diff any two versions and see structured Office diff
- User can diff a local `.docx` against a SharePoint version
- Cache stores and retrieves files correctly, evicts when full

## Phase 6: Collaboration Features

**Goal:** Review queue, conflict detection, AI enrichment.

**Depends on:** Phase 5 (and Phase 3 of AI features for AI enrichment).

### Tasks

- [ ] Implement "Shared With Me" / recent collaborator changes view
- [ ] Implement check-out status detection and warning
- [ ] Implement local-vs-server conflict detection (3-way merge)
- [ ] Add "Resolve with AI" for Office merge conflicts (requires `ai-adapter`)
- [ ] Add AI summary for SharePoint diffs ("What did Alex change?")
- [ ] Add risk scoring for Office document changes
- [ ] Add SharePoint-specific settings (auto-refresh, cache TTL)
- [ ] Performance optimization: parallel downloads for multi-file diffs
- [ ] Add usage documentation and onboarding tooltips

### Exit Criteria

- User can see a review queue of recent collaborator changes
- Conflict detection works for local-vs-server scenarios
- AI features work with Office diffs (if AI adapter is configured)

## Timeline Estimates

These are rough estimates for a single developer. They assume the existing
Rust core and desktop shell are stable.

| Phase | Effort | Cumulative |
|-------|--------|------------|
| 1. Office adapter foundation | 3–4 weeks | 3–4 weeks |
| 2. Office comparison engine | 2–3 weeks | 5–7 weeks |
| 3. CLI and desktop integration | 2–3 weeks | 7–10 weeks |
| 4. SharePoint adapter foundation | 3–4 weeks | 10–14 weeks |
| 5. SharePoint version diff | 2–3 weeks | 12–17 weeks |
| 6. Collaboration features | 2–3 weeks | 14–20 weeks |

**Total: ~14–20 weeks** for full Office + SharePoint integration.

### Parallelization Opportunities

- Phases 1–3 (Office diff) can proceed in parallel with current P2–P4 work
  (folder diff, merge, images).
- Phase 4 can begin once Phase 2 is complete (sharepoint-adapter needs
  office-adapter for diff, but not the CLI/UI work in Phase 3).
- AI enrichment (Phase 6 tasks) can be developed in parallel with Phases 4–5
  once the AI adapter is available.

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| XML parsing complexity for Office formats exceeds estimates | High | Spike extraction for each format early; consider `roxmltree` vs `quick-xml` tradeoffs |
| Microsoft Graph API rate limits block real-world usage | Medium | Aggressive caching, request batching, exponential backoff |
| OAuth flow UX friction (system browser, redirect) | Medium | Test on macOS + Windows early; custom protocol as fallback |
| Large Excel workbooks cause memory issues | Medium | Stream cells, apply range filters early, paginate results |
| SharePoint on-prem not supported (V1 limitation) | Low | Document clearly; address in V2 if demand exists |
| Office format changes between Office versions | Low | Test against Office 2019, 2021, and Microsoft 365 generated files |

## Dependencies on Other Workstreams

| Workstream | Needed By | Status |
|------------|-----------|--------|
| `diff-core` text diff | Phase 2 | ✅ Implemented |
| `ai-adapter` crate | Phase 6 | 🔲 Planned (P6 in TODO.md) |
| Tauri IPC wiring (Phase 1) | Phase 3 | ✅ Implemented |
| Desktop settings system | Phase 3, 4 | ✅ Implemented |
| `git-adapter` folder diff | Phase 6 (debatable) | 🔲 Planned (P2 in TODO.md) |
