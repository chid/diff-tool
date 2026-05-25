//! # office-adapter
//!
//! Office document format detection, extraction, and structured comparison
//! for the diff-tool project.
//!
//! Supports `.docx` (Word), `.pptx` (PowerPoint), and `.xlsx` (Excel) files
//! by unpacking the Open Packaging Convention (zip + XML) format and comparing
//! structured document models.
//!
//! ## Safety
//!
//! - Enforces maximum part size and count limits to prevent zip-bomb attacks.
//! - Hashes embedded media incrementally without loading full binary into memory.
//! - XML entity expansion is controlled by the parser configuration.
//!
//! ## Design
//!
//! This crate is format-aware but UI-agnostic. It produces deterministic,
//! serializable `OfficeDiffResult` values that feed CLI, desktop UI, and
//! AI enrichment pipelines.

pub mod excel;
pub mod format;
pub mod models;
pub mod powerpoint;
pub mod unpack;
pub mod word;
pub mod xml_util;

use std::path::PathBuf;
use thiserror::Error;

use models::{ChangeStatus, OfficeDiffOptions, OfficeDiffResult, OfficeDiffSummary, OfficeFormat};

// ── Error type ─────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum OfficeError {
    #[error("unknown or unsupported Office format: {path}")]
    UnknownFormat { path: PathBuf },

    #[error("not a valid Office file (invalid zip): {path}")]
    InvalidZip { path: PathBuf },

    #[error("missing required part '{part}' in {path}")]
    MissingPart { path: PathBuf, part: String },

    #[error("XML parse error in {path}: {message}")]
    XmlParse { path: PathBuf, message: String },

    #[error("file too large: {path} ({size} bytes) exceeds limit of {limit} bytes")]
    FileTooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },

    #[error("too many parts in {path}: {count} exceeds limit of {limit}")]
    TooManyParts {
        path: PathBuf,
        count: usize,
        limit: usize,
    },

    #[error("part too large in {path}: part '{part}' is {size} bytes, limit is {limit} bytes")]
    PartTooLarge {
        path: PathBuf,
        part: String,
        size: u64,
        limit: u64,
    },

    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
}

// ── Public API ─────────────────────────────────────────────────────────

/// Compare two Office documents and return a structured diff result.
///
/// This is the main entry point. It detects the format, unpacks both files,
/// extracts structured document models, and compares them section-by-section.
pub fn compare_office_files(
    left: PathBuf,
    right: PathBuf,
    options: OfficeDiffOptions,
) -> Result<OfficeDiffResult, OfficeError> {
    let format = format::detect_format(&left)?;

    // Verify the right file matches the detected format.
    let right_format = format::detect_format(&right)?;
    if format != right_format {
        return Err(OfficeError::UnknownFormat { path: right });
    }

    let left_bytes = std::fs::read(&left).map_err(|source| OfficeError::ReadFile {
        path: left.clone(),
        source,
    })?;
    let right_bytes = std::fs::read(&right).map_err(|source| OfficeError::ReadFile {
        path: right.clone(),
        source,
    })?;

    let left_archive = unpack::read_zip_safe(&left, &left_bytes, &options)?;
    let right_archive = unpack::read_zip_safe(&right, &right_bytes, &options)?;

    // Collect oversized entries from both archives as unsupported features.
    let mut unsupported: Vec<String> = Vec::new();
    for entry in &left_archive.oversized {
        unsupported.push(format!("left: {} (too large)", entry));
    }
    for entry in &right_archive.oversized {
        unsupported.push(format!("right: {} (too large)", entry));
    }

    let warnings: Vec<String> = Vec::new();

    let sections = match format {
        OfficeFormat::Word => {
            let left_xml = left_archive
                .get("word/document.xml")
                .ok_or_else(|| OfficeError::MissingPart {
                    path: left.clone(),
                    part: "word/document.xml".to_owned(),
                })?;
            let right_xml = right_archive
                .get("word/document.xml")
                .ok_or_else(|| OfficeError::MissingPart {
                    path: right.clone(),
                    part: "word/document.xml".to_owned(),
                })?;

            word::compare::compare_documents(&left_xml.data, &right_xml.data).map_err(
                |e| OfficeError::XmlParse {
                    path: left.to_path_buf(),
                    message: format!("Word document XML parse error: {}", e),
                },
            )?
        }
        OfficeFormat::PowerPoint => {
            // Collect and sort slide XML parts from both archives.
            let left_slides = collect_slides(&left_archive);
            let right_slides = collect_slides(&right_archive);

            let left_refs: Vec<&[u8]> = left_slides.iter().map(|(_, d)| d.as_slice()).collect();
            let right_refs: Vec<&[u8]> = right_slides.iter().map(|(_, d)| d.as_slice()).collect();

            powerpoint::compare::compare_presentations(&left_refs, &right_refs).map_err(
                |e| OfficeError::XmlParse {
                    path: left.to_path_buf(),
                    message: format!("PowerPoint XML parse error: {}", e),
                },
            )?
        }
        OfficeFormat::Excel => {
            // Collect and sort sheet XML parts from both archives.
            let left_sheets = collect_sheets(&left_archive);
            let right_sheets = collect_sheets(&right_archive);

            // Convert to the (name, &[u8]) tuples expected by compare_workbooks.
            let left_refs: Vec<(String, &[u8])> = left_sheets
                .iter()
                .map(|(n, d)| (n.clone(), d.as_slice()))
                .collect();
            let right_refs: Vec<(String, &[u8])> = right_sheets
                .iter()
                .map(|(n, d)| (n.clone(), d.as_slice()))
                .collect();

            excel::compare::compare_workbooks(&left_refs, &right_refs).map_err(
                |e| OfficeError::XmlParse {
                    path: left.to_path_buf(),
                    message: format!("Excel XML parse error: {}", e),
                },
            )?
        }
    };

    let summary = compute_summary(&sections);

    Ok(OfficeDiffResult {
        format,
        summary,
        sections,
        warnings,
        unsupported_features: unsupported,
    })
}

/// Collect and sort slide XML parts from a PowerPoint archive.
///
/// Finds all entries matching `ppt/slides/slideN.xml`, extracts the slide
/// number `N`, and returns them in ascending numeric order.
fn collect_slides(archive: &unpack::SafeZip) -> Vec<(String, Vec<u8>)> {
    let mut slides: Vec<(u32, String, Vec<u8>)> = Vec::new();

    for name in archive.entry_names() {
        // Match ppt/slides/slideN.xml where N is a positive integer.
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            let stem = &name["ppt/slides/slide".len()..name.len() - ".xml".len()];
            if let Ok(n) = stem.parse::<u32>() {
                if let Some(entry) = archive.get(name) {
                    slides.push((n, entry.name.clone(), entry.data.clone()));
                }
            }
        }
    }

    // Sort by slide number.
    slides.sort_by_key(|(n, _, _)| *n);

    slides
        .into_iter()
        .map(|(_, name, data)| (name, data))
        .collect()
}

/// Collect and sort sheet XML parts from an Excel workbook archive.
///
/// Finds all entries matching `xl/worksheets/sheetN.xml`, extracts the
/// sheet number `N`, reads the sheet name from `xl/workbook.xml` if
/// available, and returns them in ascending numeric order.
fn collect_sheets(archive: &unpack::SafeZip) -> Vec<(String, Vec<u8>)> {
    // Parse xl/workbook.xml to get sheet names (name → sheetId).
    let sheet_names = if let Some(entry) = archive.get("xl/workbook.xml") {
        parse_sheet_names(&entry.data)
    } else {
        std::collections::BTreeMap::new()
    };

    let mut sheets: Vec<(u32, String, Vec<u8>)> = Vec::new();

    for name in archive.entry_names() {
        // Match xl/worksheets/sheetN.xml where N is a positive integer.
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            let stem =
                &name["xl/worksheets/sheet".len()..name.len() - ".xml".len()];
            if let Ok(n) = stem.parse::<u32>() {
                if let Some(entry) = archive.get(name) {
                    let sheet_label = sheet_names
                        .get(&n)
                        .cloned()
                        .unwrap_or_else(|| format!("Sheet{}", n));
                    sheets.push((n, sheet_label, entry.data.clone()));
                }
            }
        }
    }

    // Sort by sheet number.
    sheets.sort_by_key(|(n, _, _)| *n);

    sheets
        .into_iter()
        .map(|(_, name, data)| (name, data))
        .collect()
}

/// Parse sheet names from `xl/workbook.xml`.
///
/// Returns a map of `sheetId → name` (e.g., `1 → "Revenue"`).
fn parse_sheet_names(data: &[u8]) -> std::collections::BTreeMap<u32, String> {
    use quick_xml::events::Event;

    let mut names = std::collections::BTreeMap::new();
    let mut reader = xml_util::create_reader(data);
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                if e.name().as_ref() == b"sheet" {
                    let mut name = String::new();
                    let mut sheet_id = 0u32;
                    for attr in e.attributes().flatten() {
                        let key = attr.key.as_ref();
                        if key == b"name" {
                            name = String::from_utf8_lossy(&attr.value).to_string();
                        } else if key == b"sheetId" {
                            if let Ok(n) = String::from_utf8_lossy(&attr.value)
                                .parse::<u32>()
                            {
                                sheet_id = n;
                            }
                        }
                    }
                    if sheet_id > 0 && !name.is_empty() {
                        names.insert(sheet_id, name);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    names
}

/// Compute summary statistics from a list of section diffs.
fn compute_summary(sections: &[models::OfficeSectionDiff]) -> OfficeDiffSummary {
    let sections_total = sections.len();
    let mut sections_added = 0usize;
    let mut sections_deleted = 0usize;
    let mut sections_changed = 0usize;
    let mut sections_unchanged = 0usize;
    let mut total_atomic_changes = 0usize;

    for section in sections {
        match section.status {
            ChangeStatus::Added => sections_added += 1,
            ChangeStatus::Deleted => sections_deleted += 1,
            ChangeStatus::Modified => sections_changed += 1,
            ChangeStatus::Unchanged => sections_unchanged += 1,
            ChangeStatus::Reordered => sections_changed += 1,
        }
        total_atomic_changes += section.changes.len();
    }

    OfficeDiffSummary {
        sections_total,
        sections_added,
        sections_deleted,
        sections_changed,
        sections_unchanged,
        total_atomic_changes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{OfficeAtomicChangeKind, OfficeFormat, OfficeSectionKind};
    use std::io::Write;

    /// Create a minimal valid .docx zip with the given WordprocessingML body content.
    fn docx_with_body(body_xml: &str) -> Vec<u8> {
        let document_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
{}
  </w:body>
</w:document>"#,
            body_xml
        );

        let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

        let mut buf = Vec::new();
        {
            let mut writer =
                zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer.start_file("[Content_Types].xml", options).unwrap();
            writer.write_all(content_types.as_bytes()).unwrap();
            writer.start_file("_rels/.rels", options).unwrap();
            writer.write_all(rels.as_bytes()).unwrap();
            writer.start_file("word/document.xml", options).unwrap();
            writer.write_all(document_xml.as_bytes()).unwrap();
            writer.finish().unwrap();
        }
        buf
    }

    #[test]
    fn test_word_compare_produces_real_results() {
        let tmp = tempfile::tempdir().unwrap();
        let left_path = tmp.path().join("left.docx");
        let right_path = tmp.path().join("right.docx");

        // Generate minimal but valid docx files with known content.
        let left_data = docx_with_body(r#"    <w:p><w:r><w:t>Left text</w:t></w:r></w:p>"#);
        let right_data = docx_with_body(r#"    <w:p><w:r><w:t>Right text</w:t></w:r></w:p>"#);
        std::fs::write(&left_path, &left_data).unwrap();
        std::fs::write(&right_path, &right_data).unwrap();

        let result = compare_office_files(
            left_path,
            right_path,
            OfficeDiffOptions::default(),
        )
        .unwrap();

        assert_eq!(result.format, OfficeFormat::Word);
        assert_eq!(result.summary.sections_total, 1);
        assert_eq!(result.summary.sections_changed, 1);
        assert_eq!(result.summary.total_atomic_changes, 1);
        assert_eq!(result.sections.len(), 1);
        assert_eq!(result.sections[0].changes.len(), 1);
        assert_eq!(
            result.sections[0].changes[0].before.as_deref(),
            Some("Left text")
        );
        assert_eq!(
            result.sections[0].changes[0].after.as_deref(),
            Some("Right text")
        );
        // No Phase 1 stub warning — we have real results now.
        assert!(
            !result
                .warnings
                .iter()
                .any(|w| w.contains("Phase 1")),
            "should NOT have Phase 1 warning"
        );
    }

    /// Create a minimal valid .pptx zip with a single slide.
    fn pptx_with_slide(title: &str, body: &str) -> Vec<u8> {
        let slide_xml = pptx_slide_xml(title, body);
        let presentation_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>
    <p:sldId id="256" r:id="rId1"/>
  </p:sldIdLst>
</p:presentation>"#;

        let rels_root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#;

        let rels_ppt = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
</Relationships>"#;

        let rels_slide = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>
"#;

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#;

        let mut buf = Vec::new();
        {
            let mut writer =
                zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer.start_file("[Content_Types].xml", options).unwrap();
            writer.write_all(content_types.as_bytes()).unwrap();
            writer.start_file("_rels/.rels", options).unwrap();
            writer.write_all(rels_root.as_bytes()).unwrap();
            writer.start_file("ppt/presentation.xml", options).unwrap();
            writer.write_all(presentation_xml.as_bytes()).unwrap();
            writer.start_file("ppt/_rels/presentation.xml.rels", options).unwrap();
            writer.write_all(rels_ppt.as_bytes()).unwrap();
            writer.start_file("ppt/slides/slide1.xml", options).unwrap();
            writer.write_all(slide_xml.as_bytes()).unwrap();
            writer.start_file("ppt/slides/_rels/slide1.xml.rels", options).unwrap();
            writer.write_all(rels_slide.as_bytes()).unwrap();
            writer.finish().unwrap();
        }
        buf
    }

    /// Build a minimal slide XML with a title placeholder and body shape.
    fn pptx_slide_xml(title: &str, body: &str) -> String {
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

    /// Create a minimal valid .xlsx zip with inline-string cells.
    fn xlsx_with_cells(sheet_name: &str, cells: &[(u32, u32, &str)]) -> Vec<u8> {
        // Group cells by row and sort.
        let mut row_map: std::collections::BTreeMap<u32, Vec<(u32, &str)>> =
            std::collections::BTreeMap::new();
        for (row, col, val) in cells {
            row_map.entry(*row).or_default().push((*col, val));
        }

        let rows_xml: String = row_map
            .iter()
            .map(|(row, cols)| {
                let cols_xml: String = cols
                    .iter()
                    .map(|(col, val)| {
                        format!(
                            r#"      <c r="{col_ref}{row}" t="inlineStr">
        <is><t>{val}</t></is>
      </c>"#,
                            col_ref = xlsx_col_letter(*col),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(r#"    <row r="{row}">
{}
    </row>"#, cols_xml)
            })
            .collect::<Vec<_>>()
            .join("\n");

        let sheet_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
           xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheetData>
{}
  </sheetData>
</worksheet>"#,
            rows_xml
        );

        let workbook_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="{name}" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#,
            name = sheet_name
        );

        let rels_root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

        let rels_xl = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#;

        let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
</Types>"#;

        let mut buf = Vec::new();
        {
            let mut writer =
                zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer.start_file("[Content_Types].xml", options).unwrap();
            writer.write_all(content_types.as_bytes()).unwrap();
            writer.start_file("_rels/.rels", options).unwrap();
            writer.write_all(rels_root.as_bytes()).unwrap();
            writer.start_file("xl/workbook.xml", options).unwrap();
            writer.write_all(workbook_xml.as_bytes()).unwrap();
            writer.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
            writer.write_all(rels_xl.as_bytes()).unwrap();
            writer.start_file("xl/worksheets/sheet1.xml", options).unwrap();
            writer.write_all(sheet_xml.as_bytes()).unwrap();
            writer.finish().unwrap();
        }
        buf
    }

    /// Convert a 1-based column number to Excel column letter.
    /// Uses the shared implementation from `excel::extract`.
    fn xlsx_col_letter(col: u32) -> String {
        crate::excel::extract::col_to_letter(col)
    }

    /// Create a minimal valid .pptx zip with multiple slides.
    fn pptx_with_slides(slides: &[(&str, &str)]) -> Vec<u8> {
        let n = slides.len();

        // Build content types with overrides for each slide.
        let slide_overrides: String = (1..=n)
            .map(|i| {
                format!(
                    r#"  <Override PartName="/ppt/slides/slide{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let content_types = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
{}
</Types>"#,
            slide_overrides
        );

        // Build presentation.xml with sldIdLst entries.
        let sld_ids: String = (1..=n)
            .map(|i| {
                format!(
                    r#"    <p:sldId id="{id}" r:id="rId{i}"/>"#,
                    id = 255 + i
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let presentation_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:sldIdLst>
{}
  </p:sldIdLst>
</p:presentation>"#,
            sld_ids
        );

        // Build presentation.xml.rels with entries for each slide.
        let slide_rels: String = (1..=n)
            .map(|i| {
                format!(
                    r#"  <Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{i}.xml"/>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let rels_ppt = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
{}
</Relationships>"#,
            slide_rels
        );

        let rels_root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#;

        let rels_slide = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>
"#;

        let mut buf = Vec::new();
        {
            let mut writer =
                zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer
                .start_file("[Content_Types].xml", options)
                .unwrap();
            writer.write_all(content_types.as_bytes()).unwrap();
            writer.start_file("_rels/.rels", options).unwrap();
            writer.write_all(rels_root.as_bytes()).unwrap();
            writer
                .start_file("ppt/presentation.xml", options)
                .unwrap();
            writer
                .write_all(presentation_xml.as_bytes())
                .unwrap();
            writer
                .start_file("ppt/_rels/presentation.xml.rels", options)
                .unwrap();
            writer.write_all(rels_ppt.as_bytes()).unwrap();

            for (i, (title, body)) in slides.iter().enumerate() {
                let num = i + 1;
                let slide_xml = pptx_slide_xml(title, body);
                writer
                    .start_file(
                        &format!("ppt/slides/slide{}.xml", num),
                        options,
                    )
                    .unwrap();
                writer.write_all(slide_xml.as_bytes()).unwrap();
                writer
                    .start_file(
                        &format!("ppt/slides/_rels/slide{}.xml.rels", num),
                        options,
                    )
                    .unwrap();
                writer.write_all(rels_slide.as_bytes()).unwrap();
            }

            writer.finish().unwrap();
        }
        buf
    }

    /// Create a minimal valid .xlsx zip with multiple sheets.
    /// Each entry is (sheet_name, cells).
    fn xlsx_with_sheets(sheets: &[(&str, &[(u32, u32, &str)])]) -> Vec<u8> {
        let n = sheets.len();

        // Build content types with overrides for each sheet.
        let sheet_overrides: String = (1..=n)
            .map(|i| {
                format!(
                    r#"  <Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let content_types = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
{}
</Types>"#,
            sheet_overrides
        );

        // Build workbook.xml with <sheet> entries.
        let wb_sheets: String = sheets
            .iter()
            .enumerate()
            .map(|(i, (name, _))| {
                let sid = i + 1;
                format!(
                    r#"    <sheet name="{name}" sheetId="{sid}" r:id="rId{sid}"/>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let workbook_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
{}
  </sheets>
</workbook>"#,
            wb_sheets
        );

        // Build workbook.xml.rels with entries for each sheet.
        let wb_rels: String = (1..=n)
            .map(|i| {
                format!(
                    r#"  <Relationship Id="rId{i}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i}.xml"/>"#
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let rels_xl = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
{}
</Relationships>"#,
            wb_rels
        );

        let rels_root = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

        let mut buf = Vec::new();
        {
            let mut writer =
                zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer
                .start_file("[Content_Types].xml", options)
                .unwrap();
            writer.write_all(content_types.as_bytes()).unwrap();
            writer.start_file("_rels/.rels", options).unwrap();
            writer.write_all(rels_root.as_bytes()).unwrap();
            writer
                .start_file("xl/workbook.xml", options)
                .unwrap();
            writer.write_all(workbook_xml.as_bytes()).unwrap();
            writer
                .start_file("xl/_rels/workbook.xml.rels", options)
                .unwrap();
            writer.write_all(rels_xl.as_bytes()).unwrap();

            for (i, (_, cells)) in sheets.iter().enumerate() {
                let num = i + 1;
                let sheet_xml = sheet_xml_from_cells(cells);
                writer
                    .start_file(
                        &format!("xl/worksheets/sheet{}.xml", num),
                        options,
                    )
                    .unwrap();
                writer.write_all(sheet_xml.as_bytes()).unwrap();
            }

            writer.finish().unwrap();
        }
        buf
    }

    /// Build a worksheet XML string from a list of (row, col, value) tuples.
    fn sheet_xml_from_cells(cells: &[(u32, u32, &str)]) -> String {
        let mut row_map: std::collections::BTreeMap<u32, Vec<(u32, &str)>> =
            std::collections::BTreeMap::new();
        for (row, col, val) in cells {
            row_map.entry(*row).or_default().push((*col, val));
        }

        let rows_xml: String = row_map
            .iter()
            .map(|(row, cols)| {
                let cols_xml: String = cols
                    .iter()
                    .map(|(col, val)| {
                        format!(
                            r#"      <c r="{col_ref}{row}" t="inlineStr">
        <is><t>{val}</t></is>
      </c>"#,
                            col_ref = xlsx_col_letter(*col),
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
            rows_xml
        )
    }

    #[test]
    fn test_pptx_compare_produces_real_results() {
        let tmp = tempfile::tempdir().unwrap();
        let left_path = tmp.path().join("left.pptx");
        let right_path = tmp.path().join("right.pptx");

        let left_data = pptx_with_slide("Quarterly Review", "Revenue grew 15%.");
        let right_data = pptx_with_slide("Annual Review", "Revenue grew 22%.");
        std::fs::write(&left_path, &left_data).unwrap();
        std::fs::write(&right_path, &right_data).unwrap();

        let result = compare_office_files(
            left_path,
            right_path,
            OfficeDiffOptions::default(),
        )
        .unwrap();

        assert_eq!(result.format, OfficeFormat::PowerPoint);
        assert_eq!(result.summary.sections_total, 1);
        assert_eq!(result.summary.sections_changed, 1);
        assert_eq!(result.summary.total_atomic_changes, 1);
        assert_eq!(result.sections.len(), 1);
        assert_eq!(result.sections[0].kind, OfficeSectionKind::PowerPointSlide);
        assert_eq!(result.sections[0].status, ChangeStatus::Modified);
        assert_eq!(result.sections[0].changes.len(), 1);
        let before = result.sections[0].changes[0].before.as_deref().unwrap();
        let after = result.sections[0].changes[0].after.as_deref().unwrap();
        // Exact text format: [title] prefix from <p:ph type="title"/>, body shape has no prefix.
        assert_eq!(before, "[title] Quarterly Review\nRevenue grew 15%.");
        assert_eq!(after, "[title] Annual Review\nRevenue grew 22%.");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_xlsx_compare_produces_real_results() {
        let tmp = tempfile::tempdir().unwrap();
        let left_path = tmp.path().join("left.xlsx");
        let right_path = tmp.path().join("right.xlsx");

        let left_data = xlsx_with_cells(
            "Revenue",
            &[
                (1, 1, "Q1 Revenue"),
                (1, 2, "100"),
                (2, 1, "Q2 Revenue"),
                (2, 2, "120"),
            ],
        );
        let right_data = xlsx_with_cells(
            "Revenue",
            &[
                (1, 1, "Q1 Revenue"),
                (1, 2, "100"),
                (2, 1, "Q2 Revenue"),
                (2, 2, "135"),
            ],
        );
        std::fs::write(&left_path, &left_data).unwrap();
        std::fs::write(&right_path, &right_data).unwrap();

        let result = compare_office_files(
            left_path,
            right_path,
            OfficeDiffOptions::default(),
        )
        .unwrap();

        assert_eq!(result.format, OfficeFormat::Excel);
        assert_eq!(result.summary.sections_total, 1);
        assert_eq!(result.summary.sections_changed, 1);
        assert_eq!(result.summary.total_atomic_changes, 1);
        assert_eq!(result.sections.len(), 1);
        assert_eq!(result.sections[0].kind, OfficeSectionKind::ExcelSheet);
        assert_eq!(result.sections[0].status, ChangeStatus::Modified);
        assert_eq!(result.sections[0].label, "Revenue");
        assert_eq!(result.sections[0].changes.len(), 1);
        let before = result.sections[0].changes[0].before.as_deref().unwrap();
        let after = result.sections[0].changes[0].after.as_deref().unwrap();
        // Exact tab-separated row format from extract_sheet_text.
        assert_eq!(before, "Q1 Revenue\t100\nQ2 Revenue\t120");
        assert_eq!(after, "Q1 Revenue\t100\nQ2 Revenue\t135");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_multi_slide_pptx_mixed() {
        // Left: 3 slides (Title, Agenda v1, Summary).
        // Right: 3 slides (Title unchanged, Agenda v2 modified, Q&A replaces Summary).
        // Expected: Slide 1 Unchanged, Slide 2 Modified, Slide 3 Modified.
        let tmp = tempfile::tempdir().unwrap();
        let left_path = tmp.path().join("left.pptx");
        let right_path = tmp.path().join("right.pptx");

        let left_data = pptx_with_slides(&[
            ("Title", "Welcome"),
            ("Agenda", "Topic A, Topic B"),
            ("Summary", "Wrap-up notes"),
        ]);
        let right_data = pptx_with_slides(&[
            ("Title", "Welcome"),
            ("Agenda", "Topic A, Topic B, Topic C"),
            ("Q&A", "Questions?"),
        ]);
        std::fs::write(&left_path, &left_data).unwrap();
        std::fs::write(&right_path, &right_data).unwrap();

        let result = compare_office_files(
            left_path,
            right_path,
            OfficeDiffOptions::default(),
        )
        .unwrap();

        assert_eq!(result.format, OfficeFormat::PowerPoint);
        assert_eq!(result.summary.sections_total, 3);
        assert_eq!(result.summary.sections_unchanged, 1);
        assert_eq!(result.summary.sections_changed, 2);
        assert_eq!(result.summary.sections_added, 0);
        assert_eq!(result.summary.sections_deleted, 0);
        assert_eq!(result.summary.total_atomic_changes, 2);
        assert_eq!(result.sections.len(), 3);

        assert_eq!(result.sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(result.sections[0].changes.len(), 0);
        assert!(result.sections[0].label.contains("Title"));

        assert_eq!(result.sections[1].status, ChangeStatus::Modified);
        assert_eq!(result.sections[1].changes.len(), 1);
        assert_eq!(
            result.sections[1].changes[0].before.as_deref().unwrap(),
            "[title] Agenda\nTopic A, Topic B"
        );
        assert_eq!(
            result.sections[1].changes[0].after.as_deref().unwrap(),
            "[title] Agenda\nTopic A, Topic B, Topic C"
        );

        assert_eq!(result.sections[2].status, ChangeStatus::Modified);
        assert_eq!(result.sections[2].changes.len(), 1);
        assert_eq!(
            result.sections[2].changes[0].before.as_deref().unwrap(),
            "[title] Summary\nWrap-up notes"
        );
        assert_eq!(
            result.sections[2].changes[0].after.as_deref().unwrap(),
            "[title] Q&A\nQuestions?"
        );
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_multi_slide_pptx_deleted() {
        // Left 3 slides, right 2 slides → Slide 3 is Deleted.
        // Expected: 1 Unchanged, 1 Modified, 1 Deleted.
        let tmp = tempfile::tempdir().unwrap();
        let left_path = tmp.path().join("left.pptx");
        let right_path = tmp.path().join("right.pptx");

        let left_data = pptx_with_slides(&[
            ("Intro", "Welcome everyone"),
            ("Old Slide", "Removed content"),
            ("Appendix", "Extra details"),
        ]);
        let right_data = pptx_with_slides(&[
            ("Intro", "Welcome everyone"),
            ("Closing", "Thank you"),
        ]);
        std::fs::write(&left_path, &left_data).unwrap();
        std::fs::write(&right_path, &right_data).unwrap();

        let result = compare_office_files(
            left_path,
            right_path,
            OfficeDiffOptions::default(),
        )
        .unwrap();

        assert_eq!(result.format, OfficeFormat::PowerPoint);
        assert_eq!(result.summary.sections_total, 3);
        assert_eq!(result.summary.sections_unchanged, 1);
        assert_eq!(result.summary.sections_changed, 1);
        assert_eq!(result.summary.sections_deleted, 1);
        assert_eq!(result.summary.sections_added, 0);
        assert_eq!(result.summary.total_atomic_changes, 2);
        assert_eq!(result.sections.len(), 3);

        assert_eq!(result.sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(result.sections[0].changes.len(), 0);
        assert!(result.sections[0].label.contains("Intro"));

        assert_eq!(result.sections[1].status, ChangeStatus::Modified);
        assert_eq!(result.sections[1].changes.len(), 1);
        assert!(result.sections[1].label.contains("Old Slide"));
        assert_eq!(
            result.sections[1].changes[0].before.as_deref().unwrap(),
            "[title] Old Slide\nRemoved content"
        );
        assert_eq!(
            result.sections[1].changes[0].after.as_deref().unwrap(),
            "[title] Closing\nThank you"
        );

        assert_eq!(result.sections[2].status, ChangeStatus::Deleted);
        assert_eq!(result.sections[2].changes.len(), 1);
        assert_eq!(
            result.sections[2].changes[0].kind,
            OfficeAtomicChangeKind::TextDeleted
        );
        assert!(result.sections[2].label.contains("Appendix"));
        assert_eq!(
            result.sections[2].changes[0].before.as_deref().unwrap(),
            "[title] Appendix\nExtra details"
        );
        assert_eq!(result.sections[2].changes[0].after, None);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_multi_slide_pptx_added() {
        // Left 2 slides, right 3 slides → Slide 3 is Added.
        // Expected: 1 Unchanged, 1 Modified, 1 Added.
        let tmp = tempfile::tempdir().unwrap();
        let left_path = tmp.path().join("left.pptx");
        let right_path = tmp.path().join("right.pptx");

        let left_data = pptx_with_slides(&[
            ("Intro", "Welcome everyone"),
            ("Agenda", "Topics A, B"),
        ]);
        let right_data = pptx_with_slides(&[
            ("Intro", "Welcome everyone"),
            ("Agenda", "Topics A, B, C"),
            ("Bonus", "Surprise!"),
        ]);
        std::fs::write(&left_path, &left_data).unwrap();
        std::fs::write(&right_path, &right_data).unwrap();

        let result = compare_office_files(
            left_path,
            right_path,
            OfficeDiffOptions::default(),
        )
        .unwrap();

        assert_eq!(result.format, OfficeFormat::PowerPoint);
        assert_eq!(result.summary.sections_total, 3);
        assert_eq!(result.summary.sections_unchanged, 1);
        assert_eq!(result.summary.sections_changed, 1);
        assert_eq!(result.summary.sections_added, 1);
        assert_eq!(result.summary.sections_deleted, 0);
        assert_eq!(result.summary.total_atomic_changes, 2);
        assert_eq!(result.sections.len(), 3);

        assert_eq!(result.sections[0].status, ChangeStatus::Unchanged);
        assert_eq!(result.sections[0].changes.len(), 0);
        assert!(result.sections[0].label.contains("Intro"));

        assert_eq!(result.sections[1].status, ChangeStatus::Modified);
        assert_eq!(result.sections[1].changes.len(), 1);
        assert!(result.sections[1].label.contains("Agenda"));
        assert_eq!(
            result.sections[1].changes[0].before.as_deref().unwrap(),
            "[title] Agenda\nTopics A, B"
        );
        assert_eq!(
            result.sections[1].changes[0].after.as_deref().unwrap(),
            "[title] Agenda\nTopics A, B, C"
        );

        assert_eq!(result.sections[2].status, ChangeStatus::Added);
        assert_eq!(result.sections[2].changes.len(), 1);
        assert_eq!(
            result.sections[2].changes[0].kind,
            OfficeAtomicChangeKind::TextAdded
        );
        assert!(result.sections[2].label.contains("Bonus"));
        assert_eq!(result.sections[2].changes[0].before, None);
        assert_eq!(
            result.sections[2].changes[0].after.as_deref().unwrap(),
            "[title] Bonus\nSurprise!"
        );
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_multi_sheet_xlsx_compare() {
        // Left: 3 sheets (Revenue, Expenses, Summary).
        // Right: 3 sheets (Revenue modified, Expenses unchanged, NewSheet replaces Summary).
        // Then also check sheet labels and names.
        let tmp = tempfile::tempdir().unwrap();
        let left_path = tmp.path().join("left.xlsx");
        let right_path = tmp.path().join("right.xlsx");

        let left_data = xlsx_with_sheets(&[
            (
                "Revenue",
                &[
                    (1, 1, "Q1"),
                    (1, 2, "100"),
                    (2, 1, "Q2"),
                    (2, 2, "120"),
                ][..],
            ),
            (
                "Expenses",
                &[(1, 1, "Category"), (1, 2, "Amount"), (2, 1, "Travel"), (2, 2, "50")][..],
            ),
            (
                "Summary",
                &[(1, 1, "Total"), (1, 2, "70")][..],
            ),
        ]);
        let right_data = xlsx_with_sheets(&[
            (
                "Revenue",
                &[
                    (1, 1, "Q1"),
                    (1, 2, "100"),
                    (2, 1, "Q2"),
                    (2, 2, "135"),
                ][..],
            ),
            (
                "Expenses",
                &[(1, 1, "Category"), (1, 2, "Amount"), (2, 1, "Travel"), (2, 2, "50")][..],
            ),
            (
                "NewSheet",
                &[(1, 1, "Bonus"), (1, 2, "Data")][..],
            ),
        ]);
        std::fs::write(&left_path, &left_data).unwrap();
        std::fs::write(&right_path, &right_data).unwrap();

        let result = compare_office_files(
            left_path,
            right_path,
            OfficeDiffOptions::default(),
        )
        .unwrap();

        assert_eq!(result.format, OfficeFormat::Excel);
        assert_eq!(result.summary.sections_total, 3);
        assert_eq!(result.summary.sections_unchanged, 1);
        assert_eq!(result.summary.sections_changed, 2);
        assert_eq!(result.summary.sections_added, 0);
        assert_eq!(result.summary.sections_deleted, 0);
        assert_eq!(result.summary.total_atomic_changes, 2);
        assert_eq!(result.sections.len(), 3);

        // Sheet 1 (Revenue): Modified — Q2 value changed from 120 → 135.
        assert_eq!(result.sections[0].status, ChangeStatus::Modified);
        assert_eq!(result.sections[0].kind, OfficeSectionKind::ExcelSheet);
        assert_eq!(result.sections[0].label, "Revenue");
        assert_eq!(result.sections[0].changes.len(), 1);
        // Exact tab-separated row format.
        assert_eq!(
            result.sections[0].changes[0].before.as_deref().unwrap(),
            "Q1\t100\nQ2\t120"
        );
        assert_eq!(
            result.sections[0].changes[0].after.as_deref().unwrap(),
            "Q1\t100\nQ2\t135"
        );

        // Sheet 2 (Expenses): Unchanged.
        assert_eq!(result.sections[1].status, ChangeStatus::Unchanged);
        assert_eq!(result.sections[1].label, "Expenses");
        assert_eq!(result.sections[1].changes.len(), 0);

        // Sheet 3: left=Summary, right=NewSheet → Modified (both exist at index 2).
        assert_eq!(result.sections[2].status, ChangeStatus::Modified);
        assert_eq!(result.sections[2].label, "Summary");
        assert_eq!(result.sections[2].changes.len(), 1);
        assert_eq!(
            result.sections[2].changes[0].before.as_deref().unwrap(),
            "Total\t70"
        );
        assert_eq!(
            result.sections[2].changes[0].after.as_deref().unwrap(),
            "Bonus\tData"
        );
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_mismatched_formats() {
        let tmp = tempfile::tempdir().unwrap();
        let left_path = tmp.path().join("test.docx");
        let right_path = tmp.path().join("test.pptx");
        let left_data = docx_with_body(r#"    <w:p><w:r><w:t>text</w:t></w:r></w:p>"#);
        std::fs::write(&left_path, &left_data).unwrap();
        // Write the same docx data as pptx — format detection will catch it.
        std::fs::write(&right_path, &left_data).unwrap();

        let result = compare_office_files(left_path, right_path, OfficeDiffOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("test.xyz");
        std::fs::write(&path, b"not an office file").unwrap();

        let result = format::detect_format(&path);
        assert!(result.is_err());
    }
}
