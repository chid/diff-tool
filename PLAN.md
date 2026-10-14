# Plan for Shippable Product

To reach a shippable state, the tool needs to move beyond single-file diffing to folder-level comparison.

## Goal
A functional desktop app that can:
1.  Select two folders to compare.
2.  Display a side-by-side file tree.
3.  Perform diffs on selected files from the tree.

## Implementation Roadmap

### 1. Folder Comparison Backend (`crates/git-adapter`)
- [x] Implement `compare_folders(left: &Path, right: &Path) -> Result<FolderDiff, AdapterError>` in `crates/git-adapter/src/lib.rs`.
    - Scans both trees, matches files by relative path, classifies as Added/Deleted/Modified/Unchanged with chunked binary equality checking.
- [x] Add unit & integration tests for folder comparison logic (e.g., identical folders, added/deleted files, gitignore filtering, error cases in `tests/compare_folders.rs`).

### 2. Tauri IPC Wiring
- [x] Expose `compare_folders` in `apps/desktop/src-tauri/src/main.rs`.
- [x] Expose `compareFolders` in `apps/desktop/src/tauri.ts` and `FolderDiff` in `apps/desktop/src/types.ts`.
- [x] Add Rust unit tests verifying Tauri IPC command behavior (`compare_folders_command_success`, `compare_folders_command_invalid_path_fails`).

### 3. Desktop UI Integration
- [x] Update `App.tsx` to handle folder selection for *both* left and right sides (via dual folder picker buttons, drag-and-drop, and CLI launch arguments).
- [x] Update `App.tsx` and `useTreeLoader` to call `compareFolders` and update the tree view with loading, error, and empty states.
- [x] Wire up file selection in the tree to trigger `compareFiles` for the selected pair into `DiffPane.tsx`.

### 4. CLI Support (`crates/diff-cli`)
- [x] Implement `dt dir <left> <right>` positional two-folder diffing.
- [x] Output status badges `[A]`, `[M]`, `[D]`, `[ ]` for human readability, and full JSON array when `--json` flag is provided.
- [x] Conforms to exit codes: `0` for identical, `1` for differences, `2` for errors.

### 5. Verification
- [x] Comprehensive test suites passing across all crates and frontend (`check.sh`, `npm test`, `perf:quick`, `cargo test -p git-adapter`, `cargo test -p diff-cli`, `vitest`).

---
This plan addresses the P2 TODOs while focusing on the core functional path of a diff tool.
