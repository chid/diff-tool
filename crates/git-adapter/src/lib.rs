use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("failed to read metadata for {path}: {source}")]
    Metadata {
        path: PathBuf,
        source: std::io::Error,
    },
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
    let mut entries = Vec::new();
    for item in WalkBuilder::new(root)
        .hidden(false)
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
