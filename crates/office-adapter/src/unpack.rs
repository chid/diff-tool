//! Safe zip archive reading for Office files.
//!
//! Provides bounded zip reading with configurable size and part-count limits
//! to prevent zip-bomb and decompression-bomb attacks. Office files are
//! expected to be well-formed zip archives conforming to the Open Packaging
//! Convention.

use crate::models::OfficeDiffOptions;
use crate::OfficeError;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// A safely-read entry from an Office zip archive.
#[derive(Debug, Clone)]
pub struct OfficeEntry {
    /// The path of the entry within the zip (e.g., `word/document.xml`).
    pub name: String,
    /// The raw bytes of the entry.
    pub data: Vec<u8>,
    /// The uncompressed size of the entry.
    pub size: u64,
}

/// Read and validate a zip archive with safety limits.
///
/// Returns a map of entry name → `OfficeEntry` for all entries that pass
/// the safety checks. Entries that exceed `options.max_part_size` are
/// skipped and recorded as unsupported features (via the returned warnings).
///
/// # Errors
///
/// Returns an error if:
/// - The zip archive cannot be opened or is corrupt.
/// - The total number of entries exceeds `options.max_part_count`.
/// - The total uncompressed size exceeds `MAX_TOTAL_UNCOMPRESSED` (200 MB).
pub fn read_zip_safe(
    path: &Path,
    data: &[u8],
    options: &OfficeDiffOptions,
) -> Result<SafeZip, OfficeError> {
    const MAX_TOTAL_UNCOMPRESSED: u64 = 200 * 1024 * 1024; // 200 MB

    let cursor = std::io::Cursor::new(data);
    let mut archive =
        zip::ZipArchive::new(cursor).map_err(|_| OfficeError::InvalidZip {
            path: path.to_path_buf(),
        })?;

    let total_entries = archive.len();
    if total_entries > options.max_part_count {
        return Err(OfficeError::TooManyParts {
            path: path.to_path_buf(),
            count: total_entries,
            limit: options.max_part_count,
        });
    }

    let mut entries: HashMap<String, OfficeEntry> = HashMap::with_capacity(total_entries);
    let mut oversized: Vec<String> = Vec::new();
    let mut total_uncompressed: u64 = 0;

    for i in 0..total_entries {
        let mut entry = archive.by_index(i).map_err(|_| OfficeError::InvalidZip {
            path: path.to_path_buf(),
        })?;

        let name = entry.name().to_owned();
        let size = entry.size();

        // Skip directories.
        if entry.is_dir() {
            continue;
        }

        // Check per-entry size.
        if size > options.max_part_size {
            oversized.push(name.clone());
            continue;
        }

        // Check cumulative uncompressed size.
        total_uncompressed += size;
        if total_uncompressed > MAX_TOTAL_UNCOMPRESSED {
            return Err(OfficeError::FileTooLarge {
                path: path.to_path_buf(),
                size: total_uncompressed,
                limit: MAX_TOTAL_UNCOMPRESSED,
            });
        }

        // Read the entry into memory.
        let mut buf = Vec::with_capacity(size as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|_| OfficeError::InvalidZip {
                path: path.to_path_buf(),
            })?;

        entries.insert(
            name.clone(),
            OfficeEntry {
                name,
                data: buf,
                size,
            },
        );
    }

    Ok(SafeZip {
        path: path.to_path_buf(),
        entries,
        oversized,
    })
}

/// A safely-read zip archive.
#[derive(Debug, Clone)]
pub struct SafeZip {
    /// The original file path.
    pub path: PathBuf,
    /// Successfully read entries, keyed by name.
    pub entries: HashMap<String, OfficeEntry>,
    /// Entries that were skipped because they exceeded `max_part_size`.
    pub oversized: Vec<String>,
}

impl SafeZip {
    /// Get an entry by its full path within the zip.
    pub fn get(&self, name: &str) -> Option<&OfficeEntry> {
        self.entries.get(name)
    }

    /// Check whether a named entry exists in the archive.
    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// List all entry names in the archive.
    pub fn entry_names(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer =
                zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for (name, data) in entries {
                writer.start_file(*name, options).unwrap();
                writer.write_all(data).unwrap();
            }
            writer.finish().unwrap();
        }
        buf
    }

    #[test]
    fn test_read_valid_zip() {
        let data = make_zip(&[
            ("[Content_Types].xml", b"<Types/>"),
            ("word/document.xml", b"<document/>"),
        ]);
        let options = OfficeDiffOptions::default();
        let result = read_zip_safe(Path::new("test.docx"), &data, &options).unwrap();

        assert!(result.contains("[Content_Types].xml"));
        assert!(result.contains("word/document.xml"));
        assert_eq!(result.oversized.len(), 0);
    }

    #[test]
    fn test_rejects_corrupt_zip() {
        let options = OfficeDiffOptions::default();
        let result = read_zip_safe(Path::new("bad.docx"), b"not a zip", &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_enforces_max_part_count() {
        // Create a zip with many tiny files.
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..20 {
            entries.push((format!("file{}.xml", i), b"<xml/>".to_vec()));
        }
        let refs: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        let data = make_zip(&refs);

        let mut options = OfficeDiffOptions::default();
        options.max_part_count = 10; // lower than our 20 entries
        let result = read_zip_safe(Path::new("test.docx"), &data, &options);
        assert!(matches!(result, Err(OfficeError::TooManyParts { .. })));
    }

    #[test]
    fn test_skips_oversized_parts() {
        let big = vec![b'x'; 2000];
        let data = make_zip(&[
            ("[Content_Types].xml", b"<Types/>"),
            ("word/document.xml", &big),
        ]);

        let mut options = OfficeDiffOptions::default();
        options.max_part_size = 1000; // "document.xml" exceeds this
        let result =
            read_zip_safe(Path::new("test.docx"), &data, &options).unwrap();

        assert!(result.contains("[Content_Types].xml"));
        assert!(!result.contains("word/document.xml"));
        assert_eq!(result.oversized, vec!["word/document.xml"]);
    }

    #[test]
    fn test_enforces_total_uncompressed_limit() {
        // Each entry is ~1 MB; 250 entries ≈ 250 MB > 200 MB limit.
        let chunk = vec![b'x'; 1_000_000];
        let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
        for i in 0..250 {
            entries.push((format!("file{}.xml", i), chunk.clone()));
        }
        let refs: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(n, d)| (n.as_str(), d.as_slice()))
            .collect();
        let data = make_zip(&refs);

        let options = OfficeDiffOptions::default();
        let result = read_zip_safe(Path::new("test.docx"), &data, &options);
        assert!(matches!(
            result,
            Err(OfficeError::FileTooLarge { .. })
        ));
    }

    #[test]
    fn test_entry_names() {
        let data = make_zip(&[
            ("[Content_Types].xml", b"<Types/>"),
            ("word/document.xml", b"<document/>"),
            ("word/styles.xml", b"<styles/>"),
        ]);
        let options = OfficeDiffOptions::default();
        let result = read_zip_safe(Path::new("test.docx"), &data, &options).unwrap();

        let mut names = result.entry_names();
        names.sort();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"[Content_Types].xml"));
        assert!(names.contains(&"word/document.xml"));
        assert!(names.contains(&"word/styles.xml"));
    }
}
