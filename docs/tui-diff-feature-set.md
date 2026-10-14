# TUI Diff Feature Set

This note captures the online feature scan behind the first `dt tui` scaffold.
The current implementation is intentionally non-interactive so it can be tested
without a terminal session; it establishes the output model that an interactive
terminal backend can later drive.

## Prior Art

- `delta` anchors the terminal diff baseline: syntax highlighting, word-level
  highlights, side-by-side view with wrapping, line numbering, large-diff
  navigation, merge-conflict display, blame display, grep highlighting,
  copyable code, hyperlinks, and stylable headers.
- `difftastic` pushes the model beyond line diffs: syntax-tree comparison,
  formatting-noise reduction, wrapper/delimiter matching, real line numbers,
  Git integration, and line-oriented fallback for unsupported or very large
  inputs.
- `diff-so-fancy` focuses on readable terminal output: simplified hunk headers,
  configurable empty-line markers, optional stripping of leading `+`/`-`
  symbols, rulers, short headers, recursive diff support, and Git pager/filter
  integration.
- `vimdiff` is the strongest interactive reference: synchronized windows,
  diff folds, jumping between changes, and commands to obtain or put changes
  between panes.

## Scaffolded Now

- `diff-tui` crate with a deterministic renderer over `diff_core::DiffResult`.
- `dt tui LEFT RIGHT` command that shares the main text diff flags:
  `--ignore-whitespace`, `--ignore-whitespace-and-case`, `--context`,
  `--algorithm`, `--deadline`, `--language`, and `--syntax-depth`.
- Split-pane output with file path header, hunk headers, old/new line numbers,
  additions, deletions, context rows, warnings, truncation note, and keymap
  footer.
- Preview controls for CI and narrow terminals: `--width`, `--max-hunks`, and
  `--max-lines-per-hunk`.

## Planned Interactive Slice

- Ratatui/crossterm backend with alternate-screen lifecycle and panic cleanup.
- Scroll-synchronized panes with `n`/`p` hunk navigation and `g`/`G` document
  jumps.
- `/` search over visible text and file paths, with next/previous match.
- Toggle between split and unified views.
- Context folding and expansion around unchanged lines.
- Syntax/theme layer that can use the existing syntax-aware diff metadata when
  available.
- Copy/apply actions for selected hunks, kept behind explicit confirmation.

## Sources

- delta features: https://dandavison.github.io/delta/features.html
- Difftastic overview/manual: https://difftastic.wilfred.me.uk/ and
  https://difftastic.wilfred.me.uk/introduction.html
- diff-so-fancy README: https://github.com/so-fancy/diff-so-fancy
- Vim diff help: https://vimhelp.org/diff.txt.html
