use diff_core::{DiffAlgorithm, DiffMode, DiffRequest, IgnoreMode};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Emitter;

const DEFAULT_MAX_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB

#[derive(Debug, Deserialize)]
struct CompareFilesRequest {
    left: PathBuf,
    right: PathBuf,
    #[serde(default)]
    algorithm: Option<String>,
    #[serde(default)]
    deadline_ms: Option<u64>,
    #[serde(default)]
    progress: bool,
}

/// Payload emitted to the frontend via `diff-progress` events.
#[derive(Debug, Clone, Serialize)]
struct DiffProgressPayload {
    stage: &'static str,
    /// File path for the `reading` stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    /// File size in bytes for the `reading` stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    bytes: Option<u64>,
}

/// Parse an algorithm string, defaulting to Myers for unknown values.
fn parse_algorithm(s: &Option<String>) -> DiffAlgorithm {
    match s.as_deref() {
        Some("histogram") => DiffAlgorithm::Histogram,
        Some("patience") => DiffAlgorithm::Patience,
        Some("myers") | None => DiffAlgorithm::Myers,
        _ => DiffAlgorithm::Myers,
    }
}

#[tauri::command]
fn compare_files(
    app_handle: tauri::AppHandle,
    request: CompareFilesRequest,
) -> Result<diff_core::DiffResult, String> {
    let progress_cb: Option<Box<dyn Fn(diff_core::Progress) + Send>> =
        if request.progress {
            let app = app_handle.clone();
            Some(Box::new(move |p| {
                let payload = match p {
                    diff_core::Progress::Reading { path, bytes } => DiffProgressPayload {
                        stage: "reading",
                        path: Some(path.display().to_string()),
                        bytes: Some(bytes),
                    },
                    diff_core::Progress::Normalizing => DiffProgressPayload {
                        stage: "normalizing",
                        path: None,
                        bytes: None,
                    },
                    diff_core::Progress::ComputingDiff => DiffProgressPayload {
                        stage: "computing_diff",
                        path: None,
                        bytes: None,
                    },
                    diff_core::Progress::ComputingIntralineSpans => DiffProgressPayload {
                        stage: "computing_intraline_spans",
                        path: None,
                        bytes: None,
                    },
                    diff_core::Progress::BuildingHunks => DiffProgressPayload {
                        stage: "building_hunks",
                        path: None,
                        bytes: None,
                    },
                };
                let _ = app.emit("diff-progress", payload);
            }))
        } else {
            None
        };

    diff_core::compare_files_with_progress(
        DiffRequest {
            left: request.left,
            right: request.right,
            mode: DiffMode::Split,
            ignore: IgnoreMode::None,
            context: 3,
            max_bytes: Some(DEFAULT_MAX_BYTES),
            algorithm: parse_algorithm(&request.algorithm),
            deadline_ms: request.deadline_ms.filter(|&ms| ms > 0),
        },
        progress_cb,
    )
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
    let bytes =
        std::fs::read(&path).map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
    let slice = &bytes[..bytes.len().min(limit)];
    Ok(String::from_utf8_lossy(slice).into_owned())
}

/// Returns true if the path exists and is a directory.
#[tauri::command]
fn path_is_dir(path: PathBuf) -> bool {
    path.is_dir()
}

/// Returns the command-line arguments this process was launched with,
/// excluding the binary name (argv[0]). The frontend uses this to
/// auto-open files or folders passed via CLI.
#[tauri::command]
fn get_launch_args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            compare_files,
            scan_tree,
            read_file_text,
            path_is_dir,
            get_launch_args
        ])
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

    #[test]
    fn path_is_dir_returns_true_for_directory() {
        let dir = std::env::temp_dir().join("diff-tool-test");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(path_is_dir(dir));
    }

    #[test]
    fn path_is_dir_returns_false_for_file() {
        let path = temp_file("regular.txt", b"content");
        assert!(!path_is_dir(path));
    }

    #[test]
    fn path_is_dir_returns_false_for_nonexistent() {
        assert!(!path_is_dir(PathBuf::from("/nonexistent/path.xyz")));
    }

    #[test]
    fn get_launch_args_returns_vec() {
        // In test context args() includes the test binary name followed by
        // test harness flags. We only check that it returns a Vec (may be empty).
        let args = get_launch_args();
        // The result is a Vec<String> — no further assertions needed.
        let _: Vec<String> = args;
    }
}
