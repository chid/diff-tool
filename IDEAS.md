# IDEAS — Common Issues With Diff Tools

> Sourced from GitHub issues, PRs, Reddit, Hacker News, and Stack Overflow across
> Meld, KDiff3, Difftastic, Delta, WinMerge, diff-so-fancy, Beyond Compare, and others.
>
> Each section is ordered roughly by how frequently the issue appears in the wild.
> Items marked with 🎯 are particularly relevant to our diff-tool MVP.

---

## 1. Algorithm Quality & Diff Output

The single biggest category of complaints. Users care deeply about diff _correctness_.

| Issue | Details | Seen In |
|---|---|---|
| 🎯 **Line-based diffs are noisy for real-world code changes** | Moving a function, renaming a variable, or running a formatter produces massive unreadable diffs. This is the #1 complaint across all tools. | All tools |
| 🎯 **Moved/renamed blocks aren't detected** | If you move a function within a file, every tool shows it as a full delete + add. No tool detects the move. | Difftastic, Meld, all line-based tools |
| 🎯 **Intraline (character-level) highlighting is wrong or missing** | Tools either don't highlight within-line changes at all, or highlight too aggressively (whole line when one char changed). | Delta issues, diff-so-fancy, Meld |
| 🎯 **Whitespace handling is all-or-nothing** | Users want fine-grained control: ignore trailing only, ignore indentation-only changes, ignore blank-line changes. Most tools offer only "ignore all" or "ignore none". | WinMerge issues, Reddit, SO |
| **Poor matching in repetitive code** | Myers/patience diffs produce "wrong" matches in JSON, YAML, HTML, or any code with repeating structure. | Difftastic #200, SO |
| **Reformatting changes swamp meaningful changes** | Running prettier/black/gofmt and then diffing produces hundreds of whitespace changes that obscure logic changes. | Reddit r/programming, blog posts |
| **No detection of semantically equivalent changes** | Reordered imports, reordered JSON keys, or equivalent code restructuring all show as changes. | HN, blog posts |
| **Fallback behaviour is jarring** | Difftastic falls back to line-based diff when parsing fails; the visual result looks completely different. | Difftastic issues |
| **Algorithm isn't configurable** | Users want to switch between Myers, patience, histogram depending on content type. | Delta issues, Difftastic discussions |

## 2. Performance

Every GUI diff tool struggles here. CLI tools fare better but lack visual features.

| Issue | Details | Seen In |
|---|---|---|
| 🎯 **Large files (5K+ lines) cause slowdowns or crashes** | Meld takes 30+ seconds. KDiff3 freezes. Difftastic can take minutes. | Meld GitLab, KDiff3 Bugzilla, Difftastic #388 |
| 🎯 **Directory comparison with many files is slow** | Scanning large repos or deep directory trees takes an unreasonable time. | Meld, WinMerge, Beyond Compare (network drives) |
| **Syntax highlighting adds significant overhead** | Structural diffing (Difftastic) and syntax-highlighted rendering (Delta) both add noticeable cost. | Difftastic issues, Delta issues |
| **Memory usage spikes / memory leaks** | Especially with multiple tabs or long sessions. | Meld GitLab, general GUI tools |
| **Startup latency** | Loading themes, syntax definitions, or initialising parsers adds visible delay. | Delta issues |

## 3. File Format Support Gaps

Almost every tool treats non-plain-text as a second-class citizen.

| Issue | Details | Seen In |
|---|---|---|
| 🎯 **Binary files show only "Binary files differ"** | Completely useless. No tool provides meaningful binary comparison by default. | Universal |
| 🎯 **Structured data diffing is line-based, not structure-aware** | JSON key reordering, XML attribute reordering, YAML anchor changes all produce noisy line-level diffs. | HN, Reddit, blog posts |
| **Image diffing is rare or primitive** | Users want side-by-side, overlay, onion-skin, or swipe comparison modes (like GitHub's image diff). Few desktop tools offer this. | Reddit, GitHub feature requests |
| **No Office document diffing** | Comparing Word, Excel, PowerPoint is a common enterprise need. Almost no tool handles it. | Reddit, SO |
| **No PDF comparison** | Users want visual or extracted-text PDF diffing. | Reddit, feature requests |
| **Jupyter notebooks produce useless diffs** | The JSON structure makes standard diffs unreadable. | Reddit r/datascience |
| **CSV/table comparison is buggy** | Column alignment and delimiter detection issues. | WinMerge issues |

## 4. UI/UX Complaints

| Issue | Details | Seen In |
|---|---|---|
| 🎯 **No dark mode or broken dark mode** | Meld's dark mode is its most-reported issue. Older tools (KDiff3) lack it entirely. | Meld GitLab, KDiff3, Reddit |
| 🎯 **Red/green defaults are unusable for colorblind users** | ~8% of males have red-green colorblindness. Few tools offer accessible alternatives. | Delta issues, accessibility discussions |
| 🎯 **Side-by-side view wastes horizontal space** | Long lines force scrolling. Users want easy switching between side-by-side and unified views. | Difftastic #221, Reddit |
| 🎯 **No minimap/overview of the full file** | Users from VS Code expect a minimap showing all changes at a glance. Standalone tools lack this. | Feature requests across tools |
| **Synchronized scrolling bugs** | Scroll sync breaks when files have significantly different lengths (many additions/deletions). | WinMerge issues, Meld issues |
| **Line wrapping breaks alignment** | Word wrap in side-by-side view misaligns corresponding lines. | Multiple tool issue trackers |
| **Navigation between changes is clunky** | Jumping between hunks with keyboard shortcuts is missing or unintuitive. | Reddit, SO |
| **Tab rendering is wrong** | Tabs display at incorrect widths, breaking column alignment. | Meld GitLab |
| **Font / text rendering issues** | Monospace rendering, ligature support, and proper tab-width handling. | Various tool issues |
| **No screen reader support** | No diff tool has meaningful accessibility for vision-impaired users. | Accessibility discussions |
| **Outdated UI appearance** | KDiff3, WinMerge, and others look dated compared to modern tools. | Reddit |

## 5. Merge-Specific Pain Points

| Issue | Details | Seen In |
|---|---|---|
| 🎯 **3-way merge UI is confusing** | Users can't tell which pane is "theirs," "mine," or "base." Even experienced devs struggle. | KDiff3 Bugzilla, Reddit r/git, SO |
| 🎯 **No undo/history during merge sessions** | Can't undo a merge resolution or review what was changed. | KDiff3 issues, Meld issues |
| **Auto-merge makes incorrect choices silently** | Produces code that compiles but behaves incorrectly. No explanation of what was auto-resolved. | Reddit, HN |
| **Conflict markers are hard to read** | Standard `<<<<<<<` / `=======` / `>>>>>>>` markers are despised, especially nested. | Reddit, blog posts |
| **Insufficient context during conflict resolution** | Tools don't show enough surrounding code to understand what's being merged. | Reddit, SO |
| **Same conflicts reappear during rebases** | Tools don't integrate with `git rerere` to remember resolutions. | Reddit r/git |
| **Difftastic has no merge support at all** | Diff-only tool; merge is out of scope. | Difftastic FAQ |

## 6. Integration Issues

| Issue | Details | Seen In |
|---|---|---|
| 🎯 **Git difftool/mergetool config is painful** | Arcane `.gitconfig` syntax. Many tools don't auto-configure. Hundreds of SO questions. | SO, Reddit |
| 🎯 **VS Code's built-in diff is "good enough"** | Standalone tools need to be _significantly_ better to justify context-switching. | Reddit, HN |
| **External diff tool won't launch or launches wrong version** | Path issues, version conflicts, process management bugs. | SO, Reddit |
| **No CI/CD integration** | Most diff tools are GUI-only. Can't use in automated pipelines. | Reddit, HN |
| **No clipboard diffing** | Users want to diff clipboard vs file, or clipboard vs clipboard without temp files. | Reddit, SO |
| **Exit code semantics vary** | Difftastic returns 0 for both "no diff" and "has diff," confusing scripts. | Difftastic issues |
| **diff-so-fancy as pager breaks git operations** | `git apply`, `git stash` fail when diff-so-fancy is `core.pager`. | diff-so-fancy #320, #351 |
| **Delta config complexity** | Many options, `.gitconfig` syntax is non-obvious. | Delta issues, Reddit |

## 7. Cross-Platform Issues

| Issue | Details | Seen In |
|---|---|---|
| 🎯 **macOS: tools look non-native or have rendering glitches** | GTK (Meld) and Qt (KDiff3) look foreign on macOS. Installation is difficult. | Meld GitLab, KDiff3 Bugzilla, SO |
| **CRLF vs LF false positives** | Cross-platform projects constantly battle line ending diffs. | SO, Git docs, multiple tools |
| **HiDPI / Retina rendering problems** | Blurry text or misaligned elements on high-DPI displays. | Meld, KDiff3 issues |
| **Windows version lags behind Linux** | Several tools prioritize Linux and let Windows/macOS builds fall behind. | Meld GitLab |
| **Path separator issues** | Backslash vs forward slash in directory comparison paths. | WinMerge issues |

## 8. Encoding & Character Handling

| Issue | Details | Seen In |
|---|---|---|
| 🎯 **Unicode / emoji cause alignment issues** | Wide characters break column alignment in side-by-side view. | Delta #556, #800+ |
| **Encoding detection failures** | Tools misidentify file encoding; garbled text results. | KDiff3 Bugzilla, WinMerge issues |
| **BOM handling inconsistencies** | Byte Order Marks treated differently across platforms and tools. | Meld GitLab |
| **Mixed encodings within a file** | UTF-8 with embedded Latin-1 or other encodings confuses all tools. | General complaint |

## 9. Most-Requested Missing Features

Roughly ordered by frequency of request.

| Feature | Description | Source |
|---|---|---|
| 🎯 **Semantic / structural diffing** | Understand code structure: function moves, variable renames, import reordering. Difftastic is the main effort; demand far exceeds supply. | HN (recurring), Reddit, blog posts |
| 🎯 **Diff filtering / focusing** | Show only specific change types: "logic changes only, not formatting." Filter by regex pattern to ignore timestamps, version numbers, etc. | Reddit, HN, WinMerge issues |
| 🎯 **Interactive staging from diff view** | Stage/unstage hunks visually (like `git add -p` but with a real UI). | Reddit, Git GUI discussions |
| 🎯 **Inline editing during diff** | Edit files directly in the diff view; diff updates live. | Reddit, multiple tool requests |
| **AI-assisted diff understanding** | Natural language summaries: "This PR renames User and adds validate()." AI-suggested merge conflict resolutions. | HN 2024-2025 |
| **History / timeline diffing** | Compare a file across multiple commits as a filmstrip, not just two versions. | Reddit, Git discussions |
| **Three-pane directory comparison** | Compare three versions of a directory tree (base, branch A, branch B). | KDiff3, Meld feature requests |
| **Diff bookmarks / annotations** | Mark hunks as "reviewed" or attach notes for code review. | Code review discussions |
| **Diff sharing** | Generate shareable HTML or URL-based diff views. | Reddit, feature requests |
| **Auto-refreshing diff views** | Diff updates automatically when underlying files change on disk. | Meld issues |
| **Branch diffing workflow** | Simple "diff branch A vs branch B" without multiple git commands. | Reddit r/git, SO |
| **Configurable algorithms per file type** | Auto-select patience for Python, histogram for C, etc. | Delta, Difftastic discussions |

---

## Opportunity Summary

The biggest gaps that a new diff tool can exploit:

1. **Performance at scale** — No GUI tool handles 10K+ line files smoothly. A Rust backend with virtual rendering could dominate here.
2. **Smart algorithm defaults** — Whitespace-aware, move-detecting, intraline-highlighting diffs out of the box. Most tools make users configure this.
3. **Real dark mode & accessibility** — First-class dark mode, colorblind-safe palettes, and keyboard navigation. Most tools bolt these on as afterthoughts.
4. **Structured data awareness** — JSON/YAML/XML diffing that understands key ordering vs value changes.
5. **Seamless Git integration** — Zero-config `git difftool` / `git mergetool` setup. Exit codes that scripts can rely on.
6. **Cross-platform native feel** — Tauri gives us native shell + web rendering without the GTK/Qt foreign-look problem.
7. **Interactive editing** — Edit in the diff view, stage hunks, see changes live.
8. **Office/binary preview** — Even basic Office diffing (via our office-adapter) is a differentiator since nobody else does it.
