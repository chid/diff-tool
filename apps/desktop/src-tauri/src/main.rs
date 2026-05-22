use diff_core::{DiffMode, DiffRequest, IgnoreMode};
use serde::Deserialize;
use std::path::PathBuf;

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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![compare_files, scan_tree])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
