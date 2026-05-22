use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiffError {
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiffMode {
    Unified,
    Split,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum IgnoreMode {
    #[default]
    None,
    Whitespace,
    WhitespaceAndCase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffRequest {
    pub left: PathBuf,
    pub right: PathBuf,
    pub mode: DiffMode,
    pub ignore: IgnoreMode,
    pub context: usize,
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffResult {
    pub files: Vec<FileDiff>,
    pub stats: DiffStats,
    pub warnings: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDiff {
    pub left: PathBuf,
    pub right: PathBuf,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub unchanged: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hunk {
    pub old_range: LineRange,
    pub new_range: LineRange,
    pub lines: Vec<LineChange>,
    pub foldable_context: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineRange {
    pub start: usize,
    pub len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LineChange {
    pub kind: ChangeKind,
    pub old_no: Option<usize>,
    pub new_no: Option<usize>,
    pub text: String,
    pub intra_spans: Vec<IntraSpan>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntraSpan {
    pub start: usize,
    pub len: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergeRequest {
    pub base: PathBuf,
    pub ours: PathBuf,
    pub theirs: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergeResult {
    pub auto_resolved: bool,
    pub conflicts: usize,
    pub output_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImageDiffResult {
    pub dimensions: Option<(u32, u32)>,
    pub mismatch_count: Option<u64>,
    pub modes: Vec<String>,
}

pub fn compare_files(request: DiffRequest) -> Result<DiffResult, DiffError> {
    let left = read_limited(&request.left, request.max_bytes)?;
    let right = read_limited(&request.right, request.max_bytes)?;
    Ok(compare_texts(
        request.left,
        request.right,
        &left.contents,
        &right.contents,
        request.ignore,
        left.truncated || right.truncated,
    ))
}

pub fn compare_texts(
    left_path: PathBuf,
    right_path: PathBuf,
    left: &str,
    right: &str,
    ignore: IgnoreMode,
    truncated: bool,
) -> DiffResult {
    let normalized_left = normalize_for_compare(left, ignore);
    let normalized_right = normalize_for_compare(right, ignore);
    let diff = TextDiff::from_lines(&normalized_left, &normalized_right);
    let left_lines = split_lines_lossless(left);
    let right_lines = split_lines_lossless(right);

    let mut stats = DiffStats {
        additions: 0,
        deletions: 0,
        unchanged: 0,
    };
    let mut lines = Vec::new();
    let mut old_no = 1usize;
    let mut new_no = 1usize;
    let mut left_idx = 0usize;
    let mut right_idx = 0usize;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                stats.unchanged += 1;
                lines.push(LineChange {
                    kind: ChangeKind::Context,
                    old_no: Some(old_no),
                    new_no: Some(new_no),
                    text: left_lines.get(left_idx).cloned().unwrap_or_default(),
                    intra_spans: Vec::new(),
                });
                old_no += 1;
                new_no += 1;
                left_idx += 1;
                right_idx += 1;
            }
            ChangeTag::Delete => {
                stats.deletions += 1;
                lines.push(LineChange {
                    kind: ChangeKind::Deletion,
                    old_no: Some(old_no),
                    new_no: None,
                    text: left_lines.get(left_idx).cloned().unwrap_or_default(),
                    intra_spans: Vec::new(),
                });
                old_no += 1;
                left_idx += 1;
            }
            ChangeTag::Insert => {
                stats.additions += 1;
                lines.push(LineChange {
                    kind: ChangeKind::Addition,
                    old_no: None,
                    new_no: Some(new_no),
                    text: right_lines.get(right_idx).cloned().unwrap_or_default(),
                    intra_spans: Vec::new(),
                });
                new_no += 1;
                right_idx += 1;
            }
        }
    }

    DiffResult {
        files: vec![FileDiff {
            left: left_path,
            right: right_path,
            hunks: vec![Hunk {
                old_range: LineRange {
                    start: 1,
                    len: old_no.saturating_sub(1),
                },
                new_range: LineRange {
                    start: 1,
                    len: new_no.saturating_sub(1),
                },
                lines,
                foldable_context: false,
            }],
        }],
        stats,
        warnings: Vec::new(),
        truncated,
    }
}

fn normalize_for_compare(input: &str, ignore: IgnoreMode) -> String {
    match ignore {
        IgnoreMode::None => input.to_owned(),
        IgnoreMode::Whitespace => input
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n"),
        IgnoreMode::WhitespaceAndCase => input
            .lines()
            .map(|line| {
                line.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase()
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn split_lines_lossless(input: &str) -> Vec<String> {
    if input.is_empty() {
        return Vec::new();
    }
    input.lines().map(ToOwned::to_owned).collect()
}

struct LimitedRead {
    contents: String,
    truncated: bool,
}

fn read_limited(path: &Path, max_bytes: Option<u64>) -> Result<LimitedRead, DiffError> {
    let bytes = fs::read(path).map_err(|source| DiffError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let truncated = max_bytes.is_some_and(|max| bytes.len() as u64 > max);
    let slice = match max_bytes {
        Some(max) => &bytes[..bytes.len().min(max as usize)],
        None => bytes.as_slice(),
    };
    Ok(LimitedRead {
        contents: String::from_utf8_lossy(slice).into_owned(),
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_text_lines() {
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "one\ntwo\n",
            "one\nthree\n",
            IgnoreMode::None,
            false,
        );
        assert_eq!(result.stats.additions, 1);
        assert_eq!(result.stats.deletions, 1);
        assert_eq!(result.stats.unchanged, 1);
    }

    #[test]
    fn ignores_whitespace_when_requested() {
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "one   two\n",
            "one two\n",
            IgnoreMode::Whitespace,
            false,
        );
        assert_eq!(result.stats.additions, 0);
        assert_eq!(result.stats.deletions, 0);
    }
}
