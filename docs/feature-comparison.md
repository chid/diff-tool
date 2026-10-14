# Feature Comparison

Last updated: 2026-05-29

## Legend

| Icon | Meaning |
|------|---------|
| ✅ | Done / shipping |
| 🔶 | Partial or experimental |
| ⬜ | Planned, not started |
| ❌ | Not planned / out of scope |
| — | Not applicable to that tool |

---

## 1. Core Text Diff

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Araxis Merge | P4Merge | Git diff | diffoscope |
|---------|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| Line-level diff | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Multi-hunk output | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Intraline (word/char) spans | ✅ | ✅ | ✅ | ✅ | ❌ | `--word-diff` | ✅ |
| Context lines control | ✅ | ✅ | ✅ | ✅ | ✅ | `-U<n>` | ✅ |
| JSON output | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Diff algorithms (Myers / Patience / Histogram) | ✅ (3) | ? | ? | ? | ? | ✅ (3) | — |
| Whitespace ignore (`-w`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Case ignore | ✅ | ✅ | ✅ | ✅ | ❌ | `-i` | ❌ |
| Line-ending normalization (CRLF/LF) | ✅ | ✅ | ✅ | ✅ | ? | `--ignore-cr-at-eol` | ✅ |
| Binary detection | ✅ | ✅ | ✅ | ✅ | ✅ | `--binary` | ✅ |
| Truncation / max-bytes guard | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Progress events (for large files) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Inline CSS-styled HTML output | 🔶 | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ |
| Stable snapshot tests | ✅ | ? | ? | ? | ? | ✅ | ? |

---

## 2. CLI (`dt`)

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Araxis | Git diff | diffoscope |
|---------|-----------|:---:|:---:|:---:|:---:|:---:|
| `diff` two files | ✅ | `ksdiff` | `bcomp` | `araxisgitdiff` | ✅ | ✅ |
| `dir` folder scan | ✅ | `ksdiff --dir` | ✅ | ✅ | `--diff-filter` | ✅ |
| JSON machine output | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Human-readable summary | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Documented exit codes | ✅ | ? | ✅ | ? | ✅ | ✅ |
| `merge` command | ⬜ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Persistent config (`dt config`) | ✅ | ✅ (GUI) | ✅ (GUI) | ✅ (GUI) | `git config` | ❌ |
| `dt config get/set/reset` | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Config via XDG path / `~/.config` | ✅ | `~/Library` | `~/.beyondcompare` | Registry | `~/.gitconfig` | — |
| `git difftool` / `git mergetool` integration | ⬜ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Pipe / stdin support | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## 3. Folder & Git Comparison

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Sublime Merge | Tower | Fork | GitKraken |
|---------|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| Left/right folder diff | ⬜ | ✅ | ✅ | ✅ (git) | ✅ (git) | ✅ (git) | ✅ (git) |
| Added / deleted / modified / unchanged classification | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| `.gitignore` / `.ignore` support | ⬜ | ? | ✅ | ✅ | ✅ | ✅ | ✅ |
| Rename detection | ⬜ | ? | ✅ | ✅ | ✅ | ✅ | ✅ |
| Git working tree status | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Commit / branch comparison | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Unresolved merge marker discovery | ⬜ | ? | ? | ✅ | ✅ | ✅ | ✅ |
| Git index stage inspection | ⬜ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |

---

## 4. Merge & Conflict Resolution

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Araxis | P4Merge | Sublime Merge |
|---------|-----------|:---:|:---:|:---:|:---:|:---:|
| 2-way file merge | ⬜ | ✅ | ✅ | ✅ | ✅ | — |
| 3-way merge (ours / theirs / base) | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Inline conflict resolution UI | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Per-hunk accept / reject | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Automatic resolution for non-conflicting hunks | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ |
| AI-assisted conflict resolution | ⬜ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Post-merge build / lint validation | ⬜ (AI only) | ❌ | ❌ | ❌ | ❌ | ❌ |

---

## 5. Office Document Diff

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Araxis | diffoscope |
|---------|-----------|:---:|:---:|:---:|:---:|
| `.docx` (Word) comparison | 🔶 (crate, not UI) | ✅ | ✅ | ✅ | ✅ |
| `.pptx` (PowerPoint) comparison | 🔶 (crate, not UI) | ❌ | ✅ | ✅ | ✅ |
| `.xlsx` (Excel) comparison | 🔶 (crate, not UI) | ❌ | ✅ | ✅ | ✅ |
| No MS Office dependency | ✅ | ✅ | ✅ | ❌ (Win only) | ✅ |
| Cross-platform Office diff | ✅ | macOS only | Win/Linux/macOS | Windows only | ✅ |
| Format detection & safe ZIP reading | ✅ | ✅ | ✅ | ✅ | ✅ |
| Paragraph text, headings, tables | 🔶 | ✅ | ✅ | ✅ | ✅ |
| Slide reordering / title / notes | 🔶 | ❌ | ? | ? | ✅ |
| Cell values, formulas, sheets | 🔶 | ❌ | ✅ | ✅ | ✅ |
| Office-specific CLI (`dt office`) | ⬜ | — | — | — | — |

---

## 6. Desktop UI

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Sublime Merge | Tower | Fork |
|---------|-----------|:---:|:---:|:---:|:---:|:---:|
| Split diff view | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Unified diff view | ⬜ | ✅ | ✅ | ❌ | ❌ | ❌ |
| Syntax highlighting | ✅ (Shiki) | ✅ | ❌ | ✅ | ✅ | ✅ |
| File tree with Git status badges | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Drag-and-drop file open | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Native file open dialog | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Progress bar for long operations | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Settings panel (25 settings, 5 categories) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Dark mode | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Keyboard shortcuts | ⬜ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Cross-platform (macOS, Windows, Linux) | ✅ | ❌ (macOS only) | ✅ | ❌ (Windows only) | ✅ | ✅ |
| `@pierre/diffs` wrapped rendering | ✅ | — | — | — | — | — |
| `@pierre/trees` wrapped file tree | ✅ | — | — | — | — | — |

---

## 7. Image & Binary Diff

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Araxis | P4Merge | diffoscope |
|---------|-----------|:---:|:---:|:---:|:---:|:---:|
| Image metadata comparison | ⬜ | ✅ | ✅ | ✅ | ❌ | ✅ |
| Pixel-level image diff | ⬜ | ✅ | ✅ | ✅ | ❌ | ❌ |
| Perceptual image diff | ⬜ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Binary hex diff | ⬜ | ❌ | ✅ | ✅ | ❌ | ✅ |
| Archive comparison (zip, tar, etc.) | ⬜ | ❌ | ✅ | ✅ | ❌ | ✅ |

---

## 8. Structured Data Diff

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Araxis | diffoscope |
|---------|-----------|:---:|:---:|:---:|:---:|
| JSON semantic diff | ⬜ | ❌ | ✅ (table) | ❌ | ✅ |
| YAML semantic diff | ⬜ | ❌ | ❌ | ❌ | ✅ |
| TOML semantic diff | ⬜ | ❌ | ❌ | ❌ | ❌ |
| package-lock / dependency diff | ⬜ | ❌ | ❌ | ❌ | ❌ |
| CSV / TSV comparison | ⬜ | ❌ | ✅ | ❌ | ✅ |

---

## 9. AI Features (BYOK)

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Sublime Merge | CodeRabbit | SemanticDiff | GitHub Copilot |
|---------|-----------|:---:|:---:|:---:|:---:|:---:|:---:|
| AI change summary | ⬜ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| AI merge conflict resolution | ⬜ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| AI risk / impact scoring | ⬜ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| AI review question generation | ⬜ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| AI commit message generation | ⬜ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| AI intent clustering | ⬜ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| AI agent-review workspace | ⬜ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| AI explain hunk / line | ⬜ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| BYOK (user's own API key) | 🔶 (planned) | — | — | — | ❌ (SaaS) | ❌ (SaaS) | ❌ (SaaS) |
| Local-first (works offline without AI) | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| OS keychain API key storage | ⬜ | — | — | — | — | — | — |
| Streaming AI responses | ⬜ | — | — | — | ✅ | ? | ✅ |
| Result caching (hash-keyed, 7-day TTL) | ⬜ | — | — | — | ? | ? | ? |

---

## 10. Performance & Safety

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | P4Merge | Git diff |
|---------|-----------|:---:|:---:|:---:|:---:|
| Histogram algorithm (33s vs 64s Myers on pathological input) | ✅ | ? | ? | ? | ✅ |
| Synthetic performance benchmark suite | ✅ | ❌ | ❌ | ❌ | ❌ |
| Peak memory measurement | ⬜ | ❌ | ❌ | ❌ | ❌ |
| Perf budget / regression file | ⬜ | ❌ | ❌ | ❌ | ❌ |
| Max-bytes truncation guard | ✅ | ❌ | ✅ | ❌ | `--max-depth` |
| Safe ZIP decompression (Office) | ✅ | — | ? | — | — |
| Lossy UTF-8 fallback with detection | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## 11. Testing & Quality

| Feature | Diff-Tool | Kaleidoscope | Beyond Compare | Git diff |
|---------|-----------|:---:|:---:|:---:|
| Rust unit tests (~86) | ✅ | ? | ? | ✅ |
| E2E CLI tests (14 files) | ✅ | ? | ? | ✅ |
| Playwright browser tests | ✅ | ? | ? | — |
| Tauri integration tests (14) | ✅ | — | — | — |
| Office adapter tests (16) | ✅ | — | ? | — |
| Fixture-driven diff tests | ✅ | ? | ? | ✅ |
| Snapshot tests (insta) | ✅ | ? | ? | ❌ |
| Performance regression runner | ✅ | ❌ | ❌ | ❌ |
| E2E test instrumentation (`target/e2e/`) | ✅ | ❌ | ❌ | ❌ |
| Vitest React component tests (45) | ✅ | ? | ? | — |

---

## 12. Unique Differentiators (This Project)

Features this tool has or plans that **no** surveyed competitor offers:

| Feature | Status |
|---------|--------|
| Progress events via Tauri event bus (per-stage, with bytes) | ✅ |
| BYOK AI that works fully offline without API key | ⬜ |
| AI intent clustering for folder diffs | ⬜ |
| AI agent-review workspace (compare outputs from multiple coding agents) | ⬜ |
| Evidence-linked AI summaries with line-number citations | ⬜ |
| Cross-platform Office diff with no MS Office dependency | 🔶 |
| Machine-readable JSON output for all modes | ✅ |
| Config from CLI (`dt config get/set/reset`) with XDG paths | ✅ |
| Synthetic performance benchmark suite with regression detection | ✅ |
| `@pierre/diffs` and `@pierre/trees` wrapped rendering | ✅ |

---

## 13. Implementation Progress Summary

### Done ✅
- Rust workspace with `diff-core`, `diff-cli`, `git-adapter`, `office-adapter`
- 3 diff algorithms (Myers, Patience, Histogram via `imara-diff`)
- Intraline spans, context control, whitespace/case ignore, CRLF normalization
- Binary detection, UTF-8 lossy fallback, max-bytes truncation
- Stable JSON output, human output, documented exit codes
- CLI config system (`dt config get/set/reset`)
- Progress events on Tauri event bus
- Tauri 2 + React/Vite desktop shell
- `@pierre/diffs` with Shiki syntax highlighting
- `@pierre/trees` with Git status badges
- Settings system (25 settings, 5 categories, 45 tests)
- File open dialogs, drag-and-drop, launch args
- 14 e2e test files, 86 Rust unit tests, 45 vitest tests
- Performance benchmark suite (synthetic fixtures)

### Partial 🔶
- `office-adapter` crate implements format detection, extraction, comparison for `.docx`/`.pptx`/`.xlsx` but not exposed in CLI or UI
- HTML output (CSS-styled exists in Pierre layer, not standalone)

### Next Up ⬜ (P1–P2)
- Left/right folder comparison with status classification
- `.gitignore` / ignore rules
- Git repository discovery, working tree status, commit/branch comparison
- Perf budget file + peak memory measurement
- `dt merge` with 3-way conflict resolution
- Rust-format JSON directly to `FileDiffMetadata` (eliminate TS converter)
- Keyboard shortcuts
- Unified diff view
- AI backend foundation (`crates/ai-adapter/`)

### Later ⬜ (P3–P6)
- Image metadata + pixel/perceptual diff
- JSON/YAML/TOML semantic diff
- Office CLI + desktop UI
- All AI features (summary, merge resolution, risk scoring, clustering, etc.)
- Agent-review workspace
- `dt commit-msg`, `dt explain`, `dt dir --risk/cluster/rank`

---

## Competing Tools Surveyed

| Tool | Type | Platforms | Price | AI |
|------|------|-----------|-------|:--:|
| [Kaleidoscope](https://kaleidoscope.app) | Desktop diff/merge | macOS | $199 | ❌ |
| [Beyond Compare](https://scootersoftware.com) | Desktop diff/merge | Win/macOS/Linux | $35–$70 | ❌ |
| [Araxis Merge](https://www.araxis.com/merge/) | Desktop diff/merge | Win/macOS | $129–$269 | ❌ |
| [P4Merge](https://www.perforce.com/products/helix-core-apps/merge-diff-tool-p4merge) | Desktop diff/merge | Win/macOS/Linux | Free | ❌ |
| [Sublime Merge](https://www.sublimemerge.com) | Git client + diff | Win/macOS/Linux | $99 | ❌ |
| [Tower](https://www.git-tower.com) | Git client | Win/macOS | $69/yr | ❌ |
| [Fork](https://git-fork.com) | Git client | Win/macOS | $60 | ❌ |
| [GitKraken](https://www.gitkraken.com) | Git client | Win/macOS/Linux | $60/yr | ❌ |
| [Git diff](https://git-scm.com/docs/git-diff) | CLI | All | Free | ❌ |
| [diffoscope](https://diffoscope.org) | CLI deep diff | Linux/macOS | Free | ❌ |
| [CodeRabbit](https://coderabbit.ai) | AI code review | Web / SaaS | $12–$24/mo | ✅ |
| [SemanticDiff](https://semanticdiff.com) | Diff + AI | Web / SaaS | Free–$15/mo | ✅ |
| [GitHub Copilot](https://github.com/features/copilot) | AI coding assistant | IDE / Web | $10–$39/mo | ✅ |
