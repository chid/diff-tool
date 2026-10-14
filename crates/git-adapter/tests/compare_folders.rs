use git_adapter::{compare_folders, AdapterError, EntryStatus};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn test_identical_directories() {
    let left = tempdir().unwrap();
    let right = tempdir().unwrap();

    fs::create_dir_all(left.path().join("sub")).unwrap();
    fs::create_dir_all(right.path().join("sub")).unwrap();

    fs::write(left.path().join("file1.txt"), "hello world\n").unwrap();
    fs::write(right.path().join("file1.txt"), "hello world\n").unwrap();

    fs::write(left.path().join("sub/file2.txt"), "nested content\n").unwrap();
    fs::write(right.path().join("sub/file2.txt"), "nested content\n").unwrap();

    let diff = compare_folders(left.path(), right.path()).unwrap();
    assert_eq!(diff.entries.len(), 3);

    for entry in &diff.entries {
        if entry.path == Path::new("sub") {
            assert_eq!(entry.status, EntryStatus::Directory);
        } else {
            assert_eq!(entry.status, EntryStatus::Unchanged);
        }
    }
}

#[test]
fn test_added_and_deleted_files() {
    let left = tempdir().unwrap();
    let right = tempdir().unwrap();

    fs::write(left.path().join("deleted.txt"), "only on left").unwrap();
    fs::write(right.path().join("added.txt"), "only on right").unwrap();
    fs::write(left.path().join("common.txt"), "common").unwrap();
    fs::write(right.path().join("common.txt"), "common").unwrap();

    let diff = compare_folders(left.path(), right.path()).unwrap();
    let find_entry = |name: &str| diff.entries.iter().find(|e| e.path == Path::new(name));

    assert_eq!(
        find_entry("deleted.txt").unwrap().status,
        EntryStatus::Deleted
    );
    assert_eq!(find_entry("added.txt").unwrap().status, EntryStatus::Added);
    assert_eq!(
        find_entry("common.txt").unwrap().status,
        EntryStatus::Unchanged
    );
}

#[test]
fn test_modified_file_contents_same_and_different_size() {
    let left = tempdir().unwrap();
    let right = tempdir().unwrap();

    // Different size
    fs::write(left.path().join("size_diff.txt"), "short").unwrap();
    fs::write(right.path().join("size_diff.txt"), "much longer text").unwrap();

    // Same size, different bytes
    fs::write(left.path().join("content_diff.txt"), "abcdef").unwrap();
    fs::write(right.path().join("content_diff.txt"), "123456").unwrap();

    let diff = compare_folders(left.path(), right.path()).unwrap();
    let find_entry = |name: &str| diff.entries.iter().find(|e| e.path == Path::new(name));

    assert_eq!(
        find_entry("size_diff.txt").unwrap().status,
        EntryStatus::Modified
    );
    assert_eq!(
        find_entry("content_diff.txt").unwrap().status,
        EntryStatus::Modified
    );
}

#[test]
fn test_gitignore_is_respected() {
    let left = tempdir().unwrap();
    let right = tempdir().unwrap();

    // Create a .gitignore in left and right
    fs::write(left.path().join(".gitignore"), "ignored.log\n*.tmp\n").unwrap();
    fs::write(right.path().join(".gitignore"), "ignored.log\n*.tmp\n").unwrap();

    // Write ignored files
    fs::write(left.path().join("ignored.log"), "log on left").unwrap();
    fs::write(right.path().join("ignored.log"), "log on right").unwrap();
    fs::write(left.path().join("temp.tmp"), "tmp on left").unwrap();

    // Write normal file
    fs::write(left.path().join("tracked.txt"), "tracked").unwrap();
    fs::write(right.path().join("tracked.txt"), "tracked").unwrap();

    let diff = compare_folders(left.path(), right.path()).unwrap();
    let paths: Vec<_> = diff
        .entries
        .iter()
        .map(|e| e.path.to_string_lossy().to_string())
        .collect();

    assert!(paths.contains(&".gitignore".to_string()));
    assert!(paths.contains(&"tracked.txt".to_string()));
    assert!(!paths.contains(&"ignored.log".to_string()));
    assert!(!paths.contains(&"temp.tmp".to_string()));
}

#[test]
fn test_error_handling_nonexistent_and_not_dir() {
    let temp = tempdir().unwrap();
    let file = temp.path().join("file.txt");
    fs::write(&file, "not a dir").unwrap();
    let nonexistent = temp.path().join("does_not_exist");

    assert!(matches!(
        compare_folders(&nonexistent, temp.path()),
        Err(AdapterError::NotFound(_))
    ));
    assert!(matches!(
        compare_folders(temp.path(), &nonexistent),
        Err(AdapterError::NotFound(_))
    ));
    assert!(matches!(
        compare_folders(&file, temp.path()),
        Err(AdapterError::NotADirectory(_))
    ));
    assert!(matches!(
        compare_folders(temp.path(), &file),
        Err(AdapterError::NotADirectory(_))
    ));
}
