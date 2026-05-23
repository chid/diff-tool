use diff_core::{DiffMode, DiffRequest, IgnoreMode};
use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

#[derive(Debug, Deserialize)]
struct CompareFilesRequest {
    left: PathBuf,
    right: PathBuf,
}

#[tauri::command]
fn compare_files(request: CompareFilesRequest) -> Result<diff_core::DiffResult, String> {
    diff_core::compare_files(DiffRequest {
        left: request.left,
        right: request.right,
        mode: DiffMode::Split,
        ignore: IgnoreMode::None,
        context: 3,
        max_bytes: Some(10 * 1024 * 1024),
    })
    .map_err(|error| error.to_string())
}

#[tauri::command]
fn scan_tree(root: PathBuf) -> Result<Vec<git_adapter::DirEntryDiff>, String> {
    git_adapter::scan_tree(&root).map_err(|error| error.to_string())
}

/// Reads the raw text contents of a file, truncating at `max_bytes` (defaults to 10 MiB).
/// Invalid UTF-8 is replaced with the Unicode replacement character.
#[tauri::command]
fn read_file_text(path: PathBuf, max_bytes: Option<u64>) -> Result<String, String> {
    let limit = max_bytes.unwrap_or(DEFAULT_MAX_BYTES) as usize;
    let bytes = std::fs::read(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let slice = &bytes[..bytes.len().min(limit)];
    Ok(String::from_utf8_lossy(slice).into_owned())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![compare_files, scan_tree, read_file_text])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join("diff-tool-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(contents).unwrap();
        path
    }

    #[test]
    fn read_valid_utf8_file() {
        let path = temp_file("hello.txt", b"hello world\n");
        let result = read_file_text(path, None).unwrap();
        assert_eq!(result, "hello world\n");
    }

    #[test]
    fn read_missing_file_returns_error() {
        let result = read_file_text(PathBuf::from("/nonexistent/path.xyz"), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed to read"));
    }

    #[test]
    fn truncation_at_max_bytes() {
        let path = temp_file("big.txt", b"0123456789");
        let result = read_file_text(path, Some(5)).unwrap();
        assert_eq!(result, "01234");
    }

    #[test]
    fn max_bytes_larger_than_file_reads_entire_file() {
        let path = temp_file("small.txt", b"abc");
        let result = read_file_text(path, Some(100)).unwrap();
        assert_eq!(result, "abc");
    }

    #[test]
    fn binary_file_is_lossy_decoded() {
        let path = temp_file("binary.bin", b"\xff\xfe\x00\x01");
        let result = read_file_text(path, None).unwrap();
        // from_utf8_lossy replaces invalid bytes with U+FFFD
        assert!(result.contains('\u{FFFD}'));
    }
}
