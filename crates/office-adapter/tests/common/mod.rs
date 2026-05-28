//! Shared test utilities for office-adapter integration tests.
//!
//! Includes hand-authored minimal OOXML fixture generators that produce
//! structurally valid Office files with known content for comparison testing.

use std::io::Write;

// ── OOXML XML templates ────────────────────────────────────────────────

/// Generate the `[Content_Types].xml` for a DOCX file.
fn content_types_docx() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#
}

/// Generate the `[Content_Types].xml` for a PPTX file.
fn content_types_pptx() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#
}

/// Generate the `[Content_Types].xml` for an XLSX file.
fn content_types_xlsx() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#
}

/// Generate `_rels/.rels` pointing to the main document part.
fn rels_root(target: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="{target}"/>
</Relationships>"#
    )
}

/// Generate `ppt/_rels/presentation.xml.rels` linking slide1.
fn rels_ppt_presentation() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#
}

/// Generate `ppt/slides/_rels/slide1.xml.rels`.
fn rels_ppt_slide() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>
"#
}

/// Generate `xl/_rels/workbook.xml.rels` linking sheet1.
fn rels_xl_workbook() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#
}

/// Generate `ppt/presentation.xml` with slide list.
fn presentation_xml(slide_count: usize) -> String {
    let slides: String = (1..=slide_count)
        .map(|_i| {
            format!(
                r#"  <p:sldId id="256" r:id="rId1"/>"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>
{slides}
  </p:sldIdLst>
</p:presentation>"#
    )
}

/// Generate `xl/workbook.xml` with sheet list.
fn workbook_xml(sheet_names: &[&str]) -> String {
    let sheets: String = sheet_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            format!(
                r#"    <sheet name="{}" sheetId="{}" r:id="rId1"/>"#,
                name,
                i + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
{}
  </sheets>
</workbook>"#,
        sheets
    )
}

// ── OOXML zip generators ───────────────────────────────────────────────

/// Build a minimal valid zip file in memory from named entries.
fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options =
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }
    buf
}

/// Generate a minimal `.docx` file with a single paragraph.
///
/// The document body contains the provided XML content (which should be
/// well-formed WordprocessingML paragraph elements).
pub fn generate_docx(body_xml: &str) -> Vec<u8> {
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
{}
  </w:body>
</w:document>"#,
        body_xml
    );

    build_zip(&[
        ("[Content_Types].xml", content_types_docx().as_bytes()),
        ("_rels/.rels", rels_root("word/document.xml").as_bytes()),
        ("word/document.xml", document_xml.as_bytes()),
    ])
}

/// Generate a minimal `.pptx` file with a single slide.
///
/// The slide XML content should be well-formed PresentationML slide elements.
pub fn generate_pptx(slide_xml: &str) -> Vec<u8> {
    build_zip(&[
        ("[Content_Types].xml", content_types_pptx().as_bytes()),
        ("_rels/.rels", rels_root("ppt/presentation.xml").as_bytes()),
        ("ppt/presentation.xml", presentation_xml(1).as_bytes()),
        (
            "ppt/_rels/presentation.xml.rels",
            rels_ppt_presentation().as_bytes(),
        ),
        ("ppt/slides/slide1.xml", slide_xml.as_bytes()),
        (
            "ppt/slides/_rels/slide1.xml.rels",
            rels_ppt_slide().as_bytes(),
        ),
    ])
}

/// Generate a minimal `.xlsx` file with one worksheet.
///
/// Uses inline strings (`t="inlineStr"`) to avoid needing a shared strings table.
/// The sheet XML content should be well-formed SpreadsheetML worksheet elements.
pub fn generate_xlsx(sheet_name: &str, sheet_xml: &str) -> Vec<u8> {
    build_zip(&[
        ("[Content_Types].xml", content_types_xlsx().as_bytes()),
        ("_rels/.rels", rels_root("xl/workbook.xml").as_bytes()),
        ("xl/workbook.xml", workbook_xml(&[sheet_name]).as_bytes()),
        (
            "xl/_rels/workbook.xml.rels",
            rels_xl_workbook().as_bytes(),
        ),
        ("xl/worksheets/sheet1.xml", sheet_xml.as_bytes()),
    ])
}

// ── Pre-built fixture convenience functions ────────────────────────────

/// Fixture: Word document with a single paragraph saying "Hello from the left file."
pub fn docx_left_body() -> &'static str {
    r#"    <w:p>
      <w:r>
        <w:t>Hello from the left file.</w:t>
      </w:r>
    </w:p>"#
}

/// Fixture: Word document with a single paragraph saying "Hello from the right file."
pub fn docx_right_body() -> &'static str {
    r#"    <w:p>
      <w:r>
        <w:t>Hello from the right file.</w:t>
      </w:r>
    </w:p>"#
}

/// Fixture: Word document with two paragraphs (multi-section).
#[allow(dead_code)]
pub fn docx_two_paragraphs() -> &'static str {
    r#"    <w:p>
      <w:r>
        <w:t>First paragraph.</w:t>
      </w:r>
    </w:p>
    <w:p>
      <w:r>
        <w:t>Second paragraph.</w:t>
      </w:r>
    </w:p>"#
}

/// Fixture: Word document with a table.
#[allow(dead_code)]
pub fn docx_with_table() -> &'static str {
    r#"    <w:p>
      <w:r>
        <w:t>Above the table.</w:t>
      </w:r>
    </w:p>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid>
        <w:gridCol w:w="2400"/>
        <w:gridCol w:w="2400"/>
      </w:tblGrid>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>Cell A1</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>Cell B1</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>Cell A2</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>Cell B2</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>"#
}

// ── Complex docx generators for approach comparison research ───────────

/// Docx with mixed formatting: bold "Important" + normal "text here." in same paragraph.
#[allow(dead_code)]
pub fn docx_body_with_formatting() -> &'static str {
    r#"    <w:p>
      <w:r>
        <w:rPr><w:b/></w:rPr>
        <w:t>Important</w:t>
      </w:r>
      <w:r>
        <w:t> text here.</w:t>
      </w:r>
    </w:p>"#
}

/// Docx body with same text but formatting removed (bold → normal).
#[allow(dead_code)]
pub fn docx_body_without_formatting() -> &'static str {
    r#"    <w:p>
      <w:r>
        <w:t>Important text here.</w:t>
      </w:r>
    </w:p>"#
}

/// Docx body with heading + two body paragraphs.
#[allow(dead_code)]
pub fn docx_body_heading_and_paras() -> &'static str {
    r#"    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Quarterly Report</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Revenue increased 15% year over year.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Costs remained flat.</w:t></w:r>
    </w:p>"#
}

/// Docx body with heading + modified body paragraphs for diff testing.
#[allow(dead_code)]
pub fn docx_body_heading_and_paras_modified() -> &'static str {
    r#"    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Quarterly Report</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Revenue increased 22% year over year.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Costs increased 5%.</w:t></w:r>
    </w:p>"#
}

/// Docx body with a table containing numeric data.
#[allow(dead_code)]
pub fn docx_body_with_table_data() -> &'static str {
    r#"    <w:p>
      <w:r><w:t>Sales Data</w:t></w:r>
    </w:p>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid>
        <w:gridCol w:w="2400"/>
        <w:gridCol w:w="2400"/>
        <w:gridCol w:w="2400"/>
      </w:tblGrid>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Product</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>Q1</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>Q2</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Widget A</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>100</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>120</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Widget B</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>85</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>95</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>"#
}

/// Docx body with table data modified (Q2 values changed).
#[allow(dead_code)]
pub fn docx_body_with_table_data_modified() -> &'static str {
    r#"    <w:p>
      <w:r><w:t>Sales Data</w:t></w:r>
    </w:p>
    <w:tbl>
      <w:tblPr/>
      <w:tblGrid>
        <w:gridCol w:w="2400"/>
        <w:gridCol w:w="2400"/>
        <w:gridCol w:w="2400"/>
      </w:tblGrid>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Product</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>Q1</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>Q2</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Widget A</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>100</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>135</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>Widget B</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>85</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>90</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>"#
}

/// Docx body with tracked changes: deleted paragraph wrapped in `<w:del>`.
#[allow(dead_code)]
pub fn docx_body_with_tracked_changes() -> &'static str {
    r#"    <w:p>
      <w:r><w:t>This paragraph is unchanged.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r>
        <w:ins w:id="1" w:author="Jane" w:date="2025-01-15T10:00:00Z">
          <w:t>This paragraph was added by Jane.</w:t>
        </w:ins>
      </w:r>
    </w:p>
    <w:p>
      <w:pPr><w:rPr><w:del w:id="2" w:author="Bob" w:date="2025-01-14T09:00:00Z"/></w:rPr></w:pPr>
      <w:r><w:t>Final paragraph.</w:t></w:r>
    </w:p>"#
}

/// Same docx without tracked changes (showing resolved state).
#[allow(dead_code)]
pub fn docx_body_tracked_changes_resolved() -> &'static str {
    r#"    <w:p>
      <w:r><w:t>This paragraph is unchanged.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>This paragraph was added by Jane.</w:t></w:r>
    </w:p>
    <w:p>
      <w:r><w:t>Final paragraph.</w:t></w:r>
    </w:p>"#
}

/// Fixture: PowerPoint slide with a title and body.
pub fn pptx_slide_with_title(title: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:cSld>
    <p:spTree>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="1" name="Title"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr><p:ph type="title"/></p:nvPr>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:r><a:t>{title}</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id="2" name="Body"/>
          <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr/>
        <p:txBody>
          <a:bodyPr/>
          <a:p>
            <a:r><a:t>{body}</a:t></a:r>
          </a:p>
        </p:txBody>
      </p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#
    )
}

/// Fixture: Excel worksheet with inline-string cells.
pub fn xlsx_sheet_with_cells(cells: &[(u32, u32, &str)]) -> String {
    let rows: String = cells
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut acc, (r, c, v)| {
            acc.entry(r).or_insert_with(Vec::new).push((c, v));
            acc
        })
        .iter()
        .map(|(row, cols)| {
            let cols_xml: String = cols
                .iter()
                .map(|(col, val)| {
                    format!(
                        r#"      <c r="{col_ref}{row}" t="inlineStr">
        <is><t>{val}</t></is>
      </c>"#,
                        col_ref = col_to_letter(**col),
                        row = row,
                        val = val
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r#"    <row r="{row}">
{}
    </row>"#,
                cols_xml
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheetData>
{}
  </sheetData>
</worksheet>"#,
        rows
    )
}

/// Convert a 1-based column number to an Excel column letter (A, B, ..., Z, AA, ...).
/// Delegates to the shared implementation in `office_adapter::excel::extract`.
fn col_to_letter(col: u32) -> String {
    office_adapter::excel::extract::col_to_letter(col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_col_to_letter() {
        assert_eq!(col_to_letter(1), "A");
        assert_eq!(col_to_letter(2), "B");
        assert_eq!(col_to_letter(26), "Z");
        assert_eq!(col_to_letter(27), "AA");
        assert_eq!(col_to_letter(52), "AZ");
    }

    #[test]
    fn test_generate_docx_is_valid_zip() {
        let data = generate_docx(docx_left_body());
        let cursor = std::io::Cursor::new(&data);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        assert!(archive.len() >= 3);
        // Verify expected entries exist.
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect();
        assert!(names.iter().any(|n| n == "[Content_Types].xml"));
        assert!(names.iter().any(|n| n == "_rels/.rels"));
        assert!(names.iter().any(|n| n == "word/document.xml"));
    }

    #[test]
    fn test_generate_pptx_is_valid_zip() {
        let slide = pptx_slide_with_title("Test Title", "Test Body");
        let data = generate_pptx(&slide);
        let cursor = std::io::Cursor::new(&data);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect();
        assert!(names.iter().any(|n| n == "ppt/presentation.xml"));
        assert!(names.iter().any(|n| n == "ppt/slides/slide1.xml"));
    }

    #[test]
    fn test_generate_xlsx_is_valid_zip() {
        let sheet = xlsx_sheet_with_cells(&[(1, 1, "hello"), (2, 3, "world")]);
        let data = generate_xlsx("Sheet1", &sheet);
        let cursor = std::io::Cursor::new(&data);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect();
        assert!(names.iter().any(|n| n == "xl/workbook.xml"));
        assert!(names.iter().any(|n| n == "xl/worksheets/sheet1.xml"));
    }

    #[test]
    fn test_docx_body_content_roundtrip() {
        let data = generate_docx(docx_left_body());
        let cursor = std::io::Cursor::new(&data);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut found = false;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            if entry.name() == "word/document.xml" {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut entry, &mut buf).unwrap();
                assert!(buf.contains("Hello from the left file."));
                found = true;
            }
        }
        assert!(found, "word/document.xml not found in generated docx");
    }
}
