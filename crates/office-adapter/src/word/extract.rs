//! Word document text extraction (Approach 1: Convert-to-Text).
//!
//! Parses `word/document.xml` and extracts plain text, flattening the
//! document into a line-per-paragraph representation suitable for feeding
//! into `diff-core::compare_texts()`.
//!
//! Also provides the shared text extraction that Approach 3 (semantic
//! comparison) uses internally.

use crate::models::{WordDocument, WordParagraph, WordTable};
use crate::xml_util::{self, XmlError, XmlResult};
use quick_xml::events::Event;
use quick_xml::name::QName;

// ── Text extraction (Approach 1) ───────────────────────────────────────

/// Extract all text from a Word document XML as a single string.
///
/// Each paragraph becomes one line. Table cells are extracted inline
/// between paragraphs with their cell reference.
///
/// This is the core of Approach 1: feed the result into `diff-core`
/// for a line-level text comparison.
pub fn extract_text(xml: &[u8]) -> XmlResult<String> {
    let mut reader = xml_util::create_reader(xml);
    let mut buf = Vec::new();
    let mut output = String::new();
    let mut depth = 0u32;
    let mut in_paragraph = false;
    let mut para_text = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name = e.name();

                if is_w_element(name, b"p") {
                    in_paragraph = true;
                    para_text.clear();
                } else if is_w_element(name, b"t") && in_paragraph {
                    // Collect text from <w:t> elements.
                    // read_text() consumes the End event, so decrement depth.
                    if let Ok(text) = reader.read_text(e.name()) {
                        depth -= 1;
                        // Preserve whitespace between runs: use the raw text
                        // without trimming to avoid losing spaces between
                        // adjacent runs in the same paragraph.
                        if !text.is_empty() {
                            para_text.push_str(&text);
                        }
                    }
                } else if is_w_element(name, b"tbl") {
                    // Extract table text inline.
                    if !output.is_empty() && !output.ends_with('\n') {
                        output.push('\n');
                    }
                    let table_text = extract_table_text(&mut reader, &mut buf, depth)?;
                    output.push_str(&table_text);
                    // extract_table_text consumed the </w:tbl> End event —
                    // balance the depth counter here.
                    depth -= 1;
                }
            }
            Ok(Event::End(ref e)) => {
                if is_w_element(e.name(), b"p") && in_paragraph {
                    in_paragraph = false;
                    output.push_str(&para_text);
                    output.push('\n');
                }
                depth -= 1;
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlError::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    // Trim trailing newline.
    while output.ends_with('\n') {
        output.pop();
    }

    Ok(output)
}

/// Extract text from a <w:tbl> element into a formatted string.
fn extract_table_text(
    reader: &mut quick_xml::Reader<&[u8]>,
    buf: &mut Vec<u8>,
    _table_depth: u32,
) -> XmlResult<String> {
    let mut output = String::new();
    let mut depth = 1u32; // we're inside the table start element
    let mut in_cell = false;
    let mut cell_text = String::new();

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                if is_w_element(e.name(), b"tc") {
                    in_cell = true;
                    cell_text.clear();
                } else if is_w_element(e.name(), b"t") && in_cell {
                    // read_text() consumes the End event, so decrement depth.
                    if let Ok(text) = reader.read_text(e.name()) {
                        depth -= 1;
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            cell_text.push_str(trimmed);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if is_w_element(e.name(), b"tc") && in_cell {
                    in_cell = false;
                    if !output.is_empty() {
                        output.push_str(" | ");
                    }
                    output.push_str(&cell_text);
                } else if is_w_element(e.name(), b"tr") {
                    output.push('\n');
                }
                depth -= 1;
                if is_w_element(e.name(), b"tbl") && depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlError::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(output)
}

// ── Structured extraction (used by Approach 3) ─────────────────────────

/// Parse a Word document XML into the structured `WordDocument` model.
///
/// Extracts paragraphs with their text, style, and heading level.
/// Extracts tables with cell contents.
/// This is the extraction half of Approach 3 (semantic comparison).
pub fn extract_document(xml: &[u8]) -> XmlResult<WordDocument> {
    let mut reader = xml_util::create_reader(xml);
    let mut buf = Vec::new();
    let mut doc = WordDocument::default();
    let mut para_index = 0usize;
    let mut table_index = 0usize;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();

                if is_w_element(name, b"p") {
                    let para = parse_paragraph(&mut reader, &mut buf, para_index)?;
                    doc.paragraphs.push(para);
                    para_index += 1;
                } else if is_w_element(name, b"tbl") {
                    let table = parse_table(&mut reader, &mut buf, table_index)?;
                    doc.tables.push(table);
                    table_index += 1;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlError::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(doc)
}

/// Parse a single `<w:p>` element into a `WordParagraph`.
fn parse_paragraph(
    reader: &mut quick_xml::Reader<&[u8]>,
    buf: &mut Vec<u8>,
    index: usize,
) -> XmlResult<WordParagraph> {
    let mut text = String::new();
    let mut style: Option<String> = None;
    let mut heading_level: Option<u8> = None;
    let mut depth = 1u32; // inside the <w:p> start

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name = e.name();

                if is_w_element(name, b"pStyle") {
                    // Extract style from <w:pStyle w:val="..."/>
                    for attr in e.attributes().flatten() {
                        if is_w_attribute(&attr, b"val") {
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            style = Some(val.clone());
                            // Detect heading level from style name (e.g., "Heading1" → 1)
                            if let Some(level) = detect_heading_level(&val) {
                                heading_level = Some(level);
                            }
                        }
                    }
                } else if is_w_element(name, b"t") {
                    // Collect text from <w:t>.
                    // read_text() consumes the End event, so decrement depth.
                    if let Ok(t) = reader.read_text(e.name()) {
                        depth -= 1;
                        let trimmed = t.trim().to_owned();
                        if !trimmed.is_empty() {
                            text.push_str(&trimmed);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                if is_w_element(e.name(), b"p") && depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlError::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    Ok(WordParagraph {
        index,
        text,
        style,
        heading_level,
    })
}

/// Parse a single `<w:tbl>` element into a `WordTable`.
fn parse_table(
    reader: &mut quick_xml::Reader<&[u8]>,
    buf: &mut Vec<u8>,
    index: usize,
) -> XmlResult<WordTable> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut depth = 1u32; // inside the <w:tbl> start
    let mut in_row = false;
    let mut current_row: Vec<String> = Vec::new();
    let mut in_cell = false;
    let mut cell_text = String::new();

    loop {
        match reader.read_event_into(buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name = e.name();

                if is_w_element(name, b"tr") {
                    in_row = true;
                    current_row = Vec::new();
                } else if is_w_element(name, b"tc") {
                    in_cell = true;
                    cell_text.clear();
                } else if is_w_element(name, b"t") && in_cell {
                    // read_text() consumes the End event, so decrement depth.
                    if let Ok(t) = reader.read_text(e.name()) {
                        depth -= 1;
                        let trimmed = t.trim().to_owned();
                        if !trimmed.is_empty() {
                            cell_text.push_str(&trimmed);
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                depth -= 1;
                let name = e.name();

                if is_w_element(name, b"tc") && in_cell {
                    in_cell = false;
                    current_row.push(std::mem::take(&mut cell_text));
                } else if is_w_element(name, b"tr") && in_row {
                    in_row = false;
                    rows.push(std::mem::take(&mut current_row));
                } else if is_w_element(name, b"tbl") && depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(XmlError::Parse(e.to_string())),
            _ => {}
        }
        buf.clear();
    }

    let cols = rows.first().map(|r| r.len()).unwrap_or(0);

    Ok(WordTable {
        index,
        rows: rows.len(),
        cols,
        cells: rows,
    })
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Check if an element name uses the `w:` namespace prefix and the local part matches.
///
/// In quick-xml 0.37, QName does not expose namespace resolution. We check
/// for the `w:` prefix in the raw bytes as a pragmatic namespace test.
fn is_w_element(name: QName<'_>, local: &[u8]) -> bool {
    let raw = name.as_ref();
    // Check for "w:" prefix.
    if raw.len() < local.len() + 2 {
        return false;
    }
    &raw[..2] == b"w:" && &raw[2..] == local
}

/// Check an attribute name for the `w:` prefix.
fn is_w_attribute(attr: &quick_xml::events::attributes::Attribute<'_>, local: &[u8]) -> bool {
    let raw = attr.key.as_ref();
    if raw.len() < local.len() + 2 {
        return false;
    }
    &raw[..2] == b"w:" && &raw[2..] == local
}

/// Detect heading level from a style name like "Heading1", "Heading2", etc.
fn detect_heading_level(style: &str) -> Option<u8> {
    if style.len() > 7 && style.starts_with("Heading") {
        style[7..].parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_paragraph() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hello world</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let text = extract_text(xml.as_bytes()).unwrap();
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn test_extract_multiple_paragraphs() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>First paragraph.</w:t></w:r></w:p>
    <w:p><w:r><w:t>Second paragraph.</w:t></w:r></w:p>
    <w:p><w:r><w:t>Third paragraph.</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let text = extract_text(xml.as_bytes()).unwrap();
        assert_eq!(
            text,
            "First paragraph.\nSecond paragraph.\nThird paragraph."
        );
    }

    #[test]
    fn test_extract_mixed_paragraphs_and_table() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Above table</w:t></w:r></w:p>
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
    <w:p><w:r><w:t>Below table</w:t></w:r></w:p>
  </w:body>
</w:document>"#;

        let text = extract_text(xml.as_bytes()).unwrap();
        // Table rows separated by newlines, cells by " | "
        assert!(text.contains("Above table"));
        assert!(text.contains("A1 | B1"));
        assert!(text.contains("A2 | B2"));
        assert!(text.contains("Below table"));
    }

    #[test]
    fn test_extract_document_structured() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>My Title</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Some body text.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let doc = extract_document(xml.as_bytes()).unwrap();
        assert_eq!(doc.paragraphs.len(), 2);
        assert_eq!(doc.paragraphs[0].text, "My Title");
        assert_eq!(doc.paragraphs[0].heading_level, Some(1));
        assert_eq!(doc.paragraphs[0].style.as_deref(), Some("Heading1"));
        assert_eq!(doc.paragraphs[1].text, "Some body text.");
        assert_eq!(doc.paragraphs[1].heading_level, None);
    }

    #[test]
    fn test_extract_document_with_table() {
        let xml = r#"<?xml version="1.0"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Text</w:t></w:r></w:p>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid><w:gridCol w:w="2400"/></w:tblGrid>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let doc = extract_document(xml.as_bytes()).unwrap();
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.tables.len(), 1);
        assert_eq!(doc.tables[0].rows, 1);
        assert_eq!(doc.tables[0].cols, 1);
        assert_eq!(doc.tables[0].cells[0][0], "Cell");
    }
}
