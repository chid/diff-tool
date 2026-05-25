# Office Adapter — Handoff & Next Steps

> **Date:** May 2025  
> **Status:** Phase 1-2 complete, Word semantic comparison wired, PPTX/XLSX extracts stubbed  
> **Tests:** 82 passing (0 failures)

---

## What Works (Solid)

### Format Detection & Unpacking
- Extension + zip-signature detection for `.docx`, `.pptx`, `.xlsx` ✓
- Safe zip reading with configurable part size/count limits (bomb protection) ✓
- Oversized/banned part tracking in `unsupported_features` ✓

### Word (.docx) — Full Pipeline ✓
- **Approach 1 (Text):** `word/extract.rs::extract_text()` — single-pass XML → plain text ✓
- **Approach 1 (Structured):** `word/extract.rs::extract_document()` — XML → `WordDocument` model with paragraphs, tables, styles, heading levels ✓
- **Approach 2 (XML Tree):** `word/xml_diff.rs::compare_xml_trees()` — event-stream structural comparison ✓
- **Approach 3 (Semantic):** `word/compare.rs::compare_documents()` — model-to-model diff with paragraph, table, and cell-level change detection ✓
- **Public API:** `compare_office_files()` wired to Approach 3 (semantic) → `OfficeDiffResult` ✓

### PowerPoint (.pptx) — Extraction Only
- `powerpoint/extract.rs::extract_slide_text()` — placeholder-aware text extraction (`[title]`, `[body]`, `[shape]`) ✓
- Picture shapes correctly skipped ✓
- Multi-paragraph shapes handled ✓
- **Semantic compare:** NOT YET — returns empty sections with "not yet implemented" warning

### Excel (.xlsx) — Extraction Only
- `excel/extract.rs::extract_sheet_text()` — inlineStr + numeric cell extraction ✓
- Sparse column handling (sorted by column reference) ✓
- Shared string cells (`t="s"`) gracefully skipped (no crash) ✓
- **Semantic compare:** NOT YET — returns empty sections with "not yet implemented" warning

### Test Infrastructure
- 5 scenario integration test runner (`tests/approaches_bench.rs`) with formatted output ✓
- 16 fixture-driven tests reading real `.docx/.pptx/.xlsx` files from disk ✓
- Auto-generated fixture binary files (idempotent, skips if exists) ✓
- Inline unit tests across all extraction and comparison modules ✓

---

## What's Missing (Prioritized)

### Priority 1: PowerPoint Semantic Compare
**Why:** PPTX is the second most common Office format. The extraction is done — now it needs comparison.

**Files to create:**
- `crates/office-adapter/src/powerpoint/compare.rs`

**What it needs to do:**
- Parse multiple slides from `ppt/slides/slideN.xml`
- Match slides by index (slide 1 ↔ slide 1)
- Extract slide metadata (title, body text, shape count, image count)
- Compare slide-by-slide → `Vec<OfficeSectionDiff>`
- Detect slide additions/deletions when slide counts differ

**Reference pattern:** `word/compare.rs` — same structure, different XML namespace (`p:` + `a:`)

### Priority 2: Excel Semantic Compare
**Why:** XLSX is the most data-heavy format — cell-level diff is the whole value prop.

**Files to create:**
- `crates/office-adapter/src/excel/compare.rs`

**What it needs to do:**
- Parse shared string table (`xl/sharedStrings.xml`) — currently skipped
- Match sheets by name across left/right workbooks
- Compare cells by row+col within matched sheets
- Detect formula changes vs value changes
- Detect sheet additions/deletions/renames
- Handle numeric precision (round to avoid `1.200000000000001` false diffs)

**Reference pattern:** `word/compare.rs` — same `OfficeSectionDiff` output model

### Priority 3: Wire PPTX/XLSX into `compare_office_files()`
Once `powerpoint/compare.rs` and `excel/compare.rs` exist, replace these blocks in `lib.rs`:

```rust
OfficeFormat::PowerPoint => {
    warnings.push("PowerPoint comparison is not yet implemented...");
    Vec::new()
}
OfficeFormat::Excel => {
    warnings.push("Excel comparison is not yet implemented...");
    Vec::new()
}
```

With actual extraction + comparison calls following the Word pattern:
```rust
OfficeFormat::PowerPoint => {
    let left_slides = ...;
    let right_slides = ...;
    powerpoint::compare::compare_presentations(&left_slides, &right_slides)?
}
```

### Priority 4: Tracked Changes Filtering
**Why:** Documents with existing tracked changes produce incorrect diffs — deleted text appears as content.

**Files to modify:**
- `crates/office-adapter/src/word/extract.rs`

**What it needs to do:**
- In `extract_text()` and `extract_document()`: detect `<w:del>` elements and skip their content
- Detect `<w:ins>` elements and treat their content as normal (it's been accepted)
- Preserve this as a configurable option in `OfficeDiffOptions`

### Priority 5: Shared String Table (XLSX)
**Why:** Many real Excel files use shared strings (`t="s"`). Currently these cells are skipped.

**Files to modify:**
- `crates/office-adapter/src/excel/extract.rs`

**What it needs to do:**
- Parse `xl/sharedStrings.xml` into a `Vec<String>`
- When a cell has `t="s"` and a `<v>N</v>`, resolve index N from the shared strings table
- Pass the SST into `extract_sheet_text()` as an optional parameter

### Priority 6: CLI Integration
**Why:** The `office-adapter` crate works in tests but has no CLI entry point.

**Files to modify:**
- `crates/diff-cli/src/main.rs`

**What it needs to do:**
- Detect `.docx/.pptx/.xlsx` extensions in file args
- Route to `office_adapter::compare_office_files()` instead of `diff_core::compare_texts()`
- Render `OfficeDiffResult` in `--human` and `--json` modes
- Add `--sheets`, `--range` flags for Excel-specific options

---

## Codebase Map (Key Files)

| File | Purpose | Status |
|------|---------|--------|
| `crates/office-adapter/src/lib.rs` | Public API + `compute_summary()` | ✅ Wired for Word |
| `crates/office-adapter/src/format.rs` | Format detection | ✅ Done |
| `crates/office-adapter/src/unpack.rs` | Safe zip reading | ✅ Done |
| `crates/office-adapter/src/models.rs` | Data models | ✅ Done |
| `crates/office-adapter/src/xml_util.rs` | XML helpers | ✅ Done |
| `crates/office-adapter/src/word/extract.rs` | Word text + structured extraction | ✅ Done |
| `crates/office-adapter/src/word/xml_diff.rs` | XML tree comparison | ✅ Done |
| `crates/office-adapter/src/word/compare.rs` | Semantic Word comparison | ✅ Done |
| `crates/office-adapter/src/powerpoint/extract.rs` | PowerPoint text extraction | ✅ Done |
| `crates/office-adapter/src/powerpoint/compare.rs` | PowerPoint semantic comparison | ❌ Not started |
| `crates/office-adapter/src/excel/extract.rs` | Excel text extraction | ✅ Done |
| `crates/office-adapter/src/excel/compare.rs` | Excel semantic comparison | ❌ Not started |
| `crates/office-adapter/tests/approaches_bench.rs` | Approach comparison runner | ✅ Done |
| `crates/office-adapter/tests/fixture_cases.rs` | Fixture-driven tests | ✅ Done |
| `crates/office-adapter/tests/common/mod.rs` | Test fixture generators | ✅ Done |
| `docs/office-research/README.md` | Research summary | ✅ Done |
| `docs/office-research/approaches.md` | Detailed approach comparison | ✅ Done |
| `docs/office-research/index.html` | Visual research report | ✅ Done |
| `fixtures/cases/office-word-basic/` | Word fixture files | ✅ On disk |
| `fixtures/cases/office-pptx-basic/` | PowerPoint fixture files | ✅ On disk |
| `fixtures/cases/office-xlsx-basic/` | Excel fixture files | ✅ On disk |

---

## Commands

```bash
# Run all office-adapter tests
cargo test -p office-adapter

# Run the approach comparison bench (with formatted output)
cargo test -p office-adapter --test approaches_bench -- --nocapture

# Run only fixture tests
cargo test -p office-adapter --test fixture_cases -- --nocapture

# Run the full project check
./scripts/check.sh
```

---

## Design Rules (Don't Break These)

1. **Don't assume namespace prefixes** — Excel uses a default namespace (no `s:` prefix), PowerPoint uses `p:` + `a:`, Word uses `w:`. Check raw bytes.
2. **Always decrement depth after `read_text()`** — it consumes the End event silently.
3. **Always decrement depth after child extraction functions return** — they consume End events internally.
4. **Whitespace:** Don't trim during raw extraction (causes false diffs). Trim at the display layer only.
5. **Tracked changes:** `<w:del>` content should be filterable, not shown as real content.
6. **Deterministic JSON output** — all models derive `Serialize + PartialEq` for snapshot testing.

---

## Known Sharp Edges

- **quick-xml 0.37** has no namespace resolution API — we match prefixes via byte comparison
- **Excel default namespace:** worksheet XML often has no `s:` prefix — we fall back to unprefixed element name matching
- **PowerPoint pictures:** `<p:pic>` elements are naturally skipped (no `<a:t>` inside) — no special handling needed
- **Shared strings:** Cells with `t="s"` are silently skipped rather than crashing — they produce incomplete output
- **Whitespace preservation:** `xml:space="preserve"` in `<w:t>` is not yet respected — could lose intentional whitespace in some documents
