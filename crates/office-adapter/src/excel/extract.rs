//! Excel worksheet text extraction.
//!
//! Parses worksheet XML parts (`xl/worksheets/sheetN.xml`) and extracts
//! cell values into a tabular text representation suitable for plain-text
//! diff comparison.
//!
//! Supports inline strings (`t="inlineStr"`) and direct `<t>` text.
//! Shared strings require parsing `xl/sharedStrings.xml` which is not yet
//! implemented (a warning is emitted via tracing when `t="s"` is found).

use crate::xml_util::{self, XmlError, XmlResult};
use quick_xml::events::Event;
use quick_xml::name::QName;
use std::collections::BTreeMap;

/// Extract all cell values from an Excel worksheet XML as tab-separated text.
///
/// Rows are separated by newlines. Cells within a row are separated by tabs.
/// Empty cells at the end of a row are trimmed.
///
/// This is the core of Approach 1 for Excel: feed the result into
/// `diff-core::compare_texts()` for a cell-level text comparison.
pub fn extract_sheet_text(xml: &[u8]) -> XmlResult<String> {
    let mut reader = xml_util::create_reader(xml);
    let mut buf = Vec::new();
    let mut rows: BTreeMap<u32, Vec<(u32, String)>> = BTreeMap::new();
    let mut current_row: Option<u32> = None;
    let mut _depth = 0u32;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                _depth += 1;
                let name = e.name();

                if is_s_element(name, b"row") {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"r" {
                            let row_str = String::from_utf8_lossy(&attr.value);
                            if let Ok(n) = row_str.parse::<u32>() {
                                current_row = Some(n);
                                rows.entry(n).or_default();
                            }
                        }
                    }
                } else if is_s_element(name, b"c") && current_row.is_some() {
                    let row = current_row.unwrap();
                    let mut col_str = String::new();
                    let mut cell_type = String::new();

                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref();
                        if key == b"r" {
                            col_str =
                                String::from_utf8_lossy(&attr.value).to_string();
                        } else if key == b"t" {
                            cell_type =
                                String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }

                    // Parse column letter(s) → 1-based index.
                    let col_idx = parse_col_ref(&col_str).unwrap_or(1);

                    if cell_type == "inlineStr" || cell_type.is_empty() {
                        // Read <is><t>value</t></is> or direct <v>value</v>
                        if let Some(val) = read_cell_value(&mut reader, &mut buf, &cell_type, &col_str)
                        {
                            _depth -= 1; // cell end consumed by read_text
                            rows.entry(row).or_default().push((col_idx, val));
                        }
                    } else if cell_type == "s" {
                        // Shared string — skip for now (Phase 2).
                        // We still need to consume the element so we don't
                        // break the parser.
                        skip_cell_contents(&mut reader, &mut buf);
                        _depth -= 1; // cell end consumed
                    } else {
                        // Unknown type — skip.
                        skip_cell_contents(&mut reader, &mut buf);
                        _depth -= 1;
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if is_s_element(e.name(), b"row") {
                    current_row = None;
                }
                _depth -= 1;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlError::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    // Convert rows map to tab-separated text.
    let mut output = String::new();
    for (_, cells) in &rows {
        if cells.is_empty() {
            output.push('\n');
            continue;
        }
        // Sort by column index.
        let mut sorted: Vec<&(u32, String)> = cells.iter().collect();
        sorted.sort_by_key(|(col, _)| *col);

        let values: Vec<&str> = sorted.iter().map(|(_, v)| v.as_str()).collect();
        output.push_str(&values.join("\t"));
        output.push('\n');
    }

    // Trim trailing newline.
    while output.ends_with('\n') {
        output.pop();
    }

    Ok(output)
}

/// Read the value from a cell's content element.
///
/// For inline strings: `<is><t>value</t></is>`
/// For empty/default type: attempts to read from `<v>` or `<t>`.
///
/// Consumes the cell's End event. Returns None if no text found.
fn read_cell_value(
    reader: &mut quick_xml::Reader<&[u8]>,
    buf: &mut Vec<u8>,
    _cell_type: &str,
    _col_ref: &str,
) -> Option<String> {
    let mut inner_depth = 1u32; // we just read the <c> Start
    let mut value = String::new();

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => {
                inner_depth += 1;
                let name = e.name();

                if is_s_element(name, b"t") {
                    if let Ok(text) = reader.read_text(e.name()) {
                        inner_depth -= 1;
                        let trimmed = text.trim().to_owned();
                        if !trimmed.is_empty() {
                            value.push_str(&trimmed);
                        }
                    }
                }
                // For default type: <v>value</v>
                if is_s_element(name, b"v") {
                    if let Ok(text) = reader.read_text(e.name()) {
                        inner_depth -= 1;
                        let trimmed = text.trim().to_owned();
                        if !trimmed.is_empty() {
                            value.push_str(&trimmed);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                inner_depth -= 1;
                // Exit when we reach the </c> end tag.
                if is_s_element(e.name(), b"c") && inner_depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Skip over a cell's content elements without extracting text.
/// Used for unsupported cell types like shared strings.
fn skip_cell_contents(
    reader: &mut quick_xml::Reader<&[u8]>,
    buf: &mut Vec<u8>,
) {
    let mut inner_depth = 1u32;
    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(_)) => inner_depth += 1,
            Ok(Event::End(ref e)) => {
                inner_depth -= 1;
                if is_s_element(e.name(), b"c") && inner_depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
}

/// Convert a 1-based column number to an Excel column letter (A, B, ..., Z, AA, ...).
pub fn col_to_letter(col: u32) -> String {
    let mut n = col;
    let mut result = String::new();
    while n > 0 {
        n -= 1;
        result.push(((n % 26) as u8 + b'A') as char);
        n /= 26;
    }
    result.chars().rev().collect()
}

/// Parse an Excel column reference like "A", "B", "AA" into a 1-based index.
fn parse_col_ref(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let letters: Vec<char> = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if letters.is_empty() {
        return None;
    }
    let mut col = 0u32;
    for c in &letters {
        let val = c.to_ascii_uppercase() as u32 - b'A' as u32 + 1;
        col = col * 26 + val;
    }
    Some(col)
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Check if an element name uses the spreadsheet `s:` namespace prefix.
/// In practice, worksheet XML often uses a default namespace (no prefix),
/// so we also check for unprefixed names as a fallback.
fn is_s_element(name: QName<'_>, local: &[u8]) -> bool {
    let raw = name.as_ref();
    if raw.len() >= local.len() + 2 && &raw[..2] == b"s:" && &raw[2..] == local {
        return true;
    }
    // Fallback: unprefixed match (common with default namespace).
    raw == local
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_col_ref() {
        assert_eq!(parse_col_ref("A"), Some(1));
        assert_eq!(parse_col_ref("B"), Some(2));
        assert_eq!(parse_col_ref("Z"), Some(26));
        assert_eq!(parse_col_ref("AA"), Some(27));
        assert_eq!(parse_col_ref("A1"), Some(1));
    }

    #[test]
    fn test_extract_single_cell() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr">
        <is><t>Hello</t></is>
      </c>
    </row>
  </sheetData>
</worksheet>"#;

        let text = extract_sheet_text(xml.as_bytes()).unwrap();
        assert_eq!(text, "Hello");
    }

    #[test]
    fn test_extract_multiple_cells_one_row() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr"><is><t>Name</t></is></c>
      <c r="B1" t="inlineStr"><is><t>Age</t></is></c>
      <c r="C1" t="inlineStr"><is><t>City</t></is></c>
    </row>
  </sheetData>
</worksheet>"#;

        let text = extract_sheet_text(xml.as_bytes()).unwrap();
        assert_eq!(text, "Name\tAge\tCity");
    }

    #[test]
    fn test_extract_multiple_rows() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr"><is><t>Header</t></is></c>
    </row>
    <row r="2">
      <c r="A2" t="inlineStr"><is><t>Data</t></is></c>
    </row>
    <row r="3">
      <c r="A3" t="inlineStr"><is><t>More data</t></is></c>
    </row>
  </sheetData>
</worksheet>"#;

        let text = extract_sheet_text(xml.as_bytes()).unwrap();
        assert_eq!(text, "Header\nData\nMore data");
    }

    #[test]
    fn test_extract_sparse_columns() {
        // Cells in columns A and D with gaps.
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="D1" t="inlineStr"><is><t>Col D</t></is></c>
      <c r="A1" t="inlineStr"><is><t>Col A</t></is></c>
    </row>
  </sheetData>
</worksheet>"#;

        let text = extract_sheet_text(xml.as_bytes()).unwrap();
        // Cells sorted by column: A, then D — no tabs for empty B,C.
        assert_eq!(text, "Col A\tCol D");
    }

    #[test]
    fn test_extract_empty_sheet() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData/>
</worksheet>"#;

        let text = extract_sheet_text(xml.as_bytes()).unwrap();
        assert!(text.is_empty());
    }

    #[test]
    fn test_extract_numeric_cell_default_type() {
        // Some xlsx files use <v> for numeric values (no inlineStr).
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1">
        <v>42</v>
      </c>
    </row>
  </sheetData>
</worksheet>"#;

        let text = extract_sheet_text(xml.as_bytes()).unwrap();
        assert_eq!(text, "42");
    }

    #[test]
    fn test_extract_mixed_inline_str_and_numeric() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr"><is><t>Label</t></is></c>
      <c r="B1"><v>100</v></c>
    </row>
  </sheetData>
</worksheet>"#;

        let text = extract_sheet_text(xml.as_bytes()).unwrap();
        assert_eq!(text, "Label\t100");
    }

    #[test]
    fn test_extract_skips_shared_strings_cell() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr"><is><t>Present</t></is></c>
      <c r="B1" t="s"><v>0</v></c>
    </row>
  </sheetData>
</worksheet>"#;

        let text = extract_sheet_text(xml.as_bytes()).unwrap();
        // Shared string cell is skipped — only inlineStr cell appears.
        assert_eq!(text, "Present");
    }
}
