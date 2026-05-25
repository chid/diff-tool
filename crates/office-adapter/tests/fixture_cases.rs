//! Fixture-driven tests for the office-adapter crate.
//!
//! These tests use hand-authored minimal OOXML fixture files generated from
//! XML templates in `common/mod.rs`. Each fixture case has:
//! - `left.{docx,pptx,xlsx}` and `right.{docx,pptx,xlsx}` — the Office files
//! - `case.json` — expected diff metadata
//!
//! The fixture binary files are generated on first run if they don't exist.
//! This avoids checking binary zip files into version control while still
//! exercising the full read-from-disk pipeline.

mod common;

use office_adapter::{compare_office_files, format, models::*, unpack};
use serde::Deserialize;
use std::path::{Path, PathBuf};

// ── Fixture metadata ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CaseMeta {
    name: String,
    format: String,
    expected: ExpectedStats,
}

#[derive(Debug, Deserialize)]
struct ExpectedStats {
    sections_total: usize,
    sections_added: usize,
    sections_deleted: usize,
    sections_changed: usize,
    sections_unchanged: usize,
    total_atomic_changes: usize,
}

// ── Fixture file generation (on-demand) ────────────────────────────────

/// Ensure all fixture binary files exist on disk. Generates them if missing.
fn ensure_fixtures() {
    // DOCX
    let word_dir = fixture_root().join("office-word-basic");
    let left_docx = word_dir.join("left.docx");
    let right_docx = word_dir.join("right.docx");
    if !left_docx.exists() {
        std::fs::write(&left_docx, common::generate_docx(common::docx_left_body())).unwrap();
    }
    if !right_docx.exists() {
        std::fs::write(&right_docx, common::generate_docx(common::docx_right_body())).unwrap();
    }

    // PPTX
    let pptx_dir = fixture_root().join("office-pptx-basic");
    let left_pptx = pptx_dir.join("left.pptx");
    let right_pptx = pptx_dir.join("right.pptx");
    if !left_pptx.exists() {
        let slide = common::pptx_slide_with_title("Left Title", "Left slide body content.");
        std::fs::write(&left_pptx, common::generate_pptx(&slide)).unwrap();
    }
    if !right_pptx.exists() {
        let slide = common::pptx_slide_with_title("Right Title", "Right slide body content.");
        std::fs::write(&right_pptx, common::generate_pptx(&slide)).unwrap();
    }

    // XLSX
    let xlsx_dir = fixture_root().join("office-xlsx-basic");
    let left_xlsx = xlsx_dir.join("left.xlsx");
    let right_xlsx = xlsx_dir.join("right.xlsx");
    if !left_xlsx.exists() {
        let sheet = common::xlsx_sheet_with_cells(&[
            (1, 1, "left value"),
            (1, 2, "unchanged B1"),
            (2, 1, "unchanged A2"),
        ]);
        std::fs::write(&left_xlsx, common::generate_xlsx("Sheet1", &sheet)).unwrap();
    }
    if !right_xlsx.exists() {
        let sheet = common::xlsx_sheet_with_cells(&[
            (1, 1, "right value"),
            (1, 2, "unchanged B1"),
            (2, 1, "unchanged A2"),
        ]);
        std::fs::write(&right_xlsx, common::generate_xlsx("Sheet1", &sheet)).unwrap();
    }
}

// ── Fixture discovery ──────────────────────────────────────────────────

fn fixture_case_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(fixture_root())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let case_json = path.join("case.json");
            if !case_json.exists() {
                return false;
            }
            // Only include directories whose case.json has a "format" field
            // (i.e., office-specific fixtures). Non-office fixtures have a
            // different schema.
            std::fs::read_to_string(&case_json)
                .map(|s| s.contains("\"format\""))
                .unwrap_or(false)
        })
        .collect();
    dirs.sort();
    dirs
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/cases")
}

// ── Tests: Format detection from disk ──────────────────────────────────

#[test]
fn fixture_format_detection_is_correct() {
    ensure_fixtures();

    for case_dir in fixture_case_dirs() {
        let meta: CaseMeta =
            serde_json::from_str(&std::fs::read_to_string(case_dir.join("case.json")).unwrap())
                .unwrap();

        // Find the left file with the correct extension.
        let left_file = find_office_file(&case_dir, "left").expect("missing left file");

        let detected = format::detect_format(&left_file).unwrap_or_else(|e| {
            panic!("{}: format detection failed: {:?}", meta.name, e)
        });

        let expected_format = match meta.format.as_str() {
            "word" => OfficeFormat::Word,
            "powerpoint" => OfficeFormat::PowerPoint,
            "excel" => OfficeFormat::Excel,
            other => panic!("unknown format in case.json: {}", other),
        };

        assert_eq!(
            detected, expected_format,
            "{}: expected {:?}, got {:?}",
            meta.name, expected_format, detected
        );
    }
}

// ── Tests: Zip unpacking from disk ─────────────────────────────────────

#[test]
fn fixture_zip_unpacking_reads_expected_parts() {
    ensure_fixtures();

    let word_dir = fixture_root().join("office-word-basic");
    let left_docx = word_dir.join("left.docx");
    let data = std::fs::read(&left_docx).unwrap();
    let archive =
        unpack::read_zip_safe(&left_docx, &data, &OfficeDiffOptions::default()).unwrap();

    assert!(
        archive.contains("[Content_Types].xml"),
        "should contain content types"
    );
    assert!(
        archive.contains("_rels/.rels"),
        "should contain package relationships"
    );
    assert!(
        archive.contains("word/document.xml"),
        "should contain document XML"
    );

    // Verify document.xml content.
    let entry = archive.get("word/document.xml").unwrap();
    let xml = std::str::from_utf8(&entry.data).unwrap();
    assert!(
        xml.contains("Hello from the left file."),
        "document.xml should contain expected text"
    );
}

#[test]
fn fixture_pptx_unpacking_reads_slide_content() {
    ensure_fixtures();

    let pptx_dir = fixture_root().join("office-pptx-basic");
    let left_pptx = pptx_dir.join("left.pptx");
    let data = std::fs::read(&left_pptx).unwrap();
    let archive =
        unpack::read_zip_safe(&left_pptx, &data, &OfficeDiffOptions::default()).unwrap();

    assert!(archive.contains("ppt/presentation.xml"));
    assert!(archive.contains("ppt/slides/slide1.xml"));

    let slide = archive.get("ppt/slides/slide1.xml").unwrap();
    let xml = std::str::from_utf8(&slide.data).unwrap();
    assert!(xml.contains("Left Title"));
    assert!(xml.contains("Left slide body content."));
}

#[test]
fn fixture_xlsx_unpacking_reads_sheet_content() {
    ensure_fixtures();

    let xlsx_dir = fixture_root().join("office-xlsx-basic");
    let left_xlsx = xlsx_dir.join("left.xlsx");
    let data = std::fs::read(&left_xlsx).unwrap();
    let archive =
        unpack::read_zip_safe(&left_xlsx, &data, &OfficeDiffOptions::default()).unwrap();

    assert!(archive.contains("xl/workbook.xml"));
    assert!(archive.contains("xl/worksheets/sheet1.xml"));

    let sheet = archive.get("xl/worksheets/sheet1.xml").unwrap();
    let xml = std::str::from_utf8(&sheet.data).unwrap();
    assert!(xml.contains("left value"));
    assert!(xml.contains("unchanged B1"));
}

// ── Tests: compare_office_files (Phase 1 stub) ─────────────────────────

#[test]
fn fixture_compare_office_files_phase1_stub() {
    ensure_fixtures();

    for case_dir in fixture_case_dirs() {
        let meta: CaseMeta =
            serde_json::from_str(&std::fs::read_to_string(case_dir.join("case.json")).unwrap())
                .unwrap();

        let left_file = find_office_file(&case_dir, "left").expect("missing left file");
        let right_file = find_office_file(&case_dir, "right").expect("missing right file");

        let result = compare_office_files(
            left_file.clone(),
            right_file.clone(),
            OfficeDiffOptions::default(),
        )
        .unwrap_or_else(|e| {
            panic!("{}: compare_office_files failed: {:?}", meta.name, e)
        });

        // Phase 1 returns stub results.
        let expected_format = match meta.format.as_str() {
            "word" => OfficeFormat::Word,
            "powerpoint" => OfficeFormat::PowerPoint,
            "excel" => OfficeFormat::Excel,
            other => panic!("unknown format in case.json: {}", other),
        };

        assert_eq!(
            result.format, expected_format,
            "{}: expected format {:?}, got {:?}",
            meta.name, expected_format, result.format
        );

        // Word, PowerPoint, and Excel comparison are now fully implemented.
        assert!(result.summary.sections_total > 0
            || result.warnings.is_empty(),
            "{format} documents should produce real results",
            format = meta.format
        );
        assert!(
            !result.warnings.iter().any(|w| w.contains("Phase 1") || w.contains("not yet implemented")),
            "{format}: should NOT have stub warning",
            format = meta.format
        );

        // Verify summary stats match case.json expectations.
        assert_eq!(
            result.summary.sections_total, meta.expected.sections_total,
            "{}: sections_total mismatch", meta.name
        );
        assert_eq!(
            result.summary.sections_added, meta.expected.sections_added,
            "{}: sections_added mismatch", meta.name
        );
        assert_eq!(
            result.summary.sections_deleted, meta.expected.sections_deleted,
            "{}: sections_deleted mismatch", meta.name
        );
        assert_eq!(
            result.summary.sections_changed, meta.expected.sections_changed,
            "{}: sections_changed mismatch", meta.name
        );
        assert_eq!(
            result.summary.sections_unchanged, meta.expected.sections_unchanged,
            "{}: sections_unchanged mismatch", meta.name
        );
        assert_eq!(
            result.summary.total_atomic_changes, meta.expected.total_atomic_changes,
            "{}: total_atomic_changes mismatch", meta.name
        );

        // Verify no oversized entries for these small fixtures.
        assert!(
            result.unsupported_features.is_empty(),
            "{}: should have no unsupported features, got {:?}",
            meta.name,
            result.unsupported_features
        );
    }
}

// ── Tests: Error paths ─────────────────────────────────────────────────

#[test]
fn fixture_mismatched_formats_error() {
    ensure_fixtures();

    let word_dir = fixture_root().join("office-word-basic");
    let pptx_dir = fixture_root().join("office-pptx-basic");
    let left_docx = word_dir.join("left.docx");
    let right_pptx = pptx_dir.join("left.pptx");

    let result = compare_office_files(left_docx, right_pptx, OfficeDiffOptions::default());
    assert!(result.is_err(), "mismatched formats should error");
}

#[test]
fn fixture_unknown_format_error() {
    ensure_fixtures();

    // Create a .txt file on disk alongside the fixtures.
    let word_dir = fixture_root().join("office-word-basic");
    let txt_file = word_dir.join("notes.txt");
    std::fs::write(&txt_file, b"not an office file").unwrap();

    let result = format::detect_format(&txt_file);
    assert!(result.is_err(), "unknown extension should error");

    // Clean up.
    let _ = std::fs::remove_file(&txt_file);
}

#[test]
fn fixture_corrupt_zip_detection() {
    ensure_fixtures();

    let word_dir = fixture_root().join("office-word-basic");
    let corrupt = word_dir.join("corrupt.docx");
    std::fs::write(&corrupt, b"this is not a zip file").unwrap();

    let result = format::detect_format(&corrupt);
    assert!(
        matches!(result, Err(office_adapter::OfficeError::InvalidZip { .. })),
        "corrupt zip should be rejected"
    );

    let _ = std::fs::remove_file(&corrupt);
}

// ── Tests: file-to-file content verification ───────────────────────────

#[test]
fn fixture_docx_left_and_right_have_known_diff() {
    ensure_fixtures();

    let word_dir = fixture_root().join("office-word-basic");

    let left_data = std::fs::read(word_dir.join("left.docx")).unwrap();
    let right_data = std::fs::read(word_dir.join("right.docx")).unwrap();

    let left_archive =
        unpack::read_zip_safe(&word_dir.join("left.docx"), &left_data, &OfficeDiffOptions::default())
            .unwrap();
    let right_archive = unpack::read_zip_safe(
        &word_dir.join("right.docx"),
        &right_data,
        &OfficeDiffOptions::default(),
    )
    .unwrap();

    let left_xml = std::str::from_utf8(&left_archive.get("word/document.xml").unwrap().data).unwrap();
    let right_xml =
        std::str::from_utf8(&right_archive.get("word/document.xml").unwrap().data).unwrap();

    assert!(left_xml.contains("left file"));
    assert!(right_xml.contains("right file"));
    assert!(!left_xml.contains("right file"));
    assert!(!right_xml.contains("left file"));
}

#[test]
fn fixture_pptx_title_diff_is_present() {
    ensure_fixtures();

    let pptx_dir = fixture_root().join("office-pptx-basic");

    let left_data = std::fs::read(pptx_dir.join("left.pptx")).unwrap();
    let right_data = std::fs::read(pptx_dir.join("right.pptx")).unwrap();

    let left_archive =
        unpack::read_zip_safe(&pptx_dir.join("left.pptx"), &left_data, &OfficeDiffOptions::default())
            .unwrap();
    let right_archive = unpack::read_zip_safe(
        &pptx_dir.join("right.pptx"),
        &right_data,
        &OfficeDiffOptions::default(),
    )
    .unwrap();

    let left_slide =
        std::str::from_utf8(&left_archive.get("ppt/slides/slide1.xml").unwrap().data).unwrap();
    let right_slide =
        std::str::from_utf8(&right_archive.get("ppt/slides/slide1.xml").unwrap().data).unwrap();

    assert!(left_slide.contains("Left Title"));
    assert!(right_slide.contains("Right Title"));
}

#[test]
fn fixture_xlsx_cell_diff_is_present() {
    ensure_fixtures();

    let xlsx_dir = fixture_root().join("office-xlsx-basic");

    let left_data = std::fs::read(xlsx_dir.join("left.xlsx")).unwrap();
    let right_data = std::fs::read(xlsx_dir.join("right.xlsx")).unwrap();

    let left_archive =
        unpack::read_zip_safe(&xlsx_dir.join("left.xlsx"), &left_data, &OfficeDiffOptions::default())
            .unwrap();
    let right_archive = unpack::read_zip_safe(
        &xlsx_dir.join("right.xlsx"),
        &right_data,
        &OfficeDiffOptions::default(),
    )
    .unwrap();

    let left_sheet = std::str::from_utf8(
        &left_archive
            .get("xl/worksheets/sheet1.xml")
            .unwrap()
            .data,
    )
    .unwrap();
    let right_sheet = std::str::from_utf8(
        &right_archive
            .get("xl/worksheets/sheet1.xml")
            .unwrap()
            .data,
    )
    .unwrap();

    assert!(left_sheet.contains("left value"));
    assert!(right_sheet.contains("right value"));
    // Unchanged cells should be identical in both.
    assert!(left_sheet.contains("unchanged B1"));
    assert!(right_sheet.contains("unchanged B1"));
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Find the left or right file in a fixture directory.
/// Scans for `.docx`, `.pptx`, or `.xlsx` with the given prefix.
fn find_office_file(dir: &Path, prefix: &str) -> Option<PathBuf> {
    for ext in &["docx", "pptx", "xlsx"] {
        let path = dir.join(format!("{}.{}", prefix, ext));
        if path.exists() {
            return Some(path);
        }
    }
    None
}
