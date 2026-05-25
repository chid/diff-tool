# Office Diff Approaches — Detailed Comparison

## Approach 1: Convert-to-Text

### How It Works

1. Parse `word/document.xml` using `quick-xml`
2. Walk all `<w:p>` (paragraph) elements
3. For each paragraph, collect text from all `<w:r>/<w:t>` (run/text) elements
4. Join paragraphs with newlines → flat text
5. Feed into `diff-core::compare_texts()` for line-level diff

### Pros

- **Simplicity**: ~50 lines of code
- **Reuses existing engine**: Leverages battle-tested `diff-core`
- **Fast**: No complex parsing, single XML pass
- **Good for content review**: "What text changed?" answered clearly
- **Easy to implement for all formats**: Same pattern works for .pptx and .xlsx

### Cons

- **No structure**: Paragraph boundaries become line breaks, but you can't
  say "paragraph 3 was deleted" vs "paragraph 3 text changed"
- **No formatting**: Bold→Normal is invisible (text content unchanged)
- **No tables**: Table cells interleave with paragraph text, losing context
- **No tracked changes**: Deleted text in tracked changes appears as content
- **False positives**: Whitespace-only changes in XML padding flag as changes
- **Lossy**: Can't reconstruct which change belongs to which document element

### Best For

- Quick "what content changed?" preview
- Integration with git diff (word-to-text filter)
- Performance-critical paths where structure doesn't matter

---

## Approach 2: XML Tree Comparison

### How It Works

1. Parse both `word/document.xml` files into a stream of `quick-xml` events
2. Walk both event streams in lockstep, matching elements by name + depth
3. When elements diverge, record as Added/Deleted/Modified
4. For matching elements, recurse into children
5. Report a flat list of XML path → change kind

### Pros

- **Full visibility**: Every XML element change is captured
- **Formatting-aware**: `<w:rPr>` (run properties) changes are detected
- **Structure-preserving**: Table structure, section breaks all preserved
- **Extensible**: Works with any OOXML part (styles, comments, headers)
- **No semantic knowledge needed**: Pure structural comparison

### Cons

- **Too sensitive**: Attribute reordering in XML counts as a change
  (e.g., `<w:rPr><w:b/><w:i/></w:rPr>` vs `<w:rPr><w:i/><w:b/></w:rPr>`
  are semantically identical but structurally different)
- **Namespace-aware but noisy**: Element name comparisons must strip namespaces
- **No semantic grouping**: A paragraph text change and its formatting change
  appear as separate diff entries
- **Hard to present to users**: "Element /w:document[1]/w:body[1]/w:p[3]/w:r[1]/w:t[1]
  changed from 'old' to 'new'" is not user-friendly
- **Tracked changes**: `<w:ins>` and `<w:del>` elements appear as structural
  changes rather than being recognized for what they are

### Best For

- Debugging the semantic parser
- Validating that two files are byte-for-byte equivalent (ignoring attribute order)
- Automated testing of Office document generators

---

## Approach 3: Semantic Model Comparison

### How It Works

1. Parse `word/document.xml` into structured `WordDocument` model
   - `WordParagraph { index, text, style, heading_level, runs: Vec<Run> }`
   - `WordTable { index, rows, cols, cells }`
   - `WordComment { id, author, text }`
2. Filter tracked changes: skip content inside `<w:del>` elements
3. Match sections by index (paragraph 3 ↔ paragraph 3)
4. For each matched pair:
   - Compare extracted text → `TextChanged` atomic change
   - Compare run-level formatting → `FormattingChanged` atomic change
   - Detect structural mismatches (one side has table, other has paragraph)
5. Report `Vec<OfficeSectionDiff>` with rich metadata

### Pros

- **Most meaningful**: "Paragraph 3: text changed from 'Q1 2025' to 'Q2 2025'"
- **Formatting-aware**: Catches bold/italic/underline changes
- **Handles tracked changes**: Filters ghost content from prior editing
- **Table-aware**: Compares cells position-by-position
- **Extensible**: New section types (headers, footers, footnotes) plug in naturally
- **UI-friendly**: Output directly feeds `OfficeSectionDiff` → React components
- **Deterministic**: Same inputs → identical JSON (critical for snapshot tests)

### Cons

- **Complexity**: Requires full OOXML schema knowledge per format
- **Format-specific**: Word, PowerPoint, and Excel each need their own parser
- **Maintenance burden**: Office schema evolves; parsers need updates
- **Performance**: More allocations than text-only extraction
- **Incomplete coverage risk**: If a rarely-used OOXML feature isn't parsed,
  those changes are invisible

### Best For

- The primary diff engine (what users see)
- AI enrichment (structured diffs are easier to summarize)
- Desktop UI rendering with format-specific navigation
- Collaborative review workflows

---

## Edge Case Comparison

| Edge Case | Text (A1) | XML Tree (A2) | Semantic (A3) |
|-----------|-----------|---------------|---------------|
| Formatting change only (bold→normal) | ❌ No diff | ✅ Detects `<w:rPr>` change | ✅ Reports `FormattingChanged` |
| Tracked changes in XML (`<w:del>`) | ❌ Shows deleted text | ❌ Shows structural noise | ✅ Filters tracked changes |
| Reordered XML attributes | ❌ N/A (text only) | ⚠️ False diff | ✅ Normalizes during parse |
| Running head change | ❌ Shows as text diff | ✅ Detects as header change | ✅ Separates from body changes |
| Table cell content change | ❌ Cell text mixed with body | ✅ Detects as `<w:tc>` change | ✅ Reports cell-level path |
| Comment added | ❌ N/A (separate XML part) | ❌ Only if `comments.xml` diffed | ✅ Separate comparison path |
| Embedded image changed | ❌ N/A | ❌ Hash compare needed | ✅ Hash compare + metadata diff |
| Whitespace-only paragraph | ⚠️ Could flag as change | ✅ Detects element presence | ✅ Normalizes whitespace |
| Slide reordering (PPTX) | ❌ Text diff meaningless | ⚠️ Hard to match slides | ✅ Match by slide ID |

---

## Implementation Notes

All three approaches are implemented in:
- `crates/office-adapter/src/word/extract.rs` — Text extraction (Approach 1 + shared for 3)
- `crates/office-adapter/src/word/xml_diff.rs` — XML tree diff (Approach 2)
- `crates/office-adapter/src/word/compare.rs` — Semantic comparison (Approach 3)

Integration tests that exercise all three against reference files live in:
- `crates/office-adapter/tests/approaches_bench.rs`

Reference `.docx` files are generated from:
- `crates/office-adapter/tests/common/mod.rs`
