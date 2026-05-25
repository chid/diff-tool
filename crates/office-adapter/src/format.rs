//! Office document format detection.
//!
//! Detects the Office format from a file path by checking the extension
//! and verifying the zip signature. Modern Office files (`.docx`, `.pptx`,
//! `.xlsx`) are zip archives conforming to the Open Packaging Convention.

use crate::models::OfficeFormat;
use crate::OfficeError;
use std::path::Path;

/// Detect the Office format from a file path.
///
/// Checks the file extension first, then verifies that the file starts
/// with the zip magic bytes (`PK\x03\x04`).
///
/// Returns an error if the extension is not recognized or the file does
/// not appear to be a valid zip archive.
pub fn detect_format(path: &Path) -> Result<OfficeFormat, OfficeError> {
    // 1. Check extension.
    let format = match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("docx") => OfficeFormat::Word,
        Some("pptx") | Some("ppt") => OfficeFormat::PowerPoint,
        Some("xlsx") | Some("xls") => OfficeFormat::Excel,
        _ => {
            return Err(OfficeError::UnknownFormat {
                path: path.to_path_buf(),
            })
        }
    };

    // 2. Verify zip signature.
    verify_zip_signature(path)?;

    Ok(format)
}

/// Check that the file starts with the zip magic bytes `PK\x03\x04`.
fn verify_zip_signature(path: &Path) -> Result<(), OfficeError> {
    let data =
        std::fs::read(path).map_err(|source| OfficeError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?;

    if data.len() < 4 || &data[..4] != b"PK\x03\x04" {
        return Err(OfficeError::InvalidZip {
            path: path.to_path_buf(),
        });
    }

    Ok(())
}

/// Quick check: does this path look like an Office file based on extension alone?
///
/// This is a lightweight check that does not read the file. Useful for UI
/// file-type filtering and initial triage.
pub fn is_office_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .as_deref(),
        Some("docx") | Some("pptx") | Some("xlsx")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_zip(path: &Path) {
        let mut buf = Vec::new();
        {
            let mut writer =
                zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("dummy.txt", options).unwrap();
            writer.write_all(b"dummy").unwrap();
            writer.finish().unwrap();
        }
        std::fs::write(path, &buf).unwrap();
    }

    #[test]
    fn test_detect_docx() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("document.docx");
        write_zip(&path);
        assert_eq!(detect_format(&path).unwrap(), OfficeFormat::Word);
    }

    #[test]
    fn test_detect_pptx() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("deck.pptx");
        write_zip(&path);
        assert_eq!(detect_format(&path).unwrap(), OfficeFormat::PowerPoint);
    }

    #[test]
    fn test_detect_xlsx() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sheet.xlsx");
        write_zip(&path);
        assert_eq!(detect_format(&path).unwrap(), OfficeFormat::Excel);
    }

    #[test]
    fn test_unknown_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("file.txt");
        std::fs::write(&path, b"not a zip").unwrap();
        assert!(detect_format(&path).is_err());
    }

    #[test]
    fn test_invalid_zip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("fake.docx");
        std::fs::write(&path, b"this is not a zip file at all").unwrap();
        assert!(matches!(
            detect_format(&path),
            Err(OfficeError::InvalidZip { .. })
        ));
    }

    #[test]
    fn test_corrupt_zip_signature() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("corrupt.docx");
        // Write something that starts with PK but is not a valid zip.
        std::fs::write(&path, b"PK\x03\x04garbage").unwrap();
        // Signature itself is valid, so detection passes.
        assert!(detect_format(&path).is_ok());
    }

    #[test]
    fn test_is_office_extension() {
        assert!(is_office_extension(Path::new("doc.docx")));
        assert!(is_office_extension(Path::new("doc.pptx")));
        assert!(is_office_extension(Path::new("doc.xlsx")));
        assert!(!is_office_extension(Path::new("doc.doc")));
        assert!(!is_office_extension(Path::new("doc.txt")));
        assert!(!is_office_extension(Path::new("doc.pdf")));
    }

    #[test]
    fn test_case_insensitive_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("UPPER.DOCX");
        write_zip(&path);
        assert_eq!(detect_format(&path).unwrap(), OfficeFormat::Word);
    }
}
