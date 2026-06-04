# Tree-Sitter Syntax-Aware Diffing — Research & Implementation Plan

## Motivation

Tree-sitter gives us a concrete syntax tree (CST) with exact byte spans for every
token and node. A syntax-aware diff can:

- Detect moved/renamed functions (matched subtrees at different positions)
- Classify changes semantically: `rename`, `refactor`, `reformat`, `logic-change`
- Produce intraline highlights aligned to AST node boundaries instead of naive
  character prefix/suffix matching
- Ignore formatting-only differences (whitespace, reordered imports) while still
  flagging logic changes

## Current Pipeline

```
File paths → read + binary detect + UTF-8 validate → normalize line endings
→ normalize for ignore mode → split into lines → InternedInput
→ imara_diff::Diff::compute → iterate hunks → build LineChange vec
→ compute_intraline_spans (character-level) → build_hunks (context grouping)
→ DiffResult { FileDiff { Hunk[] } }
```

## Where Tree-Sitter Fits

```
File paths → read + binary detect + UTF-8 validate → normalize line endings
→ split into lines → [line-based imara_diff]

                     OR (if language supported + --algorithm syntax):

→ parse both files with tree-sitter → compute AST node correspondence
→ map matched/unmatched nodes to line ranges → produce LineChange/Hunk vec
→ DiffResult { FileDiff { Hunk[] } }
```

The existing `DiffAlgorithm` enum gets a new `Syntax` variant. When selected,
the pipeline branches: supported languages use the AST path, unsupported ones
fall back to `imara-diff` with a warning.

---

# Algorithm Design

## 1. Parsing Phase

For each file:
1. Detect language from file extension → load the matching tree-sitter grammar
2. Parse the file content → root `Tree` and root `Node`
3. Walk the CST to collect a flat list of `AstNode` records:
   ```rust
   struct AstNode {
       id: usize,          // pre-order traversal index
       kind: String,       // node type name (e.g. "function_definition")
       kind_id: u16,       // numeric kind from grammar
       parent: Option<usize>,
       children: Vec<usize>,
       start_byte: usize,
       end_byte: usize,
       start_line: usize,
       end_line: usize,
       text: String,       // slice of source text
       hash: u64,          // hash of normalized text (whitespace-stripped)
       named: bool,        // true for named nodes, false for anonymous tokens
   }
   ```

**Depth limit:** Stop at statement-level granularity (function defs, class defs,
if-blocks, loops, etc.) — do not descend into every expression token. This keeps
the node count manageable (~100-500 for a typical file vs 1000s of leaf tokens).

**Configurable depth:** Add a `--syntax-depth` flag (default: `statement`) with
values `statement`, `expression`, `token` for tuning.

## 2. Matching Phase — Bottom-Up Container Matching

Adapted from GumTree's simplified approach:

1. **Hash-based container matching (buckets):**
   - Hash every node's content (whitespace-stripped)
   - Group left and right nodes by their hash
   - Where a group has exactly one left and one right node → they match
   - Walk from leaves upward; if a node's children all match (same count + same
     hashes), the parent is also matched

2. **Positional heuristic** (for moved nodes):
   - For unmatched named nodes, compare their ancestry path signature
     (a string like `"program > function_definition > block"`)
   - If two nodes have the same path signature and similar content hash,
     mark as a probable move/rename

3. **Fallback:** Any remaining unmatched lines fall through to the existing
   `imara-diff` line-based algorithm inside each unmatched region (bridge
   mode).

## 3. Change Classification

Once matched/unmatched pairs are established, classify each difference:

| Classification | Detection Rule |
|---|---|
| `identical` | Node matched, no content change |
| `modified` | Node matched, content differs (subtree changed) |
| `moved` | Node matched, but parent/position changed |
| `renamed` | Node is a name-bearing identifier that changed |
| `added` | Node exists only in right file |
| `deleted` | Node exists only in left file |
| `reformatted` | Content identical after whitespace normalization |
| `type_changed` | Node kind changed (e.g. `if` → `switch`) |

## 4. Hunks & LineChange Production

- Each unmatched/moved/modified AST node maps to a set of lines in the source
- Adjacent changed lines from the same AST node cluster into one hunk
- Context lines are produced by taking sibling nodes that matched unchanged
- `IntraSpan` highlights use the CST byte spans directly — the changed range
  is defined by the difference between the matched node's text and its new text

---

# Language Support — MVP Phase

## Tier 1 (MVP — ship with these)

| Language | Crate | Grammar |
|---|---|---|
| Rust | `tree-sitter-rust` | rust |
| TypeScript | `tree-sitter-typescript` (typescript + tsx) | typescript, tsx |
| Python | `tree-sitter-python` | python |

## Tier 2 (post-MVP, add on demand)

| Language | Crate |
|---|---|
| JavaScript | `tree-sitter-javascript` |
| Go | `tree-sitter-go` |
| C | `tree-sitter-c` |
| C++ | `tree-sitter-cpp` |
| Java | `tree-sitter-java` |
| JSON | `tree-sitter-json` |
| YAML | `tree-sitter-yaml` |
| TOML | `tree-sitter-toml` |
| Markdown | `tree-sitter-md` |
| HTML | `tree-sitter-html` |
| CSS | `tree-sitter-css` |
| Shell | `tree-sitter-bash` |
| Ruby | `tree-sitter-ruby` |
| PHP | `tree-sitter-php` |
| Swift | `tree-sitter-swift` |
| Kotlin | `tree-sitter-kotlin` |

## Language Detection Strategy

```rust
fn language_from_extension(path: &Path) -> Option<SupportedLanguage> {
    match path.extension()?.to_str()? {
        "rs" => Some(Language::Rust),
        "ts" | "tsx" => Some(Language::TypeScript),
        "py" => Some(Language::Python),
        "js" | "jsx" => Some(Language::JavaScript),
        "go" => Some(Language::Go),
        "c" => Some(Language::C),
        // ... etc
        _ => None,  // fall back to line-based diff
    }
}
```

If `--algorithm syntax` is set but the language is unsupported, fall back
silently with a `warning` in `DiffResult::warnings`.

---

# Implementation Plan — Phased

## Phase 0: Dependency Setup

- Add to `crates/diff-core/Cargo.toml`:
  ```toml
  [dependencies]
  tree-sitter = "0.25"
  tree-sitter-rust = "0.23"
  tree-sitter-typescript = "0.25"
  tree-sitter-python = "0.23"
  ```

- Create module structure in `crates/diff-core/src/`:
  ```
  diff-core/src/
    lib.rs                # existing — add Syntax to DiffAlgorithm
    parser.rs             # new — language detection, parse → AstNodes
    ast_diff.rs           # new — AST matching algorithm
    ast_to_hunks.rs       # new — convert AST diff → LineChange/Hunk
  ```

## Phase 1: Parser Module (`parser.rs`)

Responsibilities:
- `Language` enum with a `grammar()` method returning `tree_sitter::Language`
- `FileLanguage` detection from extension (with optional CLI override via `--language`)
- `parse_file(text: &str, lang: Language) -> Vec<AstNode>` — parse and flatten CST
- Node filtering by depth (statement-level by default)
- Hashing (whitespace-stripped content hash for semantic matching)

Key decisions:
- **Reuse the `tree_sitter::Parser` across parses** (it's stateful and expensive to create)
- **Cache the parsed tree** by file path + content hash in `LruCache`
- **Thread safety:** Use a `Mutex<Parser>` pool since `tree_sitter::Parser` is not `Send`

## Phase 2: AST Matching (`ast_diff.rs`)

Implement `compute_ast_diff(left: &[AstNode], right: &[AstNode]) -> AstDiff`

```rust
struct AstDiff {
    matches: Vec<NodeMatch>,    // (left_id, right_id, similarity)
    left_unmatched: Vec<usize>, // left node ids with no right partner
    right_unmatched: Vec<usize>,// right node ids with no left partner
}

struct NodeMatch {
    left_id: usize,
    right_id: usize,
    classification: ChangeClass,  // identical, modified, moved, renamed
    similarity: f64,              // 0.0–1.0
}
```

Algorithm (simplified GumTree bottom-up):

```
1. Build hash buckets for both left and right nodes
2. For each hash that appears exactly once in both sides → mark as match
3. For each matched node, walk up the tree: if parent's children all match → match parent
4. For remaining unmatched named nodes, compute ancestry path signatures
5. Group unmatched nodes by signature; within each group, compute Levenshtein
   similarity on text content → best matches are moves/renames
6. All remaining nodes are true additions/deletions
```

## Phase 3: Hunk Production (`ast_to_hunks.rs`)

Convert `AstDiff` + source text → `Vec<LineChange>` → `Vec<Hunk>`

This is the bridge back into the existing `DiffResult` types. The key insight:
we don't need to change `Hunk` or `LineChange` — we produce the same types,
just with better content and spans.

```
1. Sort AST differences by line position (left file for deletions, right for additions)
2. For each changed region, emit a LineChange for each line
3. Set IntraSpan to the exact byte range from the AST node's changed portion
4. Group into Hunk using existing build_hunks() with context lines
5. For moved blocks, emit them as matched deletion + matched addition pair
   with a synthetic annotation
```

**Extension:** Add optional `SemanticTag` field to `Hunk` to expose the
classification to the UI:

```rust
pub struct Hunk {
    pub old_range: LineRange,
    pub new_range: LineRange,
    pub lines: Vec<LineChange>,
    pub foldable_context: bool,
    pub semantic_tag: Option<SemanticTag>,  // NEW
}

pub enum SemanticTag {  // NEW
    Modified,
    Moved,
    Renamed,
    Added,
    Deleted,
    Reformatted,
    TypeChanged,
}
```

## Phase 4: CLI Integration

- Add `--algorithm syntax` to the existing `AlgorithmArg` enum
- Add `--language <lang>` to override file-extension detection
- Add `--syntax-depth <statement|expression|token>` for granularity control
- Show `[semantic: moved]` annotations in human-readable output

## Phase 5: Desktop Integration

No changes needed at the IPC layer — `diff-core` produces the same
`DiffResult` struct. If `SemanticTag` is added to `Hunk`, the desktop
renderer can optionally show icons/badges per hunk.

---

# Performance Considerations

## Benchmark Expectations

| Scenario | Line-Based | AST (statement) | AST (expression) |
|---|---|---|---|
| `equal-1k-lines` | ~180ms | ~250ms (+parse) | ~350ms (+parse+tokens) |
| `sparse-edits` | ~3ms | ~50ms (+parse overhead) | ~80ms |
| `whitespace-churn` | ~100ms | ~120ms (identical → fast match) | ~140ms |
| `many-small-lines` | ~33s (Histogram) | Memory-bound | Risk |

**Key concern:** AST parsing + tree matching on large files. A 5K-line Rust
file produces ~300 statement-level nodes and ~2000 leaf tokens. The O(n²)
matching step needs optimization for large files.

## Mitigations

1. **Statement-level depth as default** — limits node count to ~100-500 per file
2. **Deadline cancellation** — the existing `deadline_ms` in `DiffRequest`
   applies to the AST path too. If exceeded, fall back to line-based diff.
3. **File size gate** — skip AST parsing for files > 10,000 lines or > 1 MB
   (configurable via `--syntax-max-size`)
4. **Parser reuse** — `tree_sitter::Parser` is pooled and reused, not
   re-allocated per file
5. **LruCache for parsed trees** — if the same file content is diffed twice
   (common in folder compare), the parsed CST is cached

## Memory

Each parsed tree holds the full source text in memory (tree-sitter copies it).
For a 1 MB file, expect ~2-3 MB per parsed tree (text + CST nodes). With
left + right files, ~4-6 MB per diff. The LruCache caps total memory.

---

# Open Questions To Investigate

| Question | Investigation Approach |
|---|---|
| Does tree-sitter parsing add acceptable overhead for <5K line files? | Run perf suite with `--algorithm syntax` on all existing cases |
| Can whitespace-normalized hashing reliably detect moves/renames? | Fixture: move a function to a different position in the same file; verify it's detected as `moved` not `deleted+added` |
| How much does node-count affect matching time? | Synthetic files with 100/500/1000/2000/5000 statements; measure match time |
| Does the tree-sitter CST handle incremental edits? | Not needed for MVP (we always diff two static files), but relevant for live editing |
| What's the memory overhead of loading 3 grammars concurrently? | Profile with `heaptrack` or `jemalloc` stats |
| Can we share a single `tree_sitter::Parser` across rayon threads? | `Parser` is not `Send` — need thread-local pool or mutex |
| Do all 3 MVP grammars compile cleanly on `aarch64-macos`? | Add them to deps and run `cargo build` |

---

# Test Strategy

## Fixtures

Create `fixtures/cases/syntax-*` directories:

| Fixture | What It Tests |
|---|---|
| `syntax-equal` | Identical files, no changes |
| `syntax-rename` | Function/variable rename detected |
| `syntax-move` | Function moved within file |
| `syntax-refactor` | Inline → extracted function |
| `syntax-reformat` | Only whitespace/formatting changes |
| `syntax-mixed` | Rename + logic change in same hunk |
| `syntax-unsupported` | Language not in MVP list → fallback to line-based |
| `syntax-large-file` | File > threshold → fallback |

Each fixture has `case.json` with the standard schema plus optional
`syntax_checks`:

```json
{
  "left": "left.rs",
  "right": "right.rs",
  "algorithm": "syntax",
  "language": "rust",
  "expected_classifications": [
    { "hunk": 1, "tag": "moved" },
    { "hunk": 2, "tag": "renamed" }
  ]
}
```

## Snapshot Tests

- `cargo test -p diff-core --test fixture_cases` runs all fixtures including
  syntax ones
- AST-based diffs produce the same `DiffResult` JSON, so existing snapshot
  machinery applies directly

## Perf Suite

A new scenario in `scripts/measure-performance.mjs`:

```json
{
  "name": "syntax-equal-5k",
  "left": "fixtures/perf/5k-equal-left.rs",
  "right": "fixtures/perf/5k-equal-right.rs",
  "algorithm": "syntax"
}
```

---

# Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| AST parsing is too slow for large files | Medium | High | Size gate + deadline fallback |
| Tree-sitter grammar versions break across updates | Low | Medium | Lock grammar versions in Cargo.toml, CI tests pin them |
| Node matching produces worse results than line-based on real code | Medium | Medium | Always compare both; `syntax` is opt-in via flag |
| Mutex contention on Parser pool | Low | Low | Thread-local pool or `ObjectPool` |
| Language detection fails for edge-case extensions | Low | Low | `--language` override flag + `.gitattributes` support |

---

# Glossary

| Term | Definition |
|---|---|
| CST | Concrete Syntax Tree — the exact parse tree from tree-sitter |
| AST | Abstract Syntax Tree — a simplified CST (terms used interchangeably here) |
| GumTree | A tree-diff algorithm that uses bottom-up container matching |
| Named node | A tree-sitter node with a semantic name (e.g. `function_definition`) |
| Anonymous node | A tree-sitter node for syntax tokens (e.g. `(`, `)`, `;`) |
| Statement depth | Tree-sitter nodes at the level of statements, function defs, class defs |
| Expression depth | Includes expression-level nodes (e.g. `binary_expression`, `call_expression`) |
| Token depth | All leaf nodes (individual keywords, identifiers, operators) |
