//! Semantic Word document comparison (Approach 3: Semantic Model).
//!
//! Compares two Word documents at the semantic level: paragraphs, tables,
//! formatting, and structure. Uses the structured `WordDocument` model
//! and produces `OfficeSectionDiff` entries that can be rendered by the
//! CLI, desktop UI, and AI pipelines.
//!
//! This is the recommended primary approach for user-facing diffs because
//! it produces the most meaningful, user-friendly output.

use crate::models::*;
use crate::word::extract;
use crate::xml_util::XmlResult;

/// Compare two Word document XMLs semantically.
///
/// 1. Extracts structured `WordDocument` models from both.
/// 2. Matches paragraphs and tables by index.
/// 3. Compares text, formatting, and structure.
/// 4. Returns `Vec<OfficeSectionDiff>` with rich metadata.
pub fn compare_documents(left_xml: &[u8], right_xml: &[u8]) -> XmlResult<Vec<OfficeSectionDiff>> {
    let left_doc = extract::extract_document(left_xml)?;
    let right_doc = extract::extract_document(right_xml)?;

    let mut sections = Vec::new();

    // Compare paragraphs.
    compare_paragraphs(&left_doc.paragraphs, &right_doc.paragraphs, &mut sections);

    // Compare tables.
    compare_tables(&left_doc.tables, &right_doc.tables, &mut sections);

    Ok(sections)
}

/// Compare two lists of paragraphs by index.
fn compare_paragraphs(
    left: &[WordParagraph],
    right: &[WordParagraph],
    sections: &mut Vec<OfficeSectionDiff>,
) {
    let max_len = left.len().max(right.len());

    for i in 0..max_len {
        let left_para = left.get(i);
        let right_para = right.get(i);

        let (status, changes) = match (left_para, right_para) {
            (Some(_lp), None) => {
                // Paragraph deleted.
                (ChangeStatus::Deleted, vec![deleted_para_change(i, left_para)])
            }
            (None, Some(_rp)) => {
                // Paragraph added.
                (ChangeStatus::Added, vec![added_para_change(i, right_para)])
            }
            (Some(lp), Some(rp)) => {
                let mut c = Vec::new();

                // Text change.
                if lp.text != rp.text {
                    c.push(OfficeAtomicChange {
                        path: format!("para={}", i),
                        kind: OfficeAtomicChangeKind::TextChanged,
                        before: Some(lp.text.clone()),
                        after: Some(rp.text.clone()),
                        metadata: serde_json::Value::Null,
                    });
                }

                // Style / heading change.
                if lp.style != rp.style {
                    c.push(OfficeAtomicChange {
                        path: format!("para={}", i),
                        kind: OfficeAtomicChangeKind::FormattingChanged,
                        before: lp.style.clone(),
                        after: rp.style.clone(),
                        metadata: serde_json::json!({"property": "style"}),
                    });
                }

                let status = if c.is_empty() {
                    ChangeStatus::Unchanged
                } else {
                    ChangeStatus::Modified
                };

                (status, c)
            }
            (None, None) => unreachable!(),
        };

        let label = match (left_para, right_para) {
            (Some(p), _) | (_, Some(p)) => {
                let truncated: String = p.text.chars().take(60).collect();
                if truncated.len() < p.text.len() {
                    format!("Paragraph {}: {}...", i + 1, truncated)
                } else if p.text.is_empty() {
                    format!("Paragraph {} (empty)", i + 1)
                } else {
                    format!("Paragraph {}: {}", i + 1, truncated)
                }
            }
            _ => format!("Paragraph {}", i + 1),
        };

        let kind = left_para
            .or(right_para)
            .and_then(|p| {
                if p.heading_level.is_some() {
                    Some(OfficeSectionKind::WordHeading)
                } else {
                    Some(OfficeSectionKind::WordParagraph)
                }
            })
            .unwrap_or(OfficeSectionKind::WordParagraph);

        sections.push(OfficeSectionDiff {
            id: format!("para-{}", i),
            label,
            kind,
            status,
            changes,
        });
    }
}

/// Compare two lists of tables by index.
fn compare_tables(
    left: &[WordTable],
    right: &[WordTable],
    sections: &mut Vec<OfficeSectionDiff>,
) {
    let max_len = left.len().max(right.len());

    for i in 0..max_len {
        let left_table = left.get(i);
        let right_table = right.get(i);

        let (status, changes) = match (left_table, right_table) {
            (Some(_lt), None) => (ChangeStatus::Deleted, Vec::new()),
            (None, Some(_rt)) => (ChangeStatus::Added, Vec::new()),
            (Some(lt), Some(rt)) => {
                let mut c = Vec::new();

                // Compare table dimensions.
                if lt.rows != rt.rows || lt.cols != rt.cols {
                    c.push(OfficeAtomicChange {
                        path: format!("table={}", i),
                        kind: OfficeAtomicChangeKind::StructureChanged,
                        before: Some(format!("{}r×{}c", lt.rows, lt.cols)),
                        after: Some(format!("{}r×{}c", rt.rows, rt.cols)),
                        metadata: serde_json::Value::Null,
                    });
                }

                // Compare cell contents.
                let max_rows = lt.rows.max(rt.rows);
                let max_cols = lt.cols.max(rt.cols);
                for r in 0..max_rows {
                    for col in 0..max_cols {
                        let lc = lt.cells.get(r).and_then(|row| row.get(col));
                        let rc = rt.cells.get(r).and_then(|row| row.get(col));

                        match (lc, rc) {
                            (Some(lv), Some(rv)) if lv != rv => {
                                let col_letter =
                                    ((col as u8 + b'A') as char).to_string();
                                c.push(OfficeAtomicChange {
                                    path: format!("table={},cell={}{}", i, col_letter, r + 1),
                                    kind: OfficeAtomicChangeKind::TextChanged,
                                    before: Some(lv.clone()),
                                    after: Some(rv.clone()),
                                    metadata: serde_json::Value::Null,
                                });
                            }
                            (Some(_), None) => {
                                c.push(OfficeAtomicChange {
                                    path: format!("table={},cell={}{}", i, col_letter(col), r + 1),
                                    kind: OfficeAtomicChangeKind::TextDeleted,
                                    before: lc.cloned(),
                                    after: None,
                                    metadata: serde_json::Value::Null,
                                });
                            }
                            (None, Some(_)) => {
                                c.push(OfficeAtomicChange {
                                    path: format!("table={},cell={}{}", i, col_letter(col), r + 1),
                                    kind: OfficeAtomicChangeKind::TextAdded,
                                    before: None,
                                    after: rc.cloned(),
                                    metadata: serde_json::Value::Null,
                                });
                            }
                            _ => {} // Both None or both same.
                        }
                    }
                }

                let status = if c.is_empty() {
                    ChangeStatus::Unchanged
                } else {
                    ChangeStatus::Modified
                };

                (status, c)
            }
            (None, None) => unreachable!(),
        };

        let label = format!("Table {}", i + 1);

        sections.push(OfficeSectionDiff {
            id: format!("table-{}", i),
            label,
            kind: OfficeSectionKind::WordTable,
            status,
            changes,
        });
    }
}

/// Build a deletion atomic change for a removed paragraph.
fn deleted_para_change(idx: usize, para: Option<&WordParagraph>) -> OfficeAtomicChange {
    OfficeAtomicChange {
        path: format!("para={}", idx),
        kind: OfficeAtomicChangeKind::TextDeleted,
        before: para.map(|p| p.text.clone()),
        after: None,
        metadata: serde_json::Value::Null,
    }
}

/// Build an addition atomic change for a new paragraph.
fn added_para_change(idx: usize, para: Option<&WordParagraph>) -> OfficeAtomicChange {
    OfficeAtomicChange {
        path: format!("para={}", idx),
        kind: OfficeAtomicChangeKind::TextAdded,
        before: None,
        after: para.map(|p| p.text.clone()),
        metadata: serde_json::Value::Null,
    }
}

/// Convert a column index to a letter (A, B, ..., Z, AA, ...).
fn col_letter(col: usize) -> String {
    let mut n = col + 1;
    let mut result = String::new();
    while n > 0 {
        n -= 1;
        result.push(((n % 26) as u8 + b'A') as char);
        n /= 26;
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_documents_no_changes() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hello world</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let sections = compare_documents(xml.as_bytes(), xml.as_bytes()).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert!(sections[0].changes.is_empty());
    }

    #[test]
    fn test_text_change_detected() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Left text</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Right text</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
        assert_eq!(sections[0].changes.len(), 1);
        assert_eq!(sections[0].changes[0].kind, OfficeAtomicChangeKind::TextChanged);
        assert_eq!(sections[0].changes[0].before.as_deref(), Some("Left text"));
        assert_eq!(sections[0].changes[0].after.as_deref(), Some("Right text"));
    }

    #[test]
    fn test_added_paragraph() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Para 1</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Para 1</w:t></w:r></w:p>
    <w:p><w:r><w:t>Para 2</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(sections[1].status, ChangeStatus::Added);
    }

    #[test]
    fn test_heading_detected() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Title</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let sections = compare_documents(xml.as_bytes(), xml.as_bytes()).unwrap();
        assert_eq!(sections[0].kind, OfficeSectionKind::WordHeading);
        assert_eq!(sections[0].label, "Paragraph 1: Title");
    }

    #[test]
    fn test_table_cell_change() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid><w:gridCol w:w="2400"/></w:tblGrid>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Old cell</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid><w:gridCol w:w="2400"/></w:tblGrid>
      <w:tr>
        <w:tc><w:p><w:r><w:t>New cell</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].kind, OfficeSectionKind::WordTable);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
        assert!(!sections[0].changes.is_empty());
    }

    #[test]
    fn test_style_only_change_produces_formatting_atomic() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Normal"/></w:pPr>
      <w:r><w:t>Same text</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Same text</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
        assert_eq!(sections[0].changes.len(), 1);
        assert_eq!(sections[0].changes[0].kind, OfficeAtomicChangeKind::FormattingChanged);
        assert_eq!(sections[0].changes[0].before.as_deref(), Some("Normal"));
        assert_eq!(sections[0].changes[0].after.as_deref(), Some("Heading1"));
    }

    #[test]
    fn test_text_and_style_change_produces_two_atomics() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Normal"/></w:pPr>
      <w:r><w:t>Old content</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading2"/></w:pPr>
      <w:r><w:t>New content</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
        assert_eq!(sections[0].changes.len(), 2);
        let kinds: Vec<&OfficeAtomicChangeKind> =
            sections[0].changes.iter().map(|c| &c.kind).collect();
        assert!(kinds.contains(&&OfficeAtomicChangeKind::TextChanged));
        assert!(kinds.contains(&&OfficeAtomicChangeKind::FormattingChanged));
    }

    #[test]
    fn test_paragraph_label_truncated_when_text_exceeds_60_chars() {
        let long_text = "A".repeat(80);
        let xml = format!(
            r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>{long_text}</w:t></w:r></w:p>
  </w:body>
</w:document>"#
        );

        let sections = compare_documents(xml.as_bytes(), xml.as_bytes()).unwrap();
        assert_eq!(sections.len(), 1);
        assert!(sections[0].label.ends_with("..."), "label should end with '...' when text > 60 chars");
        assert!(sections[0].label.len() < 80, "label should be shorter than full text");
    }

    #[test]
    fn test_empty_paragraph_label_uses_empty_suffix() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p/>
  </w:body>
</w:document>"#;

        let sections = compare_documents(xml.as_bytes(), xml.as_bytes()).unwrap();
        assert_eq!(sections.len(), 1);
        assert!(sections[0].label.contains("(empty)"), "empty paragraph label should contain '(empty)'");
    }

    #[test]
    fn test_deleted_paragraph_has_before_text_and_no_after() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Kept paragraph</w:t></w:r></w:p>
    <w:p><w:r><w:t>Removed paragraph</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Kept paragraph</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[1].status, ChangeStatus::Deleted);
        assert_eq!(sections[1].changes.len(), 1);
        assert_eq!(sections[1].changes[0].kind, OfficeAtomicChangeKind::TextDeleted);
        assert_eq!(sections[1].changes[0].before.as_deref(), Some("Removed paragraph"));
        assert_eq!(sections[1].changes[0].after, None);
    }

    #[test]
    fn test_added_paragraph_has_after_text_and_no_before() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Original only</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Original only</w:t></w:r></w:p>
    <w:p><w:r><w:t>Brand new paragraph</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[1].status, ChangeStatus::Added);
        assert_eq!(sections[1].changes.len(), 1);
        assert_eq!(sections[1].changes[0].kind, OfficeAtomicChangeKind::TextAdded);
        assert_eq!(sections[1].changes[0].before, None);
        assert_eq!(sections[1].changes[0].after.as_deref(), Some("Brand new paragraph"));
    }

    #[test]
    fn test_table_deleted_produces_deleted_section() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid><w:gridCol w:w="2400"/></w:tblGrid>
      <w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body/>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        let table_section = sections.iter().find(|s| s.kind == OfficeSectionKind::WordTable);
        assert!(table_section.is_some(), "should have a table section");
        assert_eq!(table_section.unwrap().status, ChangeStatus::Deleted);
    }

    #[test]
    fn test_table_added_produces_added_section() {
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body/>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid><w:gridCol w:w="2400"/></w:tblGrid>
      <w:tr><w:tc><w:p><w:r><w:t>New cell</w:t></w:r></w:p></w:tc></w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        let table_section = sections.iter().find(|s| s.kind == OfficeSectionKind::WordTable);
        assert!(table_section.is_some(), "should have a table section");
        assert_eq!(table_section.unwrap().status, ChangeStatus::Added);
    }

    #[test]
    fn test_table_dimension_change_produces_structure_change_atomic() {
        // Left: 2×1 table; right: 2×2 table — dimensions change → StructureChanged.
        let left = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid><w:gridCol w:w="2400"/></w:tblGrid>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let right = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid><w:gridCol w:w="2400"/><w:gridCol w:w="2400"/></w:tblGrid>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let sections = compare_documents(left.as_bytes(), right.as_bytes()).unwrap();
        let table_section = sections.iter().find(|s| s.kind == OfficeSectionKind::WordTable).unwrap();
        assert_eq!(table_section.status, ChangeStatus::Modified);
        let has_structure_change = table_section.changes.iter().any(|c| {
            c.kind == OfficeAtomicChangeKind::StructureChanged
        });
        assert!(has_structure_change, "should have a StructureChanged atomic for dimension change");
    }

    #[test]
    fn test_serialize_sections() {
        let sections = vec![OfficeSectionDiff {
            id: "para-0".to_owned(),
            label: "Paragraph 1: Hello".to_owned(),
            kind: OfficeSectionKind::WordParagraph,
            status: ChangeStatus::Modified,
            changes: vec![OfficeAtomicChange {
                path: "para=0".to_owned(),
                kind: OfficeAtomicChangeKind::TextChanged,
                before: Some("Hello".to_owned()),
                after: Some("World".to_owned()),
                metadata: serde_json::Value::Null,
            }],
        }];

        let json = serde_json::to_string_pretty(&sections).unwrap();
        assert!(json.contains("text-changed"));
        assert!(json.contains("Hello"));
        assert!(json.contains("World"));
    }
}
