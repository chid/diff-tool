use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("failed to access {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read metadata for {path}: {source}")]
    Metadata {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("path does not exist: {0}")]
    NotFound(PathBuf),
    #[error("path is not a directory: {0}")]
    NotADirectory(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FolderDiff {
    pub entries: Vec<DirEntryDiff>,
}

pub fn compare_folders(left_root: &Path, right_root: &Path) -> Result<FolderDiff, AdapterError> {
    if !left_root.exists() {
        return Err(AdapterError::NotFound(left_root.to_path_buf()));
    }
    if !left_root.is_dir() {
        return Err(AdapterError::NotADirectory(left_root.to_path_buf()));
    }
    if !right_root.exists() {
        return Err(AdapterError::NotFound(right_root.to_path_buf()));
    }
    if !right_root.is_dir() {
        return Err(AdapterError::NotADirectory(right_root.to_path_buf()));
    }

    let left_entries = scan_tree(left_root)?;
    let right_entries = scan_tree(right_root)?;

    let mut left_map: HashMap<PathBuf, DirEntryDiff> = left_entries
        .into_iter()
        .map(|e| (e.path.clone(), e))
        .collect();
    let mut right_map: HashMap<PathBuf, DirEntryDiff> = right_entries
        .into_iter()
        .map(|e| (e.path.clone(), e))
        .collect();

    let mut all_paths: HashSet<PathBuf> = left_map.keys().cloned().collect();
    all_paths.extend(right_map.keys().cloned());

    let mut diff_entries = Vec::new();
    let mut sorted_paths: Vec<PathBuf> = all_paths.into_iter().collect();
    sorted_paths.sort();

    for path in sorted_paths {
        let left = left_map.remove(&path);
        let right = right_map.remove(&path);

        let entry = match (left, right) {
            (Some(l), Some(r)) => {
                let status = if l.status == EntryStatus::Directory
                    && r.status == EntryStatus::Directory
                {
                    EntryStatus::Directory
                } else if l.status == EntryStatus::Directory || r.status == EntryStatus::Directory {
                    EntryStatus::Modified
                } else if l.size != r.size {
                    EntryStatus::Modified
                } else {
                    let left_full = left_root.join(&path);
                    let right_full = right_root.join(&path);
                    match files_content_equal(&left_full, &right_full) {
                        Ok(true) => EntryStatus::Unchanged,
                        Ok(false) => EntryStatus::Modified,
                        Err(e) => {
                            return Err(AdapterError::Io {
                                path: left_full,
                                source: e,
                            });
                        }
                    }
                };
                DirEntryDiff {
                    path: path.clone(),
                    status,
                    size: r.size.or(l.size),
                    mtime: r.mtime.or(l.mtime),
                    hash: None,
                    children: Vec::new(),
                }
            }
            (Some(l), None) => DirEntryDiff {
                path: path.clone(),
                status: EntryStatus::Deleted,
                ..l
            },
            (None, Some(r)) => DirEntryDiff {
                path: path.clone(),
                status: EntryStatus::Added,
                ..r
            },
            (None, None) => unreachable!(),
        };
        diff_entries.push(entry);
    }

    Ok(FolderDiff {
        entries: diff_entries,
    })
}

fn files_content_equal(left_path: &Path, right_path: &Path) -> std::io::Result<bool> {
    let mut f1 = File::open(left_path)?;
    let mut f2 = File::open(right_path)?;

    let mut buf1 = [0u8; 64 * 1024];
    let mut buf2 = [0u8; 64 * 1024];

    loop {
        let n1 = f1.read(&mut buf1)?;
        let n2 = f2.read(&mut buf2)?;
        if n1 != n2 {
            return Ok(false);
        }
        if n1 == 0 {
            return Ok(true);
        }
        if buf1[..n1] != buf2[..n2] {
            return Ok(false);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirCompareRequest {
    pub left_root: PathBuf,
    pub right_root: PathBuf,
    pub filters: Vec<String>,
    pub compare_by: CompareBy,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CompareBy {
    SizeAndMtime,
    Contents,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirEntryDiff {
    pub path: PathBuf,
    pub status: EntryStatus,
    pub size: Option<u64>,
    pub mtime: Option<u64>,
    pub hash: Option<String>,
    pub children: Vec<DirEntryDiff>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EntryStatus {
    Added,
    Deleted,
    Modified,
    Unchanged,
    Directory,
}

pub fn scan_tree(root: &Path) -> Result<Vec<DirEntryDiff>, AdapterError> {
    if !root.exists() {
        return Err(AdapterError::NotFound(root.to_path_buf()));
    }
    if !root.is_dir() {
        return Err(AdapterError::NotADirectory(root.to_path_buf()));
    }

    let mut entries = Vec::new();
    for item in WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .git_global(false)
        .require_git(false)
        .build()
        .filter_map(Result::ok)
    {
        let path = item.path();
        if path == root {
            continue;
        }
        let metadata = path.metadata().map_err(|source| AdapterError::Metadata {
            path: path.to_path_buf(),
            source,
        })?;
        let rel = path.strip_prefix(root).unwrap_or(path).to_path_buf();
        entries.push(DirEntryDiff {
            path: rel,
            status: if metadata.is_dir() {
                EntryStatus::Directory
            } else {
                EntryStatus::Unchanged
            },
            size: metadata.is_file().then_some(metadata.len()),
            mtime: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs()),
            hash: None,
            children: Vec::new(),
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_compare_folders() {
        let left_dir = tempdir().unwrap();
        let right_dir = tempdir().unwrap();

        // Unchanged (same size, same content)
        fs::write(left_dir.path().join("unchanged.txt"), "same content").unwrap();
        fs::write(right_dir.path().join("unchanged.txt"), "same content").unwrap();

        // Modified by size
        fs::write(left_dir.path().join("modified_size.txt"), "old").unwrap();
        fs::write(
            right_dir.path().join("modified_size.txt"),
            "this-is-a-longer-content",
        )
        .unwrap();

        // Modified with same size but different content
        fs::write(left_dir.path().join("modified_same_len.txt"), "content_A").unwrap();
        fs::write(right_dir.path().join("modified_same_len.txt"), "content_B").unwrap();

        // Deleted
        fs::write(left_dir.path().join("deleted.txt"), "gone").unwrap();

        // Added
        fs::write(right_dir.path().join("added.txt"), "new").unwrap();

        let diff = compare_folders(left_dir.path(), right_dir.path()).unwrap();
        let entries = diff.entries;

        let find_entry = |path: &str| entries.iter().find(|e| e.path == Path::new(path));

        assert_eq!(
            find_entry("unchanged.txt").unwrap().status,
            EntryStatus::Unchanged
        );
        assert_eq!(
            find_entry("modified_size.txt").unwrap().status,
            EntryStatus::Modified
        );
        assert_eq!(
            find_entry("modified_same_len.txt").unwrap().status,
            EntryStatus::Modified
        );
        assert_eq!(
            find_entry("deleted.txt").unwrap().status,
            EntryStatus::Deleted
        );
        assert_eq!(find_entry("added.txt").unwrap().status, EntryStatus::Added);
    }

    #[test]
    fn test_compare_folders_not_found_or_not_dir() {
        let temp = tempdir().unwrap();
        let file_path = temp.path().join("a_file.txt");
        fs::write(&file_path, "hello").unwrap();

        let non_existent = temp.path().join("non_existent_folder");

        // Non existent left
        assert!(matches!(
            compare_folders(&non_existent, temp.path()),
            Err(AdapterError::NotFound(_))
        ));

        // Non existent right
        assert!(matches!(
            compare_folders(temp.path(), &non_existent),
            Err(AdapterError::NotFound(_))
        ));

        // Not a directory left
        assert!(matches!(
            compare_folders(&file_path, temp.path()),
            Err(AdapterError::NotADirectory(_))
        ));

        // Not a directory right
        assert!(matches!(
            compare_folders(temp.path(), &file_path),
            Err(AdapterError::NotADirectory(_))
        ));
    }

    #[test]
    fn request_model_serializes() {
        let request = DirCompareRequest {
            left_root: "left".into(),
            right_root: "right".into(),
            filters: vec!["*.rs".to_owned()],
            compare_by: CompareBy::SizeAndMtime,
        };
        assert_eq!(request.filters, vec!["*.rs"]);
    }
}
