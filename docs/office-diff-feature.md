# Office Document Diff Feature

## Summary

Add first-class comparison for Microsoft Office files:

- Word: `.docx`
- PowerPoint: `.pptx`
- Excel: `.xlsx`

The goal is not to clone Microsoft Office. The goal is to make Office changes
reviewable inside the same diff workflow as code, folder, image, and Git
changes.

## Product USP

Most Office comparison tools are either:

- tied to Microsoft Office and Windows,
- legal-document focused,
- Excel-only,
- cloud/API-first,
- or disconnected from developer review workflows.

This app can differentiate by making Office files part of a local, cross-
platform, evidence-linked review queue:

- review `.docx`, `.pptx`, `.xlsx`, source code, images, and config files in one
  workspace;
- group Office changes by intent, slide/sheet/section, and risk;
- produce machine-readable change summaries for agents and CI;
- keep comparisons local by default.

## Supported Formats

### Word

Initial `.docx` support should compare:

- paragraph text;
- headings and outline structure;
- tables;
- comments;
- tracked changes presence;
- headers and footers;
- footnotes/endnotes if extraction support is available;
- embedded image presence and dimensions as metadata.

V1 output should prioritize a structured change list over a pixel-perfect
redline document.

### PowerPoint

Initial `.pptx` support should compare:

- slide added/deleted/reordered;
- slide title and speaker notes;
- text box contents;
- shape count and coarse geometry;
- image presence, size, and checksum;
- table text;
- chart presence and source data summary when available.

V1 output should show slide-level cards with a visual thumbnail later, but the
first backend model can be text and metadata only.

### Excel

Initial `.xlsx` support should compare:

- workbook sheets added/deleted/renamed/reordered;
- used range size;
- cell values;
- formulas;
- cell comments/notes;
- named ranges;
- row/column insertions and deletions;
- basic formatting changes if cheap to extract.

V1 should support sheet/range filters so large workbooks do not force a full
comparison.

## Data Model Additions

Add an Office comparison model separate from text diffs:

```rust
OfficeDiffRequest {
    left: PathBuf,
    right: PathBuf,
    format: OfficeFormat,
    options: OfficeDiffOptions,
}

OfficeDiffResult {
    format: OfficeFormat,
    summary: OfficeDiffSummary,
    sections: Vec<OfficeSectionDiff>,
    warnings: Vec<String>,
    unsupported: Vec<String>,
}

OfficeSectionDiff {
    id: String,
    label: String,
    kind: OfficeSectionKind,
    status: OfficeChangeStatus,
    changes: Vec<OfficeAtomicChange>,
}

OfficeAtomicChange {
    path: String,
    kind: OfficeAtomicChangeKind,
    before: Option<String>,
    after: Option<String>,
    metadata: serde_json::Value,
}
```

Keep this model serializable and UI-neutral. The same model should feed CLI,
desktop UI, future agent summaries, and CI reports.

## Architecture

Add a new Rust crate:

```text
crates/office-adapter/
```

Responsibilities:

- detect Office format;
- unpack Open Packaging Convention zip files safely;
- parse relevant XML parts;
- normalize document-specific structures;
- emit `OfficeDiffResult`;
- never require Microsoft Office to be installed.

Recommended implementation sequence:

1. Add format detection and safe zip reading for `.docx`, `.pptx`, `.xlsx`.
2. Extract plain structured text from each format.
3. Compare document sections using existing `diff-core`.
4. Add Office-specific structural changes.
5. Add thumbnails/previews later, probably in a separate renderer path.

## CLI

Add:

```bash
dt office left.docx right.docx --json
dt office left.pptx right.pptx --json
dt office left.xlsx right.xlsx --sheet Sheet1 --json
```

Human output should be a compact summary:

```text
Word document: 3 sections changed, 12 text changes, 1 table changed
PowerPoint: 2 slides changed, 1 slide added
Excel: 4 sheets compared, 18 cells changed, 2 formulas changed
```

## Desktop UI

Add an Office compare mode with format-specific navigation:

- Word: outline/sidebar by heading or paragraph region.
- PowerPoint: slide list with per-slide change badges.
- Excel: sheet list, grid-like change summary, range filters.

The first UI can render structured cards and text snippets. Pixel-perfect Office
rendering and redline export are later milestones.

## Test Plan

Create fixtures under:

```text
fixtures/office/
  word-basic/
  powerpoint-basic/
  excel-basic/
```

Initial fixtures:

- Word: heading changed, paragraph inserted, table cell changed.
- PowerPoint: slide title changed, slide added, notes changed.
- Excel: cell value changed, formula changed, sheet added.

Tests:

- format detection;
- parse succeeds on valid files;
- parse fails safely on invalid zip/XML;
- structured extraction is deterministic;
- expected section/change counts match fixture metadata;
- CLI JSON output is parseable.

## Performance And Safety

- Enforce max uncompressed zip size and max part count.
- Avoid loading large binary embedded media into memory unless requested.
- Hash images and embedded files incrementally.
- Add workbook range filters early for Excel.
- Record parse time and comparison time separately in future instrumentation.

## Status

**⚠️ EXPERIMENTAL — Parked for MVP**

The `office-adapter` crate implements format detection, safe zip reading, XML
extraction, and structured comparison for all three Office formats. It passes
tests against hand-authored fixtures and has zero compiler warnings. However,
it is **not exposed** in the CLI or desktop UI for the MVP release.

Office support will be activated in a post-MVP release after:
- Real-world fixture coverage
- CLI integration (`dt office`)
- Desktop UI with format-specific views
- Shared-string (Excel) and image comparison
- Performance benchmarks on real documents

## TODO

- [x] Add `crates/office-adapter`.
- [ ] Add `dt office` CLI placeholder.
- [x] Add hand-authored minimal `.docx`, `.pptx`, and `.xlsx` fixtures.
- [x] Add format detection and safe zip guards.
- [x] Add structured text extraction for Word.
- [x] Add slide/text extraction for PowerPoint.
- [x] Add workbook/sheet/cell extraction for Excel.
- [x] Add Office result model and JSON tests.
- [ ] Add desktop Office compare route.
- [ ] Add market-informed ADR after choosing parser/rendering libraries.
