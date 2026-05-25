//! Shared data models for Office document comparison.
//!
//! These types are serializable and UI-neutral. They are designed to feed
//! CLI output, desktop UI rendering, and AI enrichment pipelines.

use serde::{Deserialize, Serialize};

// ── Format ─────────────────────────────────────────────────────────────

/// Identifies an Office document format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OfficeFormat {
    Word,
    PowerPoint,
    Excel,
}

// ── Change status ──────────────────────────────────────────────────────

/// The change status of a document section or atomic change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeStatus {
    Added,
    Deleted,
    Modified,
    Unchanged,
    Reordered,
}

// ── Section kinds ──────────────────────────────────────────────────────

/// The kind of a document section being compared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

// ── Atomic change kinds ────────────────────────────────────────────────

/// The kind of an individual atomic change within a section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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

// ── Options ────────────────────────────────────────────────────────────

/// Options controlling Office document comparison behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeDiffOptions {
    /// For Excel: only compare these sheets (empty = all).
    pub sheets: Vec<String>,
    /// For Excel: only compare cells within this range (e.g., "A1:Z100").
    pub range: Option<String>,
    /// Maximum uncompressed size per part, in bytes. Default: 50 MB.
    pub max_part_size: u64,
    /// Maximum number of parts to process. Default: 10,000.
    pub max_part_count: usize,
    /// Include embedded image metadata (presence, dimensions, hash).
    pub include_image_metadata: bool,
    /// Include formatting change detection.
    pub include_formatting: bool,
}

impl Default for OfficeDiffOptions {
    fn default() -> Self {
        Self {
            sheets: Vec::new(),
            range: None,
            max_part_size: 50 * 1024 * 1024, // 50 MB
            max_part_count: 10_000,
            include_image_metadata: true,
            include_formatting: false,
        }
    }
}

// ── Result types ───────────────────────────────────────────────────────

/// Top-level result for an Office document comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficeDiffResult {
    pub format: OfficeFormat,
    pub summary: OfficeDiffSummary,
    pub sections: Vec<OfficeSectionDiff>,
    pub warnings: Vec<String>,
    pub unsupported_features: Vec<String>,
}

/// High-level change summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficeDiffSummary {
    pub sections_total: usize,
    pub sections_added: usize,
    pub sections_deleted: usize,
    pub sections_changed: usize,
    pub sections_unchanged: usize,
    pub total_atomic_changes: usize,
}

/// Diff for a single document section (paragraph, slide, sheet, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficeSectionDiff {
    pub id: String,
    pub label: String,
    pub kind: OfficeSectionKind,
    pub status: ChangeStatus,
    pub changes: Vec<OfficeAtomicChange>,
}

/// A single atomic change within a section.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

// ── Format-specific document models ────────────────────────────────────

/// Extracted Word document model (Phase 2 — currently a stub).
#[derive(Debug, Clone, Default)]
pub struct WordDocument {
    pub paragraphs: Vec<WordParagraph>,
    pub tables: Vec<WordTable>,
    pub comments: Vec<WordComment>,
}

#[derive(Debug, Clone)]
pub struct WordParagraph {
    pub index: usize,
    pub text: String,
    pub style: Option<String>,
    pub heading_level: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct WordTable {
    pub index: usize,
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct WordComment {
    pub id: String,
    pub author: String,
    pub text: String,
}

/// Extracted PowerPoint presentation model (Phase 2 — currently a stub).
#[derive(Debug, Clone, Default)]
pub struct Presentation {
    pub slides: Vec<PowerPointSlide>,
}

#[derive(Debug, Clone)]
pub struct PowerPointSlide {
    pub id: String,
    pub index: usize,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub text_boxes: Vec<String>,
    pub shape_count: usize,
    pub image_count: usize,
}

/// Extracted Excel workbook model (Phase 2 — currently a stub).
#[derive(Debug, Clone, Default)]
pub struct Workbook {
    pub sheets: Vec<ExcelSheet>,
    pub named_ranges: Vec<ExcelNamedRange>,
}

#[derive(Debug, Clone)]
pub struct ExcelSheet {
    pub name: String,
    pub index: usize,
    pub used_rows: u32,
    pub used_cols: u32,
    pub cells: Vec<ExcelCell>,
}

#[derive(Debug, Clone)]
pub struct ExcelCell {
    pub row: u32,
    pub col: u32,
    pub value: Option<String>,
    pub formula: Option<String>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExcelNamedRange {
    pub name: String,
    pub reference: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_office_diff_options_defaults() {
        let opts = OfficeDiffOptions::default();
        assert_eq!(opts.max_part_size, 50 * 1024 * 1024);
        assert_eq!(opts.max_part_count, 10_000);
        assert!(opts.include_image_metadata);
        assert!(!opts.include_formatting);
        assert!(opts.sheets.is_empty());
        assert!(opts.range.is_none());
    }

    #[test]
    fn test_office_diff_result_serialization() {
        let result = OfficeDiffResult {
            format: OfficeFormat::Word,
            summary: OfficeDiffSummary {
                sections_total: 1,
                sections_added: 0,
                sections_deleted: 0,
                sections_changed: 1,
                sections_unchanged: 0,
                total_atomic_changes: 1,
            },
            sections: vec![OfficeSectionDiff {
                id: "para-3".to_owned(),
                label: "Section 3".to_owned(),
                kind: OfficeSectionKind::WordParagraph,
                status: ChangeStatus::Modified,
                changes: vec![OfficeAtomicChange {
                    path: "para=3".to_owned(),
                    kind: OfficeAtomicChangeKind::TextChanged,
                    before: Some("Q1 2025".to_owned()),
                    after: Some("Q2 2025".to_owned()),
                    metadata: serde_json::Value::Null,
                }],
            }],
            warnings: Vec::new(),
            unsupported_features: Vec::new(),
        };

        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("word"));
        assert!(json.contains("Q1 2025"));
        assert!(json.contains("Q2 2025"));

        // Round-trip.
        let parsed: OfficeDiffResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.format, OfficeFormat::Word);
        assert_eq!(parsed.sections.len(), 1);
    }

    #[test]
    fn test_change_status_serde() {
        let statuses = vec![
            ChangeStatus::Added,
            ChangeStatus::Deleted,
            ChangeStatus::Modified,
            ChangeStatus::Unchanged,
            ChangeStatus::Reordered,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let parsed: ChangeStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, parsed);
        }
    }
}
