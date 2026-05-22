use git_adapter::{EntryStatus, scan_tree};
use std::fs;
use tempfile::tempdir;

#[test]
fn scan_tree_reports_sorted_nested_entries() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("README.md"), "# sample\n").unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(dir.path().join(".env"), "TOKEN=example\n").unwrap();

    let entries = scan_tree(dir.path()).unwrap();
    let paths = entries
        .iter()
        .map(|entry| entry.path.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    assert_eq!(paths, vec![".env", "README.md", "src", "src/main.rs"]);
    assert_eq!(entries[2].status, EntryStatus::Directory);
    assert_eq!(entries[3].status, EntryStatus::Unchanged);
    assert_eq!(entries[3].size, Some("fn main() {}\n".len() as u64));
}
