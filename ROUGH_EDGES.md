# Syntax-Aware Diffing — Rough Edges

`--algorithm syntax` uses tree-sitter AST parsing for smarter diffs.
This feature is **experimental**. Known limitations below.

## Missing

- **Intraline spans** — character-level highlighting within changed lines
  is skipped for Syntax diffs. You get line-level diff only.
- **Context counting** — unchanged context lines are not counted in stats.
  `dt diff` shows the right hunks but `stats.unchanged` is always 0 for
  non-identical files.
- **`--context` ignored** — the context parameter has no effect on AST output.
  Hunk grouping still uses outer context, but context lines aren't counted.

## Known Issues

- **Memory** — each file is fully parsed into a CST before diffing.
  Very large files (>10K lines) skip syntax and fall back to line-based.
- **Performance** — ~25-40% slower than Myers/Histogram for typical files.
- **Language support** — only Rust, TypeScript, and Python.
  Unsupported languages fall back silently to line-based.
- **Node filtering** — Statement depth is the default. `function` and `token`
  depths may produce unexpected results with certain grammars.
- **`source_file` excluded** — the root wrapper node is excluded from
  matching, which is correct for diffs but differs from standard GumTree.
- **No rename tracking in output** — AST detects moves/renames internally
  but hunk output still shows them as add+delete pairs at the line level.

## Test Coverage

- **14 syntax fixtures** — Rust (8), Python (3), TypeScript (3)
- **Perf harness** — 2 Rust scenarios in `npm run perf:quick`
- **Missing** — Large file benchmarks, mixed-language scenarios,
  edge cases (empty files, single-line files, deeply nested code)

## Compatibility

- **Desktop UI** — Syntax diff output feeds through the same `Hunk`/`LineChange`
  types. Hunks do not carry `ChangeClass` classification yet (no semantic badges).
- **JSON output** — Stats work but `truncated` only tracks file-size truncation,
  not AST-node truncation.

## Future Work

- Wire `ChangeClass` into `Hunk` for per-hunk semantic badges in UI
- Add syntax fixtures for all three languages at scale
- Replace `ast_diff_to_hunks_full` with proper context-aware line emission
- Add intraline span computation from AST node boundaries
