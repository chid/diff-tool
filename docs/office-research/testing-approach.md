# Office Adapter — Comprehensive Testing Approach

> **Date:** May 2025  
> **Based on:** 82 passing tests across 4 test binaries (54 lib + 12 approaches_bench + 16 fixture_cases), 3 approaches implemented, 2 formats stubbed

---

## Table of Contents

1. [Testing Philosophy & Pyramid](#1-testing-philosophy--pyramid)
2. [Feature Inventory: What We're Testing](#2-feature-inventory-what-were-testing)
3. [Current Test Coverage Matrix](#3-current-test-coverage-matrix)
4. [Edge Case Catalog](#4-edge-case-catalog)
5. [Test Gap Analysis](#5-test-gap-analysis)
6. [Attack Surface: Things That Can Break](#6-attack-surface-things-that-can-break)
7. [Test Strategy for Upcoming Features](#7-test-strategy-for-upcoming-features)
8. [Test Commands & Automation](#8-test-commands--automation)
9. [Performance Regression Testing](#9-performance-regression-testing)
10. [Documentation & Handoff](#10-documentation--handoff)

---

## 1. Testing Philosophy & Pyramid

### Pyramid Structure

```
        ┌─────────┐
        │  E2E    │  1-3 tests     ← Full CLI pipeline with real files
        ├─────────┤
        │Fixtures │  16 tests       ← Disk → extract → compare → assert
        ├─────────┤
        │  Integ  │  12 tests       ← Cross-module: generate docx → compare approaches
        ├─────────┤
        │  Unit   │  54 tests       ← Per-function: extract, compare, models, unpack, format
        └─────────┘
```

### Principles

1. **Fixture-driven first.** Every format, approach, and edge case should have a corresponding fixture pair (left/right files) on disk. Fixtures are the single source of truth for diff behavior.

2. **Test the output shape, not the algorithm.** Assert on `OfficeDiffResult` fields (status, kind, before/after), not internal implementation details. This keeps tests resilient to refactoring.

3. **Round-trip serialization for every model.** Every `Serialize + Deserialize` type must have a round-trip test verifying JSON stability.

4. **Safety paths tested with malformed input.** Zip bombs, corrupt files, missing parts, and oversized entries each have dedicated tests.

5. **Approach comparison validates documented trade-offs.** The approaches_bench test explicitly verifies that Approach 1 misses formatting changes and Approach 3 catches everything.

6. **Inline tests co-located with implementation.** Each module tests its own behavior in `#[cfg(test)] mod tests` blocks, following Rust convention.

---

## 2. Feature Inventory: What We're Testing

### Core Pipeline (All Formats)

| Component | Module | What It Does | Test Location |
|-----------|--------|-------------|---------------|
| Format detection | `format.rs` | Extension → OfficeFormat, zip signature check | `format.rs` inline + `fixture_cases.rs` |
| Zip unpacking | `unpack.rs` | Safe reading with size/count limits | `unpack.rs` inline + `fixture_cases.rs` |
| XML reader config | `xml_util.rs` | `expand_empty_elements`, disabled DTD | `xml_util.rs` inline |
| Public API | `lib.rs` | `compare_office_files()` + `compute_summary()` | `lib.rs` inline + `fixture_cases.rs` |
| Data models | `models.rs` | Serialization, defaults, enum coverage | `models.rs` inline |

### Word (.docx) — Implementation & Test Coverage

| Function | Module | What It Does | Test Count | Test Location |
|----------|--------|-------------|------------|---------------|
| `extract_text()` | `word/extract.rs` | XML → plain text (Approach 1a) | 3 inline | `extract.rs` tests |
| `extract_document()` | `word/extract.rs` | XML → `WordDocument` model | 3 inline | `extract.rs` tests |
| `extract_table_text()` | `word/extract.rs` | Table text extraction from `<w:tbl>` | (exercised by above) | — |
| `parse_paragraph()` | `word/extract.rs` | `<w:p>` → `WordParagraph` | (exercised by `extract_document`) | — |
| `parse_table()` | `word/extract.rs` | `<w:tbl>` → `WordTable` | (exercised by `extract_document`) | — |
| `compare_documents()` | `word/compare.rs` | Semantic paragraph + table diff | 6 inline | `compare.rs` tests |
| `compare_paragraphs()` | `word/compare.rs` | Index-matched paragraph comparison | (exercised by above) | — |
| `compare_tables()` | `word/compare.rs` | Cell-level table comparison | (exercised by above) | — |
| `compare_xml_trees()` | `word/xml_diff.rs` | Event-stream structural diff | 5 inline | `xml_diff.rs` tests |

### PowerPoint (.pptx) — Implementation & Test Coverage

| Function | Module | What It Does | Test Count | Test Location |
|----------|--------|-------------|------------|---------------|
| `extract_slide_text()` | `powerpoint/extract.rs` | Slide XML → labeled text | 7 inline | `extract.rs` tests |
| `extract_presentation_text()` | `powerpoint/extract.rs` | Multi-slide extraction | (exercised by `extract_slide_text`) | — |
| **compare** | NOT YET | Semantic slide comparison | 0 | ❌ Not started |

### Excel (.xlsx) — Implementation & Test Coverage

| Function | Module | What It Does | Test Count | Test Location |
|----------|--------|-------------|------------|---------------|
| `extract_sheet_text()` | `excel/extract.rs` | Worksheet XML → tab-separated rows | 8 inline | `extract.rs` tests |
| `extract_workbook_text()` | `excel/extract.rs` | Multi-sheet extraction | (exercised by `extract_sheet_text`) | — |
| **compare** | NOT YET | Semantic cell comparison | 0 | ❌ Not started |
| **shared strings** | NOT YET | Parse `xl/sharedStrings.xml` | 0 | ❌ Not started |

### Test Infrastructure

| File | Purpose | Test Count |
|------|---------|------------|
| `tests/common/mod.rs` | Fixture generators (`generate_docx`, `generate_pptx`, `generate_xlsx`, 20+ body templates) | 3 self-tests |
| `tests/fixture_cases.rs` | Disk-based fixture tests (format detection, unpacking, comparison, error paths) | 16 |
| `tests/approaches_bench.rs` | Cross-approach comparison runner (5 scenarios) | 12 |
| `docs/office-research/index.html` | Visual research report | 0 (documentation) |
| `docs/office-research/HANDOFF.md` | Handoff & next steps | 0 (documentation) |

---

## 3. Current Test Coverage Matrix

### By Feature (✅ tested, ⚠️ partially covered, ❌ untested)

| Feature | Unit | Integration | Fixture | E2E | Notes |
|---------|------|-------------|---------|-----|-------|
| Format detection (docx/pptx/xlsx) | ✅ | — | ✅ | ❌ | |
| Format detection (unknown ext) | ✅ | ✅ | ✅ | ❌ | |
| Format detection (case insensitive) | ✅ | — | — | ❌ | `.DOCX` tested |
| Zip unpacking (valid) | ✅ | — | ✅ | ❌ | |
| Zip unpacking (corrupt) | ✅ | — | ✅ | ❌ | |
| Zip unpacking (part count limit) | ✅ | — | — | ❌ | |
| Zip unpacking (part size limit) | ✅ | — | — | ❌ | |
| Zip unpacking (total size limit) | ✅ | — | — | ❌ | |
| Zip unpacking (directory entries) | ⚠️ | — | — | ❌ | Implicit in tests, no explicit dir test |
| XML reader creation | ✅ | — | — | ❌ | |
| Text extraction from events | ✅ | — | — | ❌ | |
| Word: `extract_text()` — simple | ✅ | — | ✅ | ❌ | |
| Word: `extract_text()` — multi-paragraph | ✅ | — | ✅ | ❌ | |
| Word: `extract_text()` — with table | ✅ | — | ✅ | ❌ | |
| Word: `extract_document()` — simple | ✅ | ✅ | ✅ | ❌ | |
| Word: `extract_document()` — heading | ✅ | ✅ | ✅ | ❌ | Tested via heading style |
| Word: `extract_document()` — formatting | ✅ | ✅ | ✅ | ❌ | Bold detection |
| Word: `extract_document()` — multi-heading | ✅ | — | — | ❌ | Heading1 + Heading2 |
| Word: `compare_documents()` — identical | ✅ | — | ✅ | ❌ | |
| Word: `compare_documents()` — text change | ✅ | ✅ | ✅ | ❌ | |
| Word: `compare_documents()` — added para | ✅ | — | — | ❌ | |
| Word: `compare_documents()` — deleted para | ❌ | — | — | ❌ | Code handles deletion but no explicit test |
| Word: `compare_documents()` — heading kind | ✅ | — | — | ❌ | |
| Word: `compare_documents()` — formatting change | ✅ | ✅ | — | ❌ | Bold→normal via style diff |
| Word: `compare_documents()` — table cell | ✅ | ✅ | ✅ | ❌ | |
| Word: `compare_documents()` — table dim change | ⚠️ | — | — | ❌ | Structure changed but no dimension test |
| Word: `compare_documents()` — JSON serialize | ✅ | ✅ | — | ❌ | Round-trip test exists |
| Word: `compare_xml_trees()` — identical | ✅ | — | — | ❌ | |
| Word: `compare_xml_trees()` — text change | ✅ | — | — | ❌ | |
| Word: `compare_xml_trees()` — added element | ✅ | — | — | ❌ | |
| Word: `compare_xml_trees()` — deleted element | ✅ | — | — | ❌ | |
| Word: `compare_xml_trees()` — serialize | ✅ | — | — | ❌ | |
| PowerPoint: `extract_slide_text()` — title+body | ✅ | — | ✅ | ❌ | |
| PowerPoint: `extract_slide_text()` — empty slide | ✅ | — | — | ❌ | |
| PowerPoint: `extract_slide_text()` — multi-para | ✅ | — | — | ❌ | |
| PowerPoint: `extract_slide_text()` — formatted runs | ✅ | — | — | ❌ | |
| PowerPoint: `extract_slide_text()` — picture skip | ✅ | — | — | ❌ | |
| PowerPoint: `extract_slide_text()` — center title | ✅ | — | — | ❌ | `type="ctrTitle"` |
| PowerPoint: `extract_slide_text()` — subtitle | ❌ | — | — | ❌ | `type="subTitle"` not tested |
| PowerPoint semantic compare | ❌ | ❌ | ❌ | ❌ | **NOT IMPLEMENTED** |
| Excel: `extract_sheet_text()` — single cell | ✅ | — | — | ❌ | |
| Excel: `extract_sheet_text()` — multi-cell | ✅ | — | — | ❌ | |
| Excel: `extract_sheet_text()` — multi-row | ✅ | — | — | ❌ | |
| Excel: `extract_sheet_text()` — sparse cols | ✅ | — | — | ❌ | |
| Excel: `extract_sheet_text()` — empty sheet | ✅ | — | — | ❌ | |
| Excel: `extract_sheet_text()` — numeric cells | ✅ | — | — | ❌ | |
| Excel: `extract_sheet_text()` — mixed types | ✅ | — | — | ❌ | |
| Excel: `extract_sheet_text()` — shared strings | ⚠️ | — | — | ❌ | Skipped gracefully, not resolved |
| Excel: `extract_sheet_text()` — formula cells | ❌ | — | — | ❌ | **NOT TESTED** |
| Excel semantic compare | ❌ | ❌ | ❌ | ❌ | **NOT IMPLEMENTED** |
| Excel shared string table | ❌ | ❌ | ❌ | ❌ | **NOT IMPLEMENTED** |
| Public API: Word comparison | ✅ | — | ✅ | ❌ | |
| Public API: PPTX comparison | — | — | ✅ | ❌ | Verifies "not yet implemented" warning |
| Public API: XLSX comparison | — | — | ✅ | ❌ | Verifies "not yet implemented" warning |
| Public API: mismatched formats | ✅ | — | ✅ | ❌ | |
| Public API: `compute_summary()` | ✅ | — | ✅ | ❌ | |
| Public API: `OfficeDiffOptions` defaults | ✅ | — | — | ❌ | |
| Public API: `OfficeDiffResult` serialize | ✅ | — | — | ❌ | |
| Public API: `ChangeStatus` serde round-trip | ✅ | — | — | ❌ | All 5 variants |
| Error: unknown format | ✅ | — | ✅ | ❌ | |
| Error: invalid/corrupt zip | ✅ | — | ✅ | ❌ | |
| Error: missing part | ✅ | — | — | ❌ | Tested via missing `word/document.xml` |
| Error: file too large | ✅ | — | — | ❌ | |
| Error: too many parts | ✅ | — | — | ❌ | |
| Error: part too large | ⚠️ | — | — | ❌ | Tested as oversized skip, not error |
| Zip bomb protection (billion laughs) | ❌ | — | — | ❌ | DTD disabled but no explicit entity test |
| Concurrent/multi-thread safety | ❌ | — | — | ❌ | `Send + Sync` not verified |
| Tracked changes: `<w:del>` filtered | ❌ | — | ⚠️ | ❌ | Scenarios exist but not filtering yet |
| Tracked changes: `<w:ins>` accepted | ❌ | — | ⚠️ | ❌ | Scenarios exist but not filtering yet |
| Whitespace: `xml:space="preserve"` | ❌ | — | — | ❌ | Not respected yet |

### By Test Binary

| Binary | Tests | What It Covers |
|--------|-------|----------------|
| `office-adapter` (lib) | 54 | All inline `#[cfg(test)]` blocks across format, unpack, xml_util, models, lib, word/extract, word/compare, word/xml_diff, powerpoint/extract, excel/extract |
| `tests/approaches_bench` | 12 | 5 scenario print + 6 assertion tests + 1 empty (fixture gen self-tests in common) |
| `tests/fixture_cases` | 16 | Format detection, zip unpacking, compare_office_files, error paths, content verification |
| `tests/common` self-tests | 3 | `col_to_letter`, `generate_docx`, `generate_pptx`, `generate_xlsx` validity checks |

---

## 4. Edge Case Catalog

### 4.1 Word (.docx) Edge Cases

#### Content Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| W1 | Empty document (no `<w:body>` content) | Panic on empty body | ❌ | Extract should return empty string / 0 paragraphs |
| W2 | Single empty paragraph `<w:p/>` | Trimming loses empty para | ❌ | Compare should report Unchanged for both-empty |
| W3 | Very long paragraph (>10K chars) | Buffer overflow, performance | ❌ | Extract + compare with 100K char paragraph |
| W4 | Deeply nested runs (100+ `<w:r>` in one `<w:p>`) | Stack overflow / depth tracking | ❌ | Extract should handle arbitrary run count |
| W5 | Unicode (emoji, RTL, CJK, combining chars) | Encoding issues, display corruption | ❌ | "Hello 👋 你好 مرحبا" in `<w:t>` |
| W6 | `xml:space="preserve"` attribute on `<w:t>` | Whitespace lost | ❌ | Leading/trailing spaces preserved |
| W7 | Soft hyphen (`\u{AD}`), non-breaking space (`\u{A0}`) | Incorrect text comparison | ❌ | These should be preserved in extracted text |
| W8 | Field codes (`<w:fldChar>`, `<w:instrText>`) | Field noise appears as content | ❌ | PAGE, DATE fields should be skipable |
| W9 | Bookmarks (`<w:bookmarkStart>`, `<w:bookmarkEnd>`) | Bookmark tags in text | ❌ | Should not affect extracted text |
| W10 | Hyperlinks (`<w:hyperlink>`) | Link text extracted, URL lost | ❌ | Link text should appear, metadata in model |

#### Structural Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| W11 | Section breaks (`<w:sectPr>`) | Section metadata ignored | ❌ | Should not cause parse errors |
| W12 | Nested table (table inside table cell) | Infinite recursion, depth issues | ❌ | Graceful handling (extract inner table text) |
| W13 | Table with merged cells (`<w:tc>` with `w:gridSpan`) | Cell mismatch, wrong col count | ❌ | `gridSpan` should inform cell layout |
| W14 | Table with vertically merged cells (`<w:vMerge>`) | Row count mismatch | ❌ | `vMerge` should inform row layout |
| W15 | Mixed content: paragraph → table → paragraph | Depth tracking after table | ✅ | Already tested; regression guard |
| W16 | Multiple tables in sequence | Table indexing correct | ❌ | Tables should be indexed 1, 2, 3 |
| W17 | Table with no `<w:tblGrid>` | Missing grid causes panic | ❌ | Graceful fallback to 0 cols |
| W18 | Table with empty cells | Empty cell comparison | ❌ | "" vs None distinction |
| W19 | Images inline (`<w:drawing>`) | Image metadata extraction | ❌ | Hash compare, not text extraction |

#### Formatting Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| W20 | Bold + italic + underline simultaneously | Multi-flag run properties | ❌ | FormattingChanged should list all changed props |
| W21 | Font change (Times → Arial) | Font metadata in `w:rFonts` | ❌ | FormattingChanged with font metadata |
| W22 | Font size change (12pt → 14pt) | `w:sz` attribute | ❌ | FormattingChanged with size metadata |
| W23 | Color change (black → red) | `w:color` attribute | ❌ | FormattingChanged with color metadata |
| W24 | Heading level change (H1 → H2) | Style name comparison | ⚠️ | `pStyle` change detected, but level not parsed |
| W25 | List paragraph (`<w:pStyle w:val="ListParagraph"/>`) | List style detection | ❌ | Should be detected as styled |
| W26 | Character style (`<w:rStyle>`) | Run-level style application | ❌ | Style diff at character level |

#### Tracked Changes Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| W27 | `<w:del>` wraps entire paragraph | Deleted text shown as content | ⚠️ | Test scenario exists, filtering NOT implemented |
| W28 | `<w:del>` wraps part of a run | Partial deletion in extracted text | ❌ | Run text should show only undeleted portion |
| W29 | `<w:ins>` with author/date metadata | Added text treated as normal | ⚠️ | Test scenario exists, acceptance NOT implemented |
| W30 | Nested tracked changes | `ins` inside `del` or vice versa | ❌ | Inner-most change wins? |
| W31 | `w:del` with `w:moveFrom`/`w:moveTo` | Move tracking different from delete | ❌ | At minimum, don't crash |
| W32 | Multiple revisions by different authors | Metadata on each change | ❌ | Author/date preserved in metadata |

#### Comparison Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| W33 | Only left has paragraphs (all deleted) | Right side empty comparison | ❌ | All sections report Deleted |
| W34 | Only right has paragraphs (all added) | Left side empty comparison | ❌ | All sections report Added |
| W35 | Paragraph reordering | A, B, C → C, A, B | ❌ | Currently matches by index; wrong pairing |
| W36 | Near-duplicate paragraphs | "Hello" vs "Hello" — index match is correct | ✅ | Already works |
| W37 | Identical text but different position | Same text at different indices | ❌ | Currently matches by index — may report false diff |
| W38 | Large document (10K+ paragraphs) | Performance, memory | ❌ | Benchmark threshold |
| W39 | Empty `before`/`after` in atomic change | Serialization of None | ✅ | Already round-trip tested |
| W40 | Content controls (`<w:sdt>`, `<w:sdtContent>`) | Structured document tags in templates | ❌ | At minimum, extract inner text; don't crash |
| W41 | Alt-text / accessibility metadata | `<wp:docPr descr="...">` on images | ❌ | Non-critical but useful for comparison |

### 4.2 PowerPoint (.pptx) Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| P1 | Slide with only images (no text shapes) | Empty extraction output | ✅ | Pictures naturally skipped |
| P2 | Slide with tables (`<a:graphic>` → `<a:tbl>`) | Table text not extracted | ❌ | Extract table text from slide tables |
| P3 | Slide with charts (`<c:chart>`) | Chart data ignored | ❌ | At minimum, don't crash |
| P4 | Slide with SmartArt (`<dgm:>`) | SmartArt text nodes | ❌ | Extract SmartArt text |
| P5 | Slide with grouped shapes (`<p:grpSp>`) | Nested shape text | ❌ | Recurse into groups |
| P6 | Multiple slides (slide1 + slide2 + slide3) | Slide indexing, missing slides | ❌ | Match by index, detect count mismatch |
| P7 | Slide reordering | Slide 1 ↔ Slide 2 swapped | ❌ | Slide ID-based matching |
| P8 | Slide master content (`slideMaster`) | Content from master, not slide | ❌ | At minimum, don't crash |
| P9 | Notes pages (`notesSlide`) | Notes text extraction | ❌ | Separate comparison section |
| P10 | Animations / transitions | XML noise | ❌ | Should not affect text extraction |
| P11 | Chinese/Japanese/Korean text in shapes | CJK encoding in `<a:t>` | ❌ | Verify correct extraction |
| P12 | Text overflow (shape too small for text) | PresentationML has `autoFit` attrs | ❌ | Should not affect extraction |
| P13 | Right-to-left text (`<a:rtl>`) | RTL text in `<a:t>` | ❌ | Text order preserved |
| P14 | Speaker notes (`notesSlide` part) | Notes text not extracted | ❌ | Separate notes extraction path |
| P15 | Slide layouts and masters | Inherited content from master | ❌ | At minimum, don't crash |

### 4.3 Excel (.xlsx) Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| X1 | Shared strings (`t="s"`) | Cells show as empty | ⚠️ | Gracefully skipped; needs SST resolution |
| X2 | Formula cells (`<f>`) | Formula text vs cached value | ❌ | Extract formula for `FormulaChanged`, show cached value |
| X3 | Numeric precision (1.200000000000001) | False diff on floating point | ❌ | Round to 12 significant digits |
| X4 | Date cells (stored as serial number) | Date shown as number | ❌ | Style-based date detection |
| X5 | Boolean cells (`t="b"`) | `"1"` vs `"TRUE"` | ❌ | Extract as "TRUE"/"FALSE" |
| X6 | Error cells (`t="e"`) | `#N/A`, `#VALUE!` as text | ❌ | Extract error string |
| X7 | Rich text cells (`t="str"` with `<r>` elements) | Formatted inline strings | ❌ | Extract text, ignore formatting |
| X8 | Empty cells in sparse sheet | Cell ordering, empty output | ✅ | Already tested |
| X9 | Very wide sheet (100+ columns) | Column letter conversion beyond ZZ | ❌ | `col_to_letter` should handle AAA |
| X10 | Very tall sheet (1M+ rows) | Performance, row indexing | ❌ | Benchmark threshold |
| X11 | Multiple sheets (Sheet1 + Sheet2 + Sheet3) | Sheet matching by name | ❌ | Match by name, detect renames |
| X12 | Sheet renamed | "Sheet1" → "Data" | ❌ | Report as `SheetRenamed` |
| X13 | Sheet added/deleted | Count mismatch | ❌ | Report Added/Deleted sections |
| X14 | Merged cells (`<mergeCells>`) | Cell reference confusion | ❌ | At minimum, don't crash |
| X15 | Conditional formatting | XML noise | ❌ | Should not affect extraction |
| X16 | Data validation / filters | XML noise | ❌ | Should not affect extraction |
| X17 | Named ranges | Not extracted | ❌ | Compare named range references |
| X18 | Pivot tables (`<pivotTableDefinition>`) | Cache vs source data | ❌ | At minimum, don't crash |
| X19 | External references / links | `externalLinks` directory | ❌ | At minimum, don't crash |
| X20 | Protected sheets / workbooks | XML metadata only | ❌ | Should not affect extraction |
| X21 | Array formulas (`<f t="array" ref="A1:A10">`) | Multi-cell formula output | ❌ | Extract formula, note array type |
| X22 | Data table (`<dataTable>`) | What-if analysis data | ❌ | At minimum, don't crash |

### 4.4 Cross-Format Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| C1 | Password-protected file (encrypted zip entry) | Decryption failure | ❌ | Graceful error, not panic |
| C2 | Legacy format (`.doc`, `.ppt`, `.xls`) | Not a zip — binary format | ❌ | UnknownFormat error, not InvalidZip |
| C3 | Macro-enabled format (`.docm`, `.pptm`, `.xlsm`) | Extension unrecognized | ❌ | Detect as their base format |
| C4 | Template format (`.dotx`, `.potx`, `.xltx`) | Extension unrecognized | ❌ | Detect as their base format |
| C5 | Zero-byte file | Zip parse failure | ❌ | Graceful error message |
| C6 | Truncated zip (last 100 bytes missing) | Partial read | ❌ | InvalidZip error, not panic |
| C7 | Embedded OLE object (nested zip/CFB) | Binary blob in zip | ❌ | Hash-compare, don't explode |
| C8 | Very large embedded image (50MB PNG) | Memory pressure | ❌ | Hash without loading full bytes (streaming) |
| C9 | File with BOM (`\u{FEFF}`) | XML parser confusion | ❌ | BOM should be handled by parser |
| C10 | Invalid UTF-8 in XML (Latin-1 encoded) | String conversion panic | ❌ | `from_utf8_lossy` or explicit encoding detection |
| C11 | Extremely long file path (>260 chars on Windows) | OS-specific path limits | ❌ | Should work on all platforms |
| C12 | Symlinked file | Path resolution | ❌ | Follow symlink or reject gracefully |

### 4.5 Security Edge Cases

| # | Case | Risk | Currently Tested? | Recommended Test |
|---|------|------|-------------------|------------------|
| S1 | Zip bomb (1MB → 1GB decompressed) | OOM / disk fill | ✅ | `max_part_size` + `MAX_TOTAL_UNCOMPRESSED` |
| S2 | Billion laughs XML entity expansion | XML parser hangs | ❌ | DTD disabled; explicit entity test |
| S3 | Zip with 100K small files | CPU exhaustion | ✅ | `max_part_count` limit |
| S4 | Path traversal in zip entries (`../../etc/passwd`) | File system escape | ❌ | Reject entries with `..` |
| S5 | Zip with overlapping entries (same name twice) | Duplicate key panic | ❌ | Last-write-wins or error |
| S6 | Extremely deep XML nesting (10K levels) | Stack overflow in depth tracking | ❌ | Configurable depth limit |

---

## 5. Test Gap Analysis

### Critical Gaps (Should Fix Before Release)

| Gap | Impact | Effort | Priority |
|-----|--------|--------|----------|
| **Word: deleted paragraph comparison** | Deleted paragraphs not tested in compare.rs | Small | P0 |
| **Tracked changes filtering** | `w:del` content shown as real content | Medium | P0 |
| **Billion-laughs XML entity expansion test** | DTD already disabled but no explicit regression test | Small | P1 |
| **PowerPoint comparison** | Entire PPTX diff pipeline missing | Large | P1 |
| **Excel comparison** | Entire XLSX diff pipeline missing | Large | P1 |
| **Excel shared strings** | Most real Excel files use SST | Medium | P1 |
| **Paragraph reordering detection** | Currently matches by index; misses moves | Large | P2 |

### Medium Gaps (Should Fix for Completeness)

| Gap | Impact | Effort | Priority |
|-----|--------|--------|----------|
| Unicode edge cases (emoji, CJK, RTL) | Display issues for non-Latin text | Small | P2 |
| Formula cell extraction | Real Excel often uses formulas | Small | P2 |
| Legacy/macro format detection | `.doc`, `.docm` etc. reject with bad error | Small | P2 |
| Path traversal in zip entries | Theoretical security concern | Small | P2 |
| Multiple table indexing | Untested with 3+ tables | Small | P2 |
| Table merged cells | Incorrect cell alignment | Medium | P3 |
| Empty document / empty paragraph | Edge case correctness | Small | P3 |
| Very long paragraph | Performance bug risk | Small | P3 |
| Word content controls (`<w:sdt>`) | Templates/forms use structured document tags | Small | P3 |
| PowerPoint speaker notes extraction | Notes pages have text content | Small | P3 |
| PowerPoint tables/charts in slides | Missing extraction | Medium | P3 |

### Nice-to-Have Gaps

| Gap | Impact | Effort | Priority |
|-----|--------|--------|----------|
| Hyperlink metadata extraction | Lost semantic info | Small | P4 |
| Image hash comparison | Image changes undetected | Medium | P4 |
| Slide reordering (ID-based matching) | False diffs on reorder | Medium | P4 |
| Concurrent access testing | Thread safety verification | Small | P4 |
| Field code filtering (PAGE, DATE) | Noise in extracted text | Small | P4 |
| Rich text in Excel cells | Formatting info lost | Small | P4 |
| Excel array formulas (`<f t="array">`) | Different from regular formulas | Small | P4 |
| Embedded OLE object comparison | Nested content ignored | Large | P5 |

---

## 6. Attack Surface: Things That Can Break

### During Refactoring

| Area | What Breaks | How We Catch It |
|------|------------|-----------------|
| `format.rs` signature change | `compare_office_files()` won't compile | Type system |
| `models.rs` field rename | JSON serialization changes | `test_word_compare_produces_real_results`, `test_serialize_*` tests |
| `models.rs` variant added to enum | Match exhaustiveness checks fail | Compiler |
| `extract.rs` depth tracking change | Table extraction panics or produces wrong output | `test_extract_*` inline tests |
| `compare.rs` paragraph matching change | Section count or status changes | `approach_3_*` tests in approaches_bench |
| `xml_diff.rs` event ordering change | XML comparison test assertions | `test_*` inline tests |
| `lib.rs` match arm reorder | Wrong comparison for format | `fixture_compare_office_files_phase1_stub` |

### During Dependency Upgrades

| Dependency | What Breaks | How We Catch It |
|-----------|------------|-----------------|
| `quick-xml` 0.37 → 0.40 | `QName` API changes, `read_text()` behavior | All XML parsing tests |
| `zip` 2.x → 3.x | `ZipArchive::new()` signature | All zip unpacking tests |
| `serde` major version | All serialization tests | Round-trip tests |
| `thiserror` → newer | Error display format changes | Error assertion tests |

### During New Feature Addition

| New Feature | Risk to Existing Code | Mitigation |
|-------------|----------------------|------------|
| PPTX semantic compare | Shared `OfficeSectionDiff` model changes | Add to models.rs, run all tests |
| Excel shared strings | Changes to `extract_sheet_text()` signature | Add optional param with default |
| Tracked changes filter | Extraction output changes | Configurable via `OfficeDiffOptions` |
| CLI integration | `compare_office_files()` called from new context | API is already public and tested |

### Known Brittle Areas

1. **Depth tracking in extract functions.** The most recurring bug class. Any change to `extract_text()` or `extract_table_text()` must be accompanied by table-heavy test cases.

2. **Namespace prefix matching via raw bytes.** A refactor that changes how prefixes are detected (e.g., switching to a real namespace resolver) would change behavior.

3. **`read_text()` consuming End events.** New contributors often miss this and introduce depth bugs. Well-documented, but fragile.

4. **Whitespace trimming.** The decision to NOT trim during extraction but trim at display time is a design rule, not enforced by the compiler. Easy to accidentally reintroduce trimming.

5. **Fixture binary files.** If someone deletes the `.docx/.pptx/.xlsx` files from `fixtures/cases/`, the tests regenerate them — but only if the XML templates haven't changed. A template change without deleting the binaries causes stale fixtures.

---

## 7. Test Strategy for Upcoming Features

### 7.1 PowerPoint Semantic Compare

**Before implementation:**
1. Create `fixtures/cases/office-pptx-multi-slide/` with 3 test cases:
   - Two identical 3-slide presentations
   - Slide 2 text changed
   - Slide added (left=2 slides, right=3 slides)
2. Define expected `case.json` format for PPTX metadata

**Implementation approach:**
1. Build `powerpoint/extract.rs::extract_presentation()` to parse ALL slides
2. Build `powerpoint/compare.rs::compare_presentations()` following `word/compare.rs` pattern
3. Add inline tests mirroring word/compare.rs structure:
   - `test_identical_presentations`
   - `test_slide_text_change`
   - `test_slide_added`
   - `test_slide_deleted`
   - `test_formatting_change` (if slide-level formatting is tracked)

**Integration:**
1. Wire into `lib.rs::compare_office_files()` for `OfficeFormat::PowerPoint`
2. Update `fixture_compare_office_files_phase1_stub` test
3. Add PPTX scenario to `approaches_bench.rs`

### 7.2 Excel Semantic Compare

**Before implementation:**
1. Create `fixtures/cases/office-xlsx-multi-sheet/` with test cases:
   - Two identical 2-sheet workbooks
   - Cell value changed (A1, no shared strings)
   - Sheet renamed
   - Sheet added/deleted
2. Build SST parser first (`excel/extract.rs::parse_shared_strings()`)
3. Add SST-dependent fixture cases

**Implementation approach:**
1. Build `excel/extract.rs::extract_workbook()` to parse ALL sheets + SST
2. Build `excel/compare.rs::compare_workbooks()` with sheet matching
3. Handle numeric precision (round to 12 sig figs before comparison)
4. Handle formula vs cached value distinction
5. Add inline tests:
   - `test_identical_workbooks`
   - `test_cell_text_change`
   - `test_sheet_renamed`
   - `test_sheet_added`
   - `test_formula_changed_value_same`
   - `test_numeric_precision`
   - `test_shared_string_resolution`

**Integration:**
1. Wire into `lib.rs::compare_office_files()` for `OfficeFormat::Excel`
2. Update `fixture_compare_office_files_phase1_stub` test
3. Add XLSX scenario to `approaches_bench.rs`

### 7.3 Tracked Changes Filtering

**Approach:**
1. Add `filter_tracked_changes: bool` to `OfficeDiffOptions` (default: `true`)
2. In `word/extract.rs::extract_text()` and `extract_document()`:
   - When entering `<w:del>`, set `in_deletion = true`, skip content
   - When entering `<w:ins>`, treat normally (content has been accepted)
   - Handle `<w:moveFrom>` / `<w:moveTo>` as their respective types
3. Track author/date metadata for each tracked change

**Tests:**
- `test_extract_text_skips_deletions` — `<w:del>` content excluded
- `test_extract_text_includes_insertions` — `<w:ins>` content included
- `test_extract_text_tracked_changes_opt_out` — when `filter_tracked_changes = false`, all content included
- `test_extract_document_skips_deleted_paragraphs`
- `test_compare_cleansed_documents` — both sides filtered, comparison is clean

### 7.4 CLI Integration

**Approach:**
1. In `diff-cli/src/main.rs`, detect Office extensions in file args
2. Route to `office_adapter::compare_office_files()`
3. Add `--human` rendering for `OfficeDiffResult`
4. Add `--json` rendering for `OfficeDiffResult`
5. Add `--sheets` flag for Excel
6. Add `--include-formatting` flag mapping to `OfficeDiffOptions`

**Tests (e2e):**
1. `tests/e2e/office_diff_cli.test.mjs` — CLI smoke test with real files
2. Snapshot tests in `diff-cli/tests/snapshots/` for JSON output

---

## 8. Test Commands & Automation

### Running Tests

```bash
# All office-adapter tests (82 tests across 4 binaries)
cargo test -p office-adapter

# Individual test binaries
cargo test -p office-adapter --lib                          # 54 unit tests
cargo test -p office-adapter --test approaches_bench         # 12 integration tests
cargo test -p office-adapter --test fixture_cases            # 16 fixture tests

# With output (for approaches_bench which prints comparisons)
cargo test -p office-adapter --test approaches_bench -- --nocapture

# Specific test
cargo test -p office-adapter test_extract_mixed_paragraphs_and_table
cargo test -p office-adapter fixture_compare_office_files_phase1_stub

# Regenerate fixture binaries (delete existing ones first)
rm fixtures/cases/office-word-basic/left.docx
cargo test -p office-adapter --test fixture_cases

# Full project check (includes diff-core, diff-cli, git-adapter)
./scripts/check.sh
```

### Adding a New Fixture Case

1. Create directory: `fixtures/cases/office-my-case/`
2. Create `case.json`:
   ```json
   {
     "name": "office-my-case",
     "format": "word",
     "expected": {
       "sections_total": 2,
       "sections_added": 0,
       "sections_deleted": 0,
       "sections_changed": 1,
       "sections_unchanged": 1,
       "total_atomic_changes": 1
     }
   }
   ```
3. Add fixture body templates to `tests/common/mod.rs`
4. Add generation logic to `ensure_fixtures()` in `fixture_cases.rs`
5. Binary files will be generated on first test run

### Adding a New Approach Comparison Scenario

1. Add body templates to `tests/common/mod.rs`
2. Add a `Scenario` entry in `approaches_bench.rs::scenarios()`
3. Optionally add an assertion test

### Adding a New Inline Test

Follow existing patterns in the module's `#[cfg(test)] mod tests` block:
- Use hand-authored XML strings (minimal, readable)
- Test both success and edge cases
- For extraction: assert on exact text output
- For comparison: assert on `OfficeSectionDiff` status, kind, changes

---

## 9. Performance Regression Testing

### Current Baseline

> From `agents.md` and initial runs. Not yet specific to office-adapter.

### Office-Specific Performance Concerns

| Scenario | Expected Performance | Risk if Degraded |
|----------|---------------------|------------------|
| Extract 100-paragraph docx | < 5ms | Extracting large Word docs |
| Extract 50-slide pptx | < 20ms | Large presentations |
| Extract 10K-cell xlsx | < 50ms | Large spreadsheets |
| Compare 100-paragraph diff | < 10ms | Diffing large documents |
| Unpack 100MB zip | < 500ms | Large embedded media |
| Parse 10K shared strings | < 20ms | Excel SST parsing |

### Adding Performance Benchmarks

```bash
# Future: add office-specific benchmarks
cargo bench -p office-adapter
```

Recommended benchmark tests (in `crates/office-adapter/benches/`):
- `extract_word_100_paragraphs` — Extraction throughput
- `extract_excel_10k_cells` — Cell extraction throughput
- `extract_pptx_50_slides` — Slide extraction throughput
- `compare_word_identical` — Baseline comparison speed
- `compare_word_every_para_changed` — Worst-case comparison
- `unpack_large_zip` — Zip reading with size limits

---

## 10. Documentation & Handoff

### Test Documentation Standards

1. **Every `pub fn` should have a doc comment.** Extraction and comparison functions must document their XML assumptions.

2. **Every fixture case should have a `case.json` with `name`, `format`, and `expected` fields.** This is the contract for fixture discovery.

3. **Approach comparison scenarios must have a `description` field** explaining what aspect of the approach trade-offs the scenario demonstrates.

4. **Known limitations must be tracked in this document** (see Section 5: Test Gap Analysis). When a gap is filled, update the matrix.

### When Adding New Code

| Code Added | Minimum Tests |
|-----------|--------------|
| New `pub fn` in extract module | 2+ tests (simple + edge case) |
| New `pub fn` in compare module | 3+ tests (identical + changed + added/deleted) |
| New model type | Round-trip serialization test |
| New error variant | Error creation + matching test |
| New match arm in `lib.rs` | Fixture test exercising that arm |
| New fixture scenario | `case.json` + body templates + assertion test |

### Review Checklist for Test PRs

- [ ] Do all 82 existing tests still pass?
- [ ] Does the new test exercise a gap in the coverage matrix?
- [ ] Is the test output stable (no flaky ordering or timing dependencies)?
- [ ] For extraction tests: is the exact text output asserted?
- [ ] For comparison tests: are status, kind, and changes all asserted?
- [ ] For fixture tests: is `case.json` present with `format` field?
- [ ] Are fixture binary files on disk and valid zip archives?
- [ ] Does `./scripts/check.sh` pass?

---

## Appendix A: Quick Reference — Test Count by Module

| Module | Inline Tests |
|--------|-------------|
| `format.rs` | 7 |
| `unpack.rs` | 6 |
| `xml_util.rs` | 2 |
| `models.rs` | 3 |
| `lib.rs` | 3 |
| `word/extract.rs` | 6 |
| `word/compare.rs` | 6 |
| `word/xml_diff.rs` | 5 |
| `powerpoint/extract.rs` | 7 |
| `excel/extract.rs` | 8 |
| `tests/common/mod.rs` | 3 |
| **Total inline** | **56** |
| `tests/approaches_bench.rs` | 12 |
| `tests/fixture_cases.rs` | 16 |
| **Total integration** | **28** |
| **Grand total** | **82** (54 lib + 12 approaches_bench + 16 fixture_cases) |

*Note: The `tests/common/mod.rs` has 3 `#[test]` functions in its own `#[cfg(test)] mod tests` block, but `mod common` inlines them into both integration test binaries (approaches_bench + fixture_cases), so they run as part of those 28 tests, not as a separate count. The 4th test binary ("0 tests") is the common module compiled as its own test target, which has no standalone tests.*

## Appendix B: Future E2E Test Plan

Once CLI integration is complete, add these e2e tests:

```javascript
// tests/e2e/office-diff-cli.test.mjs

// 1. Basic docx diff
//    $ dt diff left.docx right.docx --human
//    → Expect: "Paragraph 1: text changed from X to Y"

// 2. JSON output
//    $ dt diff left.docx right.docx --json
//    → Expect: valid JSON with OfficeDiffResult schema

// 3. Format mismatch error
//    $ dt diff file.docx file.pptx
//    → Expect: error message about mismatched formats

// 4. Unknown format
//    $ dt diff file.xyz file.xyz
//    → Expect: error message about unknown format

// 5. Corrupt file
//    $ dt diff corrupt.docx valid.docx
//    → Expect: error message about invalid zip

// 6. Excel with --sheets flag
//    $ dt diff workbook1.xlsx workbook2.xlsx --sheets "Sheet1,Sheet3"
//    → Expect: only specified sheets compared

// 7. Exit codes
//    $ dt diff identical.docx identical.docx; echo $?
//    → Expect: 0 (no differences)
//    $ dt diff different.docx different.docx; echo $?
//    → Expect: 1 (differences found)
//    $ dt diff corrupt.docx valid.docx; echo $?
//    → Expect: 2 (error)
```
