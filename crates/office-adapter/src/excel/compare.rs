//! Semantic Excel comparison (Approach 3: Semantic Model).
//!
//! Compares two Excel workbooks sheet-by-sheet. Each sheet's text is
//! extracted via `extract_sheet_text()`, then sheets are matched by
//! index and compared character-by-character.
//!
//! This produces `OfficeSectionDiff` entries that can be rendered by the
//! CLI, desktop UI, and AI pipelines.

use crate::models::*;
use crate::excel::extract;
use crate::xml_util::XmlResult;

/// Compare two workbooks, sheet-by-sheet by index.
///
/// Each tuple in the input slices is `(sheet_name, sheet_xml_bytes)`.
/// The caller is responsible for sorting sheet parts numerically and
/// providing sheet names (e.g., from `xl/workbook.xml`).
///
/// Sheets present in both workbooks are compared by their extracted text;
/// sheets present in only one are reported as Added or Deleted.
pub fn compare_workbooks(
    left_sheets: &[(String, &[u8])],
    right_sheets: &[(String, &[u8])],
) -> XmlResult<Vec<OfficeSectionDiff>> {
    let max_sheets = left_sheets.len().max(right_sheets.len());
    let mut sections = Vec::with_capacity(max_sheets);

    for i in 0..max_sheets {
        let left = left_sheets.get(i);
        let right = right_sheets.get(i);

        let (status, changes, name) = match (left, right) {
            (Some((lname, lx)), None) => {
                let left_text = extract::extract_sheet_text(lx)?;
                (
                    ChangeStatus::Deleted,
                    vec![OfficeAtomicChange {
                        path: format!("sheet={}", lname),
                        kind: OfficeAtomicChangeKind::TextDeleted,
                        before: Some(left_text),
                        after: None,
                        metadata: serde_json::Value::Null,
                    }],
                    lname.clone(),
                )
            }
            (None, Some((rname, rx))) => {
                let right_text = extract::extract_sheet_text(rx)?;
                (
                    ChangeStatus::Added,
                    vec![OfficeAtomicChange {
                        path: format!("sheet={}", rname),
                        kind: OfficeAtomicChangeKind::TextAdded,
                        before: None,
                        after: Some(right_text),
                        metadata: serde_json::Value::Null,
                    }],
                    rname.clone(),
                )
            }
            (Some((name, lx)), Some((_, rx))) => {
                let left_text = extract::extract_sheet_text(lx)?;
                let right_text = extract::extract_sheet_text(rx)?;

                if left_text == right_text {
                    (ChangeStatus::Unchanged, Vec::new(), name.clone())
                } else {
                    (
                        ChangeStatus::Modified,
                        vec![OfficeAtomicChange {
                            path: format!("sheet={}", name),
                            kind: OfficeAtomicChangeKind::TextChanged,
                            before: Some(left_text),
                            after: Some(right_text),
                            metadata: serde_json::Value::Null,
                        }],
                        name.clone(),
                    )
                }
            }
            (None, None) => unreachable!(),
        };

        sections.push(OfficeSectionDiff {
            id: format!("sheet-{}", i + 1),
            label: name.to_owned(),
            kind: OfficeSectionKind::ExcelSheet,
            status,
            changes,
        });
    }

    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal sheet XML helper — single cell at A1.
    fn single_cell_sheet(value: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr">
        <is><t>{value}</t></is>
      </c>
    </row>
  </sheetData>
</worksheet>"#
        )
    }

    /// Sheet with multiple cells in one row.
    fn multi_cell_sheet(values: &[&str]) -> String {
        let cells: String = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let col = ((i as u8 + b'A') as char).to_string();
                format!(
                    r#"      <c r="{col}1" t="inlineStr">
        <is><t>{v}</t></is>
      </c>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
{}
    </row>
  </sheetData>
</worksheet>"#,
            cells
        )
    }

    #[test]
    fn test_identical_sheets_no_changes() {
        let sheet = single_cell_sheet("Hello world");
        let sheet_bytes = sheet.as_bytes();

        let sections = compare_workbooks(
            &[("Sheet1".to_owned(), sheet_bytes)],
            &[("Sheet1".to_owned(), sheet_bytes)],
        )
        .unwrap();

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert!(sections[0].changes.is_empty());
        assert_eq!(sections[0].kind, OfficeSectionKind::ExcelSheet);
        assert_eq!(sections[0].label, "Sheet1");
    }

    #[test]
    fn test_text_change_detected() {
        let left = single_cell_sheet("Q1 Revenue");
        let right = single_cell_sheet("Q2 Revenue");

        let sections = compare_workbooks(
            &[("Revenue".to_owned(), left.as_bytes())],
            &[("Revenue".to_owned(), right.as_bytes())],
        )
        .unwrap();

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
        assert_eq!(sections[0].changes.len(), 1);
        assert_eq!(
            sections[0].changes[0].kind,
            OfficeAtomicChangeKind::TextChanged
        );
        assert!(sections[0].changes[0].before.as_deref().unwrap().contains("Q1 Revenue"));
        assert!(sections[0].changes[0].after.as_deref().unwrap().contains("Q2 Revenue"));
    }

    #[test]
    fn test_multi_row_sheet_change() {
        let left = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1"><c r="A1" t="inlineStr"><is><t>Header</t></is></c></row>
    <row r="2"><c r="A2" t="inlineStr"><is><t>Row 1</t></is></c></row>
    <row r="3"><c r="A3" t="inlineStr"><is><t>Row 2</t></is></c></row>
  </sheetData>
</worksheet>"#
        );

        let right = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1"><c r="A1" t="inlineStr"><is><t>Header</t></is></c></row>
    <row r="2"><c r="A2" t="inlineStr"><is><t>Row 1 modified</t></is></c></row>
    <row r="3"><c r="A3" t="inlineStr"><is><t>Row 2</t></is></c></row>
  </sheetData>
</worksheet>"#
        );

        let sections = compare_workbooks(
            &[("Data".to_owned(), left.as_bytes())],
            &[("Data".to_owned(), right.as_bytes())],
        )
        .unwrap();

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
    }

    #[test]
    fn test_added_sheet() {
        let left = single_cell_sheet("Revenue Data");

        let sections = compare_workbooks(
            &[("Revenue".to_owned(), left.as_bytes())],
            &[
                ("Revenue".to_owned(), left.as_bytes()),
                ("Expenses".to_owned(), single_cell_sheet("Expense Data").as_bytes()),
            ],
        )
        .unwrap();

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(sections[0].label, "Revenue");
        assert_eq!(sections[1].status, ChangeStatus::Added);
        assert_eq!(sections[1].label, "Expenses");
        assert_eq!(sections[1].changes[0].kind, OfficeAtomicChangeKind::TextAdded);
    }

    #[test]
    fn test_deleted_sheet() {
        let sections = compare_workbooks(
            &[
                ("Sheet1".to_owned(), single_cell_sheet("Data 1").as_bytes()),
                ("Sheet2".to_owned(), single_cell_sheet("Data 2").as_bytes()),
            ],
            &[("Sheet1".to_owned(), single_cell_sheet("Data 1").as_bytes())],
        )
        .unwrap();

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(sections[1].status, ChangeStatus::Deleted);
        assert_eq!(sections[1].changes[0].kind, OfficeAtomicChangeKind::TextDeleted);
    }

    #[test]
    fn test_sheet_renamed_but_same_content() {
        // When sheets differ by name but same content: matched by index.
        // This is modified because names differ (but since we only compare
        // extracted text, identical text → Unchanged).
        let sheet = single_cell_sheet("Same data");

        let sections = compare_workbooks(
            &[("Old Name".to_owned(), sheet.as_bytes())],
            &[("New Name".to_owned(), sheet.as_bytes())],
        )
        .unwrap();

        // Content is identical, so Unchanged. The label uses the left name.
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        // Label comes from left side's name.
        assert_eq!(sections[0].label, "Old Name");
    }

    #[test]
    fn test_empty_sheet() {
        let empty = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
</worksheet>"#;

        let sections = compare_workbooks(
            &[("Empty".to_owned(), empty.as_bytes())],
            &[("Empty".to_owned(), empty.as_bytes())],
        )
        .unwrap();

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(sections[0].label, "Empty");
    }

    #[test]
    fn test_multi_cell_sheet() {
        let left = multi_cell_sheet(&["Name", "Age", "City"]);
        let right = multi_cell_sheet(&["Name", "Age", "Country"]);

        let sections = compare_workbooks(
            &[("Sheet1".to_owned(), left.as_bytes())],
            &[("Sheet1".to_owned(), right.as_bytes())],
        )
        .unwrap();

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].status, ChangeStatus::Modified);
        // The extracted text is tab-separated: changed "City" → "Country"
        assert!(sections[0].changes[0]
            .before
            .as_deref()
            .unwrap()
            .contains("City"));
        assert!(sections[0].changes[0]
            .after
            .as_deref()
            .unwrap()
            .contains("Country"));
    }

    #[test]
    fn test_serialize_sections() {
        let sections = vec![OfficeSectionDiff {
            id: "sheet-1".to_owned(),
            label: "Revenue".to_owned(),
            kind: OfficeSectionKind::ExcelSheet,
            status: ChangeStatus::Modified,
            changes: vec![OfficeAtomicChange {
                path: "sheet=Revenue".to_owned(),
                kind: OfficeAtomicChangeKind::TextChanged,
                before: Some("100\t200\t300".to_owned()),
                after: Some("100\t250\t300".to_owned()),
                metadata: serde_json::Value::Null,
            }],
        }];

        let json = serde_json::to_string_pretty(&sections).unwrap();
        assert!(json.contains("excel-sheet"));
        assert!(json.contains("100\\t200\\t300"));
        assert!(json.contains("100\\t250\\t300"));
    }
}
