# Office Document Diff Feature

## Summary

Add first-class comparison for Microsoft Office files:

- Word: `.docx`
- PowerPoint: `.pptx`
- Excel: `.xlsx`

The goal is not to clone Microsoft Office. The goal is to make Office changes
reviewable inside the same diff workflow as code, folder, image, and Git
changes — without requiring Microsoft Office to be installed.

## How It Works

Office files in the modern Open XML format (`.docx`, `.pptx`, `.xlsx`) are
zip archives containing XML parts. The `office-adapter` crate:

1. Detects the format from the file extension and zip structure.
2. Unpacks the zip archive safely (size limits, part count limits).
3. Parses relevant XML parts into structured document models.
4. Compares left and right document models section-by-section.
5. Emits an `OfficeDiffResult` with structured change details.

No Microsoft Office installation is required. The comparison is entirely
offline and deterministic.

## Supported Formats

### Word (`.docx`)

V1 extraction and comparison targets:

- **Paragraph text** — body content, including runs with formatting spans
- **Headings and outline structure** — heading levels 1–9, document outline tree
- **Tables** — cell text content, row/column counts, merged cell detection
- **Comments** — comment text, author, and anchor location
- **Tracked changes** — presence detection (insertions, deletions, formatting changes)
- **Headers and footers** — section-level header/footer text
- **Footnotes and endnotes** — if extraction support is available
- **Embedded images** — presence, dimensions, and content hash (not pixel diff)

V1 output prioritizes a structured change list over a pixel-perfect redline
document. The backend model captures *what* changed (paragraph 3 text changed,
table 2 row added, heading "Overview" renamed to "Summary") rather than
producing a formatted redline `.docx`.

### PowerPoint (`.pptx`)

V1 extraction and comparison targets:

- **Slide added/deleted/reordered** — slide-level structural diff
- **Slide title** — title shape text
- **Speaker notes** — notes slide text content
- **Text box contents** — all text shapes, including multi-paragraph
- **Shape count and coarse geometry** — shape type, position, and size
- **Image presence, size, and content hash**
- **Table text** — cell contents for embedded tables
- **Chart presence and source data summary** — when embedded data is available

V1 output shows slide-level cards with text and metadata changes. Visual
thumbnails of slides are a later milestone.

### Excel (`.xlsx`)

V1 extraction and comparison targets:

- **Sheets** — added, deleted, renamed, reordered
- **Used range size** — dimensions of the populated cell area
- **Cell values** — string, number, boolean, and date values
- **Formulas** — formula text (not computed results)
- **Cell comments and notes**
- **Named ranges** — name, scope, and reference
- **Row/column insertions and deletions** — structural grid changes
- **Basic formatting** — number format, font, fill, border (if cheap to extract)

V1 must support **sheet and range filters** so large workbooks (100k+ cells)
do not force a full comparison. Users should be able to specify:
`--sheet Sheet1 --range A1:Z100`.

## Data Models

### Core Types

```rust
/// Identifies an Office document format.
pub enum OfficeFormat {
    Word,
    PowerPoint,
    Excel,
}

/// Top-level request for comparing two Office documents.
pub struct OfficeDiffRequest {
    pub left: PathBuf,
    pub right: PathBuf,
    pub format: OfficeFormat,
    pub options: OfficeDiffOptions,
}

/// Options controlling comparison behavior.
pub struct OfficeDiffOptions {
    /// For Excel: only compare these sheets (empty = all).
    pub sheets: Vec<String>,
    /// For Excel: only compare cells within this range (e.g., "A1:Z100").
    pub range: Option<String>,
    /// Maximum uncompressed size per part, in bytes.
    pub max_part_size: usize,
    /// Maximum number of parts to process.
    pub max_part_count: usize,
    /// Include embedded image metadata (presence, dimensions, hash).
    pub include_image_metadata: bool,
    /// Include formatting change detection.
    pub include_formatting: bool,
}
```

### Result Model

```rust
/// Top-level result for an Office document comparison.
pub struct OfficeDiffResult {
    pub format: OfficeFormat,
    pub summary: OfficeDiffSummary,
    pub sections: Vec<OfficeSectionDiff>,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}

/// High-level change summary.
pub struct OfficeDiffSummary {
    pub sections_total: usize,
    pub sections_added: usize,
    pub sections_deleted: usize,
    pub sections_changed: usize,
    pub sections_unchanged: usize,
    pub total_atomic_changes: usize,
}

/// Diff for a single document section (paragraph, slide, sheet, etc.).
pub struct OfficeSectionDiff {
    pub id: String,
    pub label: String,
    pub kind: OfficeSectionKind,
    pub status: ChangeStatus,
    pub changes: Vec<OfficeAtomicChange>,
}

pub enum OfficeSectionKind {
    WordParagraph,
    WordHeading,
    WordTable,
    WordComment,
    WordHeader,
    WordFooter,
    PowerPointSlide,
    PowerPointNotes,
    ExcelSheet,
    ExcelNamedRange,
}

pub enum ChangeStatus {
    Added,
    Deleted,
    Modified,
    Unchanged,
    Reordered,
}

/// A single atomic change within a section.
pub struct OfficeAtomicChange {
    /// Path-like identifier within the section (e.g., "row=5,col=C",
    /// "slide=3,shape=title", "para=7,run=2").
    pub path: String,
    pub kind: OfficeAtomicChangeKind,
    pub before: Option<String>,
    pub after: Option<String>,
    /// Additional type-specific metadata (formatting details, shape type, etc.).
    pub metadata: serde_json::Value,
}

pub enum OfficeAtomicChangeKind {
    TextChanged,
    TextAdded,
    TextDeleted,
    StructureChanged,
    FormattingChanged,
    ImageChanged,
    FormulaChanged,
    SheetRenamed,
    SlideReordered,
}
```

### Design Principles

- **Serializable and UI-neutral.** The same model feeds CLI, desktop UI,
  AI summaries, and CI reports.
- **Deterministic.** Same inputs always produce the same output.
- **Extensible.** `metadata: serde_json::Value` allows format-specific
  enrichment without changing the core model.
- **Separate from text diffs.** Office documents have structural semantics
  (slides, sheets, paragraphs) that don't map cleanly to line-oriented diffs.

## CLI

Add `dt office` subcommand:

```bash
# Word
dt office left.docx right.docx --json
dt office left.docx right.docx           # human summary

# PowerPoint
dt office left.pptx right.pptx --json

# Excel
dt office left.xlsx right.xlsx --sheet Sheet1 --range A1:Z100 --json
dt office left.xlsx right.xlsx --sheet "Budget,Forecast" --json
```

### Human Output Format

```
Word document: 3 sections changed, 12 text changes, 1 table changed

  [Modified] §3 "Project Overview"
    - Text changed: "Q1 2025" → "Q2 2025"
    - Text changed: paragraph 2, 34 words changed

  [Modified] §7 "Budget"
    - Table changed: "Cost Summary" — 2 cells changed, 1 row added
    - Text changed: paragraph 1, 12 words changed

  [Modified] Comments
    - Text added: "Review the Q2 numbers before publishing — @alex"

PowerPoint: 2 slides changed, 1 slide added

  [Added] Slide 5: "Roadmap"
  [Modified] Slide 2: "Executive Summary"
    - Title changed: "Q1 Summary" → "Q2 Summary"
  [Modified] Slide 8: "Financials"
    - Table changed: "Revenue by Region" — 3 cells changed
```

## Test Plan

### Fixtures

Create under `fixtures/office/`:

```
fixtures/office/
  word-basic/
    left.docx
    right.docx
    expected.json        # expected OfficeDiffResult
  powerpoint-basic/
    left.pptx
    right.pptx
    expected.json
  excel-basic/
    left.xlsx
    right.xlsx
    expected.json
  excel-large/
    left.xlsx
    right.xlsx
    expected.json        # range-filtered
```

### Test Cases

| Test | Description |
|------|-------------|
| Format detection | Correctly identifies `.docx`, `.pptx`, `.xlsx` from zip structure |
| Valid parse | Parse succeeds on well-formed Office files |
| Invalid input | Parse fails safely on corrupt zip, invalid XML, truncated files |
| Deterministic output | Same inputs always produce identical JSON |
| Section counts | Expected section/change counts match fixture metadata |
| CLI JSON output | `dt office --json` produces parseable, valid JSON |
| CLI human output | Human mode shows readable summary |
| Large workbook | Excel range filter prevents full comparison |
| Binary media | Embedded images produce hashes, not pixel data |
| Size limits | Files exceeding max_part_size/max_part_count fail gracefully |

## Performance And Safety

- **Zip bomb protection.** Enforce `max_part_size` (default: 50 MB per part)
  and `max_part_count` (default: 10,000 parts).
- **Avoid loading embedded media.** Hash images and embedded files
  incrementally without loading full binary data into memory.
- **Excel range filters.** Apply sheet/range filters before extracting cell
  data to avoid loading unused regions.
- **Instrumentation.** Record parse time and comparison time separately for
  future performance tracking.
- **Streaming extraction.** When possible, stream XML parts rather than
  buffering entire documents in memory.
