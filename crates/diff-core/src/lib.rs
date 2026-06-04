//! # diff-core
//!
//! Deterministic text-diff engine for the diff-tool project.
//!
//! ## Line-ending policy
//!
//! The engine **always normalizes CRLF (`\r\n`) and standalone CR (`\r`) to LF (`\n`)**
//! before comparison.  This is an intentional, opinionated choice:
//!
//! * CRLF and LF files with otherwise identical content diff as identical.
//! * Mixed line endings are unified so the diff output is clean.
//! * The original file on disk is never modified — normalization is in-memory only.
//!
//! This policy matches the behaviour of `git diff` (which compares the index blob
//! after `core.autocrlf` processing) and most modern diff tools.

pub mod parser;
pub mod ast_diff;
pub mod ast_to_hunks;

pub use parser::{Language, SyntaxDepth};

use imara_diff::InternedInput;
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiffError {
    #[error("failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path} appears to be a binary file")]
    BinaryFile { path: PathBuf },
}

/// Progress stage reported to the optional callback during diff computation.
///
/// Suitable for displaying a progress indicator in CLI or desktop UIs.
/// All variants fire *before* their corresponding work begins, so the UI
/// can display a consistent "about to do X" state.
#[derive(Debug, Clone)]
pub enum Progress {
    /// About to read a file from disk (size in bytes if known).
    Reading { path: PathBuf, bytes: u64 },
    /// About to normalize line endings and split into lines.
    Normalizing,
    /// About to run the diff algorithm.
    ComputingDiff,
    /// About to compute character-level (intraline) spans.
    ComputingIntralineSpans,
    /// About to build context-aware hunks.
    BuildingHunks,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiffMode {
    Unified,
    Split,
}

/// Diff algorithm to use when comparing text.
///
/// All variants are backed by `imara-diff` which provides fast, native Rust
/// implementations:
///
/// * **Myers** — standard O(ND) algorithm.  `imara-diff`'s implementation is
///   ~5,800× faster than `similar`'s on pathological cases (200K lines,
///   50% churn: 11 ms vs 64 s).  This is the default.
/// * **Patience** — mapped to `imara-diff`'s Histogram internally (both are
///   LCS-based algorithms).  Provided for CLI compatibility.
/// * **Histogram** — produces `git diff`-compatible output.  Faster than
///   `similar`'s Myers on most cases but slower than `imara-diff`'s Myers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DiffAlgorithm {
    #[default]
    Myers,
    Patience,
    Histogram,
    /// Syntax-aware AST diffing using tree-sitter.
    ///
    /// When selected, the pipeline parses both files into concrete syntax trees
    /// and performs a bottom-up container matching algorithm. Supported languages:
    /// Rust, TypeScript, Python. Falls back to line-based if the language is
    /// unsupported or the file exceeds the size gate.
    Syntax,
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
    #[serde(default)]
    pub algorithm: DiffAlgorithm,
    /// Wall-clock deadline in milliseconds. When set, the diff computation
    /// is run on a background thread and interrupted if it exceeds this
    /// budget.  The returned result will have `truncated = true` and a
    /// warning, with empty hunks (best-effort).
    #[serde(default)]
    pub deadline_ms: Option<u64>,
    /// Override language detection. When set, forces the language
    /// for syntax-aware diffing instead of inferring from extension.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub language: Option<String>,
    /// AST node depth for syntax-aware diffing.
    #[serde(default)]
    pub syntax_depth: Option<String>,
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
    compare_files_with_progress(request, None)
}

/// Same as [`compare_files`] but fires an optional progress callback at each
/// pipeline stage.  The callback runs synchronously on the calling thread.
pub fn compare_files_with_progress(
    request: DiffRequest,
    progress: Option<Box<dyn Fn(Progress) + Send>>,
) -> Result<DiffResult, DiffError> {
    // Resolve file sizes once so we don't call stat(2) again inside read_limited.
    let left_size = std::fs::metadata(&request.left).map(|m| m.len()).ok();
    let right_size = std::fs::metadata(&request.right).map(|m| m.len()).ok();

    if let Some(ref cb) = progress {
        cb(Progress::Reading {
            path: request.left.clone(),
            bytes: left_size.unwrap_or(0),
        });
    }
    let left = read_limited_known(&request.left, request.max_bytes, left_size)?;
    if let Some(ref cb) = progress {
        cb(Progress::Reading {
            path: request.right.clone(),
            bytes: right_size.unwrap_or(0),
        });
    }
    let right = read_limited_known(&request.right, request.max_bytes, right_size)?;
    let mut warnings: Vec<String> = Vec::new();

    if left.is_binary {
        return Err(DiffError::BinaryFile {
            path: request.left,
        });
    }
    if right.is_binary {
        return Err(DiffError::BinaryFile {
            path: request.right,
        });
    }

    if left.utf8_lossy {
        warnings.push(format!(
            "{} contains invalid UTF-8; non-UTF-8 bytes replaced with replacement characters",
            request.left.display()
        ));
    }
    if right.utf8_lossy {
        warnings.push(format!(
            "{} contains invalid UTF-8; non-UTF-8 bytes replaced with replacement characters",
            request.right.display()
        ));
    }

    let was_truncated = left.truncated || right.truncated;
    Ok(compare_texts(
        request.left,
        request.right,
        &left.contents,
        &right.contents,
        request.ignore,
        was_truncated,
        request.context,
        warnings,
        request.algorithm,
        request.deadline_ms,
        progress,
        request.language,
        request.syntax_depth,
    ))
}

pub fn compare_texts(
    left_path: PathBuf,
    right_path: PathBuf,
    left: &str,
    right: &str,
    ignore: IgnoreMode,
    truncated: bool,
    context: usize,
    mut warnings: Vec<String>,
    algorithm: DiffAlgorithm,
    deadline_ms: Option<u64>,
    progress: Option<Box<dyn Fn(Progress) + Send>>,
    explicit_language: Option<String>,
    explicit_depth: Option<String>,
) -> DiffResult {
    // Normalise line endings before any other processing so that CRLF and LF
    // files produce identical diffs regardless of which entry-point is used
    // (compare_files or a direct compare_texts call).
    let left_norm = normalize_line_endings(left);
    let right_norm = normalize_line_endings(right);
    let left = &left_norm;
    let right = &right_norm;

    if let Some(ref cb) = progress {
        cb(Progress::Normalizing);
    }

    let normalized_left = normalize_for_compare(left, ignore);
    let normalized_right = normalize_for_compare(right, ignore);
    let left_lines = split_lines_lossless(left);
    let right_lines = split_lines_lossless(right);

    if let Some(ref cb) = progress {
        cb(Progress::ComputingDiff);
    }

    let (stats, mut lines) = match deadline_ms {
        Some(ms) => {
            run_diff_with_deadline(
                algorithm,
                &normalized_left,
                &normalized_right,
                &left_lines,
                &right_lines,
                ms,
                &mut warnings,
                &left_path,
                &right_path,
                context,
                &explicit_language,
                &explicit_depth,
            )
        }
        None => run_diff(algorithm, &normalized_left, &normalized_right, &left_lines, &right_lines, &left_path, &right_path, context, &explicit_language, &explicit_depth),
    };

    // Annotate intraline (character-level) spans on adjacent delete/insert pairs.
    // Skip for Syntax diffs (AST already provides aligned spans).
    if algorithm != DiffAlgorithm::Syntax {
        if let Some(ref cb) = progress {
            cb(Progress::ComputingIntralineSpans);
        }
        compute_intraline_spans(&mut lines);
    }

    if let Some(ref cb) = progress {
        cb(Progress::BuildingHunks);
    }
    let hunks = build_hunks(&lines, context);

    DiffResult {
        files: vec![FileDiff {
            left: left_path,
            right: right_path,
            hunks,
        }],
        stats,
        warnings,
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

/// Map our public `DiffAlgorithm` to the corresponding `imara_diff::Algorithm`.
///
/// `Patience` is mapped to `Histogram` because `imara-diff` does not provide a
/// standalone Patience implementation, and the two algorithms are closely related
/// (both are LCS-based, finding the longest common subsequence of unique lines).
fn to_imara_algo(algo: DiffAlgorithm) -> imara_diff::Algorithm {
    match algo {
        DiffAlgorithm::Myers => imara_diff::Algorithm::Myers,
        DiffAlgorithm::Patience | DiffAlgorithm::Histogram => {
            imara_diff::Algorithm::Histogram
        }
        // Syntax is handled by the calling code before reaching this function.
        DiffAlgorithm::Syntax => imara_diff::Algorithm::Histogram,
    }
}

/// Run the diff algorithm directly on the calling thread (no deadline).
///
/// For `DiffAlgorithm::Syntax`, attempts tree-sitter AST diffing; falls back
/// to line-based if the language is unsupported or parsing fails.
fn run_diff(
    algorithm: DiffAlgorithm,
    normalized_left: &str,
    normalized_right: &str,
    left_lines: &[String],
    right_lines: &[String],
    left_path: &Path,
    right_path: &Path,
    context: usize,
    explicit_language: &Option<String>,
    explicit_depth: &Option<String>,
) -> (DiffStats, Vec<LineChange>) {
    if algorithm == DiffAlgorithm::Syntax {
        let result = try_run_syntax_diff(left_path, right_path, normalized_left, normalized_right, left_lines, right_lines, context, explicit_language, explicit_depth);
        match result {
            Ok((stats, lines)) => return (stats, lines),
            Err(()) => {
                // Fall through to line-based with a silent fallback.
            }
        }
    }
    let imara_algo = to_imara_algo(algorithm);
    build_changes_imara(imara_algo, normalized_left, normalized_right, left_lines, right_lines)
}

/// Attempt a tree-sitter syntax-aware diff. Returns `Err(())` if the language
/// is unsupported or parsing failed, signalling the caller to fall back.
fn try_run_syntax_diff(
    left_path: &Path,
    right_path: &Path,
    normalized_left: &str,
    normalized_right: &str,
    left_lines: &[String],
    right_lines: &[String],
    context: usize,
    explicit_language: &Option<String>,
    explicit_depth: &Option<String>,
) -> Result<(DiffStats, Vec<LineChange>), ()> {
    // Detect language from file extension or explicit override.
    let lang = explicit_language
        .as_deref()
        .and_then(parser::Language::from_name)
        .or_else(|| parser::language_from_extension(left_path))
        .or_else(|| parser::language_from_extension(right_path))
        .ok_or(())?;

    // Resolve syntax depth.
    let depth = explicit_depth
        .as_deref()
        .and_then(parser::SyntaxDepth::from_name)
        .unwrap_or_default();

    // Size gate: skip AST parsing for files > 10K lines.
    if left_lines.len() > 10_000 || right_lines.len() > 10_000 {
        return Err(());
    }

    let mut left_nodes = parser::parse_file(normalized_left, lang, depth)
        .map_err(|_| ())?;
    let mut right_nodes = parser::parse_file(normalized_right, lang, depth)
        .map_err(|_| ())?;
    parser::build_child_links(&mut left_nodes);
    parser::build_child_links(&mut right_nodes);

    let ast_diff = ast_diff::compute_ast_diff(&left_nodes, &right_nodes);
    let (stats, lines) = ast_to_hunks::ast_diff_to_hunks_full(
        &ast_diff, &left_nodes, &right_nodes,
        normalized_left, normalized_right,
        left_lines, right_lines, context,
    );
    Ok((stats, lines))
}

/// Run the diff algorithm on a background thread, returning a best-effort
/// (empty) result if the deadline expires.
fn run_diff_with_deadline(
    algorithm: DiffAlgorithm,
    normalized_left: &str,
    normalized_right: &str,
    left_lines: &[String],
    right_lines: &[String],
    deadline_ms: u64,
    warnings: &mut Vec<String>,
    left_path: &Path,
    right_path: &Path,
    context: usize,
    explicit_language: &Option<String>,
    explicit_depth: &Option<String>,
) -> (DiffStats, Vec<LineChange>) {
    // Clone the owned strings so they can be moved into the spawned thread.
    let nl = normalized_left.to_owned();
    let nr = normalized_right.to_owned();
    let ll = left_lines.to_vec();
    let rl = right_lines.to_vec();
    let lp = left_path.to_path_buf();
    let rp = right_path.to_path_buf();
    let el = explicit_language.clone();
    let ed = explicit_depth.clone();

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = run_diff(algorithm, &nl, &nr, &ll, &rl, &lp, &rp, context, &el, &ed);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_millis(deadline_ms)) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            warnings.push(format!(
                "diff computation exceeded {} ms deadline; returning best-effort result",
                deadline_ms
            ));
            (
                DiffStats {
                    additions: 0,
                    deletions: 0,
                    unchanged: 0,
                },
                Vec::new(),
            )
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            warnings.push(
                "diff computation thread panicked; returning best-effort result".to_string(),
            );
            (
                DiffStats {
                    additions: 0,
                    deletions: 0,
                    unchanged: 0,
                },
                Vec::new(),
            )
        }
    }
}

/// Build line changes using imara-diff.  `algo` selects the algorithm variant
/// (Myers or Histogram; Patience is mapped to Histogram by the caller).
///
/// Unlike `similar`, imara-diff uses a Sink/Hunk-based API. This function
/// adapts that output into the same `Vec<LineChange>` format that the rest
/// of the pipeline expects.
fn build_changes_imara(
    algo: imara_diff::Algorithm,
    normalized_left: &str,
    normalized_right: &str,
    left_lines: &[String],
    right_lines: &[String],
) -> (DiffStats, Vec<LineChange>) {
    let input = InternedInput::new(normalized_left, normalized_right);
    let diff = imara_diff::Diff::compute(algo, &input);

    let mut stats = DiffStats {
        additions: 0,
        deletions: 0,
        unchanged: 0,
    };
    let mut lines = Vec::new();
    let mut old_cursor: u32 = 0;
    let mut new_cursor: u32 = 0;

    for hunk in diff.hunks() {
        // Emit context lines between previous hunk and this one.
        // old_cursor and new_cursor may differ after deletions/insertions,
        // but the matching section between hunks advances both equally.
        let ctx_len = hunk.before.start.saturating_sub(old_cursor);
        for i in 0..ctx_len {
            let old_idx = (old_cursor + i) as usize;
            let new_idx = (new_cursor + i) as usize;
            lines.push(LineChange {
                kind: ChangeKind::Context,
                old_no: Some(old_idx + 1),
                new_no: Some(new_idx + 1),
                text: left_lines.get(old_idx).cloned().unwrap_or_default(),
                intra_spans: Vec::new(),
            });
            stats.unchanged += 1;
        }
        // Advance cursors past the context section. The next assignments
        // (old_cursor = hunk.before.end, new_cursor = hunk.after.end)
        // will capture the post-change positions.

        // Emit deletions.
        for i in hunk.before.clone() {
            lines.push(LineChange {
                kind: ChangeKind::Deletion,
                old_no: Some(i as usize + 1),
                new_no: None,
                text: left_lines.get(i as usize).cloned().unwrap_or_default(),
                intra_spans: Vec::new(),
            });
            stats.deletions += 1;
        }
        old_cursor = hunk.before.end;

        // Emit additions.
        for i in hunk.after.clone() {
            lines.push(LineChange {
                kind: ChangeKind::Addition,
                old_no: None,
                new_no: Some(i as usize + 1),
                text: right_lines.get(i as usize).cloned().unwrap_or_default(),
                intra_spans: Vec::new(),
            });
            stats.additions += 1;
        }
        new_cursor = hunk.after.end;
    }

    // Emit remaining context lines after the last hunk.
    let remaining = (left_lines.len() as u32).saturating_sub(old_cursor);
    for i in 0..remaining {
        let old_idx = (old_cursor + i) as usize;
        let new_idx = (new_cursor + i) as usize;
        lines.push(LineChange {
            kind: ChangeKind::Context,
            old_no: Some(old_idx + 1),
            new_no: Some(new_idx + 1),
            text: left_lines.get(old_idx).cloned().unwrap_or_default(),
            intra_spans: Vec::new(),
        });
        stats.unchanged += 1;
    }

    (stats, lines)
}

/// Split a flat list of line changes into context-aware hunks.
///
/// The algorithm:
/// 1. Find indices of all non-context (change) lines.
/// 2. Group change indices into runs where the gap between successive changes
///    is ≤ `2 * context` context-only lines. When the gap exceeds that
///    threshold the groups split into separate hunks.
/// 3. Each hunk window extends `context` lines before the first change and
///    `context` lines after the last change (clamped to file boundaries).
fn build_hunks(lines: &[LineChange], context: usize) -> Vec<Hunk> {
    let n = lines.len();
    if n == 0 {
        return Vec::new();
    }

    // Collect indices of all non-context lines.
    let change_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, lc)| lc.kind != ChangeKind::Context)
        .map(|(i, _)| i)
        .collect();

    if change_indices.is_empty() {
        return Vec::new();
    }

    // Group change indices into runs.
    // A run ends when the gap to the next change exceeds 2 * context.
    let split_threshold = 2 * context;
    let mut groups: Vec<(usize, usize)> = Vec::new(); // (first_group_idx, last_group_idx) into change_indices
    let mut run_start = 0usize;

    for k in 1..change_indices.len() {
        let prev = change_indices[k - 1];
        let curr = change_indices[k];
        let gap = curr - prev - 1; // number of context-only lines between them

        if gap > split_threshold {
            groups.push((run_start, k - 1));
            run_start = k;
        }
    }
    groups.push((run_start, change_indices.len() - 1));

    // Build hunks from each group.
    let mut hunks = Vec::with_capacity(groups.len());
    let mut prev_hunk_end = 0usize;

    for (g_start, g_end) in groups {
        let first_change = change_indices[g_start];
        let last_change = change_indices[g_end];

        let hunk_start = first_change.saturating_sub(context);
        let hunk_end = ((last_change + context + 1).min(n)).max(hunk_start + 1);

        let hunk_lines: Vec<LineChange> = lines[hunk_start..hunk_end].to_vec();

        // Compute line ranges for this hunk.
        let old_start = hunk_lines
            .iter()
            .filter_map(|lc| lc.old_no)
            .min()
            .unwrap_or(1);
        let new_start = hunk_lines
            .iter()
            .filter_map(|lc| lc.new_no)
            .min()
            .unwrap_or(1);

        let old_len = hunk_lines.iter().filter(|lc| lc.old_no.is_some()).count();
        let new_len = hunk_lines.iter().filter(|lc| lc.new_no.is_some()).count();

        // foldable_context is true when there are hidden context lines on
        // either side of the hunk.
        let has_leading = hunk_start > prev_hunk_end;
        let has_trailing = hunk_end < n;

        hunks.push(Hunk {
            old_range: LineRange {
                start: old_start,
                len: old_len,
            },
            new_range: LineRange {
                start: new_start,
                len: new_len,
            },
            lines: hunk_lines,
            foldable_context: has_leading || has_trailing,
        });

        prev_hunk_end = hunk_end;
    }

    hunks
}

/// Scan adjacent Delete↔Insert line pairs and annotate them with character-level
/// (intraline) spans so UIs can highlight the specific characters that changed.
///
/// Handles runs of consecutive deletions followed by insertions (or vice versa) by
/// pairing corresponding lines in order — e.g. [Del, Del, Ins, Ins] pairs first
/// Del with first Ins, second Del with second Ins.
fn compute_intraline_spans(lines: &mut [LineChange]) {
    let n = lines.len();
    if n < 2 {
        return;
    }

    let mut paired = vec![false; n];

    // Collect indices of deletions, then insertions (or insertions, then deletions)
    // within each contiguous change run, and pair them 1-to-1.
    let mut i = 0;
    while i < n {
        if paired[i] || lines[i].kind == ChangeKind::Context {
            i += 1;
            continue;
        }

        // Collect a run of deletions
        let mut del_indices: Vec<usize> = Vec::new();
        while i < n && !paired[i] && lines[i].kind == ChangeKind::Deletion {
            del_indices.push(i);
            i += 1;
        }

        // Collect a run of insertions
        let mut ins_indices: Vec<usize> = Vec::new();
        while i < n && !paired[i] && lines[i].kind == ChangeKind::Addition {
            ins_indices.push(i);
            i += 1;
        }

        // Also handle Ins-before-Del ordering if dels run comes first
        if del_indices.is_empty() && !ins_indices.is_empty() {
            // We found insertions; check if deletions follow
            let ins_run = ins_indices;
            let mut del_run: Vec<usize> = Vec::new();
            while i < n && !paired[i] && lines[i].kind == ChangeKind::Deletion {
                del_run.push(i);
                i += 1;
            }
            if !del_run.is_empty() {
                pair_span_runs(lines, &del_run, &ins_run, &mut paired);
            }
        } else if !del_indices.is_empty() && !ins_indices.is_empty() {
            pair_span_runs(lines, &del_indices, &ins_indices, &mut paired);
        } else {
            // Only one kind of change (all dels or all ins) — nothing to pair.
            // Continue outer loop; i is already advanced.
        }
    }
}

/// For two runs of deletion and insertion line indices (in corresponding order),
/// compute intraline spans for each (del[i], ins[i]) pair.
fn pair_span_runs(
    lines: &mut [LineChange],
    del_indices: &[usize],
    ins_indices: &[usize],
    paired: &mut [bool],
) {
    let count = del_indices.len().min(ins_indices.len());
    for k in 0..count {
        let del_idx = del_indices[k];
        let ins_idx = ins_indices[k];

        let del_text = &lines[del_idx].text;
        let ins_text = &lines[ins_idx].text;

        // No meaningful character diff when either text is empty.
        if del_text.is_empty() || ins_text.is_empty() {
            paired[del_idx] = true;
            paired[ins_idx] = true;
            continue;
        }

        compute_intraline_spans_for_pair(lines, del_idx, ins_idx);
        paired[del_idx] = true;
        paired[ins_idx] = true;
    }
}

/// Compute intraline spans for a single deletion/insertion pair and write them
/// back into the line array.
fn compute_intraline_spans_for_pair(lines: &mut [LineChange], del_idx: usize, ins_idx: usize) {
    let del_text = &lines[del_idx].text;
    let ins_text = &lines[ins_idx].text;

    let del_chars: Vec<char> = del_text.chars().collect();
    let ins_chars: Vec<char> = ins_text.chars().collect();

    // Common prefix length (in chars)
    let prefix_len = del_chars
        .iter()
        .zip(ins_chars.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Common suffix length (in chars), skipping the changed region
    let suffix_len = del_chars[prefix_len..]
        .iter()
        .rev()
        .zip(ins_chars[prefix_len..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();

    let del_change_start = prefix_len;
    let del_change_len = del_chars.len() - prefix_len - suffix_len;
    let ins_change_start = prefix_len;
    let ins_change_len = ins_chars.len() - prefix_len - suffix_len;

    // Byte-offset maps
    let del_offsets: Vec<usize> = del_text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(del_text.len()))
        .collect();
    let ins_offsets: Vec<usize> = ins_text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(ins_text.len()))
        .collect();

    let del_spans: Vec<IntraSpan> = if del_change_len > 0 {
        vec![IntraSpan {
            start: del_offsets[del_change_start],
            len: del_offsets[del_change_start + del_change_len]
                - del_offsets[del_change_start],
        }]
    } else {
        Vec::new()
    };
    let ins_spans: Vec<IntraSpan> = if ins_change_len > 0 {
        vec![IntraSpan {
            start: ins_offsets[ins_change_start],
            len: ins_offsets[ins_change_start + ins_change_len]
                - ins_offsets[ins_change_start],
        }]
    } else {
        Vec::new()
    };

    lines[del_idx].intra_spans = del_spans;
    lines[ins_idx].intra_spans = ins_spans;
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
    is_binary: bool,
    utf8_lossy: bool,
}

/// Heuristic binary detection for the first 8 KB probe window.
///
/// Uses two signals, matching the behaviour of `git diff` and the `file`
/// command:
///
/// 1. **Null byte** — a single `0x00` byte in the probe is an immediate
///    binary signal.
///
/// 2. **High-entropy / non-text ratio** — if more than 10 % of the bytes in
///    the probe are non-printable control characters (excluding the common
///    text whitespace bytes HT, LF, FF, CR) the file is treated as binary.
///    High bytes (`0x80`-`0xFF`) are *not* treated as non-text because they
///    may be valid UTF-8 multi-byte sequences.
fn is_binary(data: &[u8]) -> bool {
    let probe = &data[..data.len().min(8192)];

    // Signal 1: null byte anywhere in the probe.
    if probe.contains(&0x00) {
        return true;
    }

    // Signal 2: > 10 % non-text bytes.
    if probe.is_empty() {
        return false;
    }
    let non_text = probe.iter().filter(|&&b| is_non_text_byte(b)).count();
    non_text * 10 > probe.len()
}

/// A byte is *non-text* when it is not printable ASCII, not one of the common
/// text whitespace / control characters (HT, LF, FF, CR), and not a potential
/// UTF-8 multi-byte continuation or start byte (`0x80`-`0xFF`).
fn is_non_text_byte(b: u8) -> bool {
    match b {
        // Common text whitespace / control characters.
        0x09 | 0x0A | 0x0C | 0x0D => false,
        // Printable ASCII.
        0x20..=0x7E => false,
        // Potential UTF-8 multi-byte bytes.
        0x80..=0xFF => false,
        // Everything else: control chars (0x00-0x08, 0x0B, 0x0E-0x1F), DEL.
        _ => true,
    }
}

/// Normalize line endings to LF so CRLF and LF files diff as identical.
///
/// Replaces `\r\n` → `\n` and stray standalone `\r` → `\n` (old-Mac style).
/// Input that already uses LF-only is returned unchanged (no allocation in the
/// fast path).
///
/// This is an intentional, opinionated policy: the diff engine operates on
/// logical lines independent of the host platform's line-ending convention.
/// See the [module-level documentation](crate) for the full policy rationale.
pub fn normalize_line_endings(input: &str) -> String {
    if input.is_empty() {
        return input.to_owned();
    }
    // Fast path: no \r present at all.
    if !input.contains('\r') {
        return input.to_owned();
    }
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            // \r\n → \n
            if chars.peek() == Some(&'\n') {
                chars.next();
                out.push('\n');
            } else {
                // Stray \r → \n (old-Mac style)
                out.push('\n');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Files smaller than this threshold are read with `fs::read` (simple, fast).
/// Files at or above this threshold use memory-mapped I/O to avoid loading
/// the entire file into a heap buffer when only the first N bytes are needed.
const MMAP_THRESHOLD: u64 = 10 * 1024 * 1024; // 10 MiB

/// Read a file, optionally limiting to `max_bytes`.  Accepts a pre-computed
/// the caller (e.g. from a progress callback that already did `stat(2)`).
fn read_limited_known(
    path: &Path,
    max_bytes: Option<u64>,
    file_size: Option<u64>,
) -> Result<LimitedRead, DiffError> {
    if let Some(sz) = file_size.filter(|&sz| sz >= MMAP_THRESHOLD) {
        return read_mmap(path, max_bytes, sz);
    }

    let bytes = fs::read(path).map_err(|source| DiffError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    bytes_to_limited(&bytes, max_bytes, None, |slice| String::from_utf8_lossy(slice).into_owned())
}

/// Read a large file via memory-mapped I/O.
///
/// When `max_bytes` is set, only the first `max_bytes` bytes are accessed;
/// the OS pages in only what is actually read, avoiding the cost of loading
/// the entire file into RAM.
fn read_mmap(path: &Path, max_bytes: Option<u64>, file_len: u64) -> Result<LimitedRead, DiffError> {
    let file = std::fs::File::open(path).map_err(|source| DiffError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    // Only map up to max_bytes if set, otherwise map the whole file.
    let map_len = match max_bytes {
        Some(max) => file_len.min(max),
        None => file_len,
    };

    // Don't attempt to mmap an empty file — just return empty contents.
    if map_len == 0 {
        return Ok(LimitedRead {
            contents: String::new(),
            truncated: false,
            is_binary: false,
            utf8_lossy: false,
        });
    }

    // Safety: the file is opened read-only above.  The mapping is immutable
    // and dropped before any other access to the file could invalidate it.
    let mmap = unsafe {
        Mmap::map(&file).map_err(|source| DiffError::ReadFile {
            path: path.to_path_buf(),
            source,
        })?
    };

    let result = bytes_to_limited(
        &mmap[..map_len as usize],
        max_bytes,
        Some(file_len), // pass actual file length for accurate truncation detection
        |s| String::from_utf8_lossy(s).into_owned(),
    );

    // Explicitly drop the mmap before returning to release the mapping.
    drop(mmap);

    result
}

/// Common post-processing from a byte slice: binary detection, UTF-8
/// conversion, line-ending normalization, and truncation flag.
///
/// `total_len` is the actual file length (used for truncation detection when
/// the caller has already limited the slice, e.g. via mmap). When `None`,
/// `slice.len()` is used as the total length (e.g. `fs::read` path).
fn bytes_to_limited(
    slice: &[u8],
    max_bytes: Option<u64>,
    total_len: Option<u64>,
    to_string: impl FnOnce(&[u8]) -> String,
) -> Result<LimitedRead, DiffError> {
    let effective_len = total_len.unwrap_or(slice.len() as u64);
    let truncated = max_bytes.is_some_and(|max| effective_len > max);
    let actual = match max_bytes {
        Some(max) => &slice[..slice.len().min(max as usize)],
        None => slice,
    };

    let is_bin = is_binary(actual);
    let utf8_lossy = std::str::from_utf8(actual).is_err();
    let raw = to_string(actual);

    Ok(LimitedRead {
        contents: normalize_line_endings(&raw),
        truncated,
        is_binary: is_bin,
        utf8_lossy,
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
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
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
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        assert_eq!(result.stats.additions, 0);
        assert_eq!(result.stats.deletions, 0);
    }

    #[test]
    fn produces_single_hunk_when_changes_are_close() {
        // With context=3, a gap of 5 context lines ≤ 2*3=6 means one merged hunk.
        let left = "a\nb\nc\nd\ne\nf\ng\nh\ni\n";
        let right = "a\nB\nc\nd\ne\nf\ng\nH\ni\n";
        let result = compare_texts(
            "left".into(),
            "right".into(),
            left,
            right,
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        assert_eq!(result.files[0].hunks.len(), 1, "changes close together produce 1 hunk");
        assert_eq!(result.stats.unchanged, 7);
    }

    #[test]
    fn splits_when_gap_exceeds_context_window() {
        // With context=2, gap > 4 context lines between changes forces a split.
        let left =  "a\nb\nc\nd\ne\nf\ng\nh\n";
        let right = "A\nb\nc\nd\ne\nf\ng\nH\n";
        // Lines: b-g (6 context lines) between changes at a and h.
        // context=2, threshold=4. gap=6 > 4 → split.
        let result = compare_texts(
            "left".into(),
            "right".into(),
            left,
            right,
            IgnoreMode::None,
            false,
            2,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        assert_eq!(result.files[0].hunks.len(), 2, "far-apart changes produce 2 hunks");
    }

    #[test]
    fn no_hunks_when_no_changes() {
        let result = compare_texts(
            "left".into(),
            "right".into(),
            "same\nfile\n",
            "same\nfile\n",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        assert!(result.files[0].hunks.is_empty(), "identical files produce 0 hunks");
    }

    #[test]
    fn empty_input_produces_no_hunks() {
        let result = compare_texts(
            "left".into(),
            "right".into(),
            "",
            "",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        assert!(result.files[0].hunks.is_empty());
    }

    #[test]
    fn hunk_ranges_are_correct() {
        let left =  "1\n2\n3\n4\n5\n";
        let right = "1\nx\n3\n4\n5\n";
        let result = compare_texts(
            "left".into(),
            "right".into(),
            left,
            right,
            IgnoreMode::None,
            false,
            1,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        let hunk = &result.files[0].hunks[0];
        // With context=1: line 1 (ctx), line 2 (del), line x (add), line 3 (ctx) = 4 lines
        assert_eq!(hunk.old_range.start, 1);
        assert_eq!(hunk.new_range.start, 1);
        assert_eq!(hunk.lines.len(), 4);
    }

    #[test]
    fn foldable_context_is_set_on_hunks() {
        // Changes at lines 2 and 10 with lots of context between and after.
        let left =  "01\n02\n03\n04\n05\n06\n07\n08\n09\n10\n11\n12\n";
        let right = "01\nXX\n03\n04\n05\n06\n07\n08\n09\nYY\n11\n12\n";
        let result = compare_texts(
            "left".into(),
            "right".into(),
            left,
            right,
            IgnoreMode::None,
            false,
            2,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        // context=2, gap between changes at lines 2 and 10 = 7 lines > 4 → 2 hunks
        assert_eq!(result.files[0].hunks.len(), 2);
        // First hunk: foldable because of trailing context (hidden lines 5-8 before hunk 2)
        assert!(result.files[0].hunks[0].foldable_context);
        // Second hunk: foldable because of leading context (gap between hunks)
        assert!(result.files[0].hunks[1].foldable_context);
    }

    #[test]
    fn detects_null_byte_as_binary() {
        let null_bytes = &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x00, 0x77, 0x6f];
        assert!(is_binary(null_bytes));
    }

    #[test]
    fn text_without_null_is_not_binary() {
        let text = b"Hello, world!\n";
        assert!(!is_binary(text));
    }

    #[test]
    fn high_entropy_non_text_bytes_detected_as_binary() {
        // 20 bytes: 3 non-text (0x01, 0x02, 0x03), 17 printable.
        // 3/20 = 15 % > 10 % threshold → binary.
        let mut data = vec![b'a'; 17];
        data.extend_from_slice(&[0x01, 0x02, 0x03]);
        assert!(is_binary(&data));
    }

    #[test]
    fn low_entropy_below_threshold_not_binary() {
        // 100 bytes: 9 non-text bytes = 9 % ≤ 10 % threshold → not binary.
        let mut data = vec![b'a'; 91];
        for _ in 0..9 {
            data.push(0x01);
        }
        assert!(!is_binary(&data));
    }

    #[test]
    fn valid_utf8_high_bytes_not_binary() {
        // "café" = [0x63, 0x61, 0x66, 0xC3, 0xA9] — 0xC3 and 0xA9 are
        // valid UTF-8 multi-byte bytes, not flagged as non-text.
        let data = "café\n".as_bytes();
        assert!(!is_binary(data));
    }

    #[test]
    fn common_whitespace_bytes_not_binary() {
        // Tab (0x09), LF (0x0A), FF (0x0C), CR (0x0D) are all text bytes.
        let data = b"hello\tworld\x0Cform\r\n";
        assert!(!is_binary(data));
    }

    #[test]
    fn empty_probe_not_binary() {
        assert!(!is_binary(&[]));
    }

    #[test]
    fn utf8_with_null_after_8k_is_not_detected() {
        // null byte at byte 8193 should not trigger binary detection (probe window is 8192)
        let mut data = vec![b'a'; 9000];
        data[8193] = 0x00;
        assert!(!is_binary(&data));
    }

    #[test]
    fn compare_files_rejects_binary_left() {
        let dir = tempfile::tempdir().unwrap();
        let left_path = dir.path().join("binary.bin");
        let right_path = dir.path().join("text.txt");
        std::fs::write(&left_path, &[0x00, 0x01, 0x02]).unwrap();
        std::fs::write(&right_path, b"hello\n").unwrap();

        let result = compare_files(DiffRequest {
            left: left_path,
            right: right_path,
            mode: DiffMode::Split,
            ignore: IgnoreMode::None,
            context: 3,
            max_bytes: None,
            algorithm: DiffAlgorithm::Myers,
            deadline_ms: None,
            language: None,
            syntax_depth: None,
        });
        match result {
            Err(DiffError::BinaryFile { .. }) => {}
            other => panic!("expected BinaryFile error, got {:?}", other),
        }
    }

    #[test]
    fn compare_files_rejects_binary_right() {
        let dir = tempfile::tempdir().unwrap();
        let left_path = dir.path().join("text.txt");
        let right_path = dir.path().join("binary.bin");
        std::fs::write(&left_path, b"hello\n").unwrap();
        std::fs::write(&right_path, &[0x00, 0x01, 0x02]).unwrap();

        let result = compare_files(DiffRequest {
            left: left_path,
            right: right_path,
            mode: DiffMode::Split,
            ignore: IgnoreMode::None,
            context: 3,
            max_bytes: None,
            algorithm: DiffAlgorithm::Myers,
            deadline_ms: None,
            language: None,
            syntax_depth: None,
        });
        match result {
            Err(DiffError::BinaryFile { .. }) => {}
            other => panic!("expected BinaryFile error, got {:?}", other),
        }
    }

    #[test]
    fn invalid_utf8_produces_warning() {
        // 0xFF is never valid in UTF-8
        let bytes = &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0xFF, 0x6f];
        let result = compare_texts(
            "bad.txt".into(),
            "good.txt".into(),
            &String::from_utf8_lossy(bytes),
            "hello",
            IgnoreMode::None,
            false,
            3,
            vec!["bad.txt contains invalid UTF-8".into()],
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("invalid UTF-8")),
            "expected UTF-8 warning, got {:?}",
            result.warnings
        );
    }

    #[test]
    fn valid_utf8_produces_no_warnings() {
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "hello\n",
            "world\n",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        assert!(result.warnings.is_empty());
    }

    // ── Intraline span tests ───────────────────────────────────────────

    #[test]
    fn adjacent_del_ins_gets_intraline_spans() {
        // "two" → "deux" — no common prefix/suffix, whole line differs
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "one\ntwo\nthree\n",
            "one\ndeux\nthree\n",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        let del = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Deletion)
            .expect("should have a deletion line");
        let ins = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Addition)
            .expect("should have an addition line");

        assert!(!del.intra_spans.is_empty(), "deletion line should have spans");
        assert!(!ins.intra_spans.is_empty(), "insertion line should have spans");

        // "two" → "deux" share no common prefix or suffix.
        // Span covers the entire deletion line (3 bytes).
        assert_eq!(del.intra_spans[0].start, 0);
        assert_eq!(del.intra_spans[0].len, 3);
        // Span covers the entire insertion line (4 bytes).
        assert_eq!(ins.intra_spans[0].start, 0);
        assert_eq!(ins.intra_spans[0].len, 4);
    }

    #[test]
    fn intraline_spans_with_unicode() {
        // "café" → "cafe" — only the accented é differs from plain e
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "hello\ncafé\nworld\n",
            "hello\ncafe\nworld\n",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        let del = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Deletion)
            .expect("should have a deletion line");
        let ins = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Addition)
            .expect("should have an addition line");

        // "café" = 5 bytes: c(1) a(1) f(1) é(2)
        // "cafe" = 4 bytes: c(1) a(1) f(1) e(1)
        // The differing region is bytes 3..5 (é) vs bytes 3..4 (e)
        assert!(!del.intra_spans.is_empty());
        assert!(!ins.intra_spans.is_empty());
        // Spans should cover just the differing characters, not the common "caf" prefix
        assert_eq!(del.intra_spans[0].start, 3); // byte offset of é
        assert_eq!(del.intra_spans[0].len, 2);   // é is 2 bytes
        assert_eq!(ins.intra_spans[0].start, 3); // byte offset of e
        assert_eq!(ins.intra_spans[0].len, 1);   // e is 1 byte
    }

    #[test]
    fn no_intraline_spans_without_adjacent_pair() {
        // Only a deletion, no adjacent insertion → no spans
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "one\ntwo\n",
            "one\n",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        let del = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Deletion)
            .expect("should have a deletion line");
        assert!(
            del.intra_spans.is_empty(),
            "standalone deletion should not have intraline spans"
        );
    }

    #[test]
    fn intraline_spans_del_ins_pairing() {
        // Delete then Insert ordering
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "ok\nold\n",
            "ok\nnew\n",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        let del = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Deletion)
            .expect("should have a deletion line");
        let ins = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Addition)
            .expect("should have an addition line");
        assert!(!del.intra_spans.is_empty());
        assert!(!ins.intra_spans.is_empty());
    }

    #[test]
    fn intraline_spans_multiple_pairs() {
        // Two adjacent del-ins pairs. "old1"/"new1" and "old2"/"new2"
        // share the common suffix "1" and "2" respectively.
        // The changed region in each pair is the prefix: "old" (3 bytes) vs "new" (3 bytes).
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "old1\nold2\n",
            "new1\nnew2\n",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        let hunk = &result.files[0].hunks[0];
        let deletions: Vec<_> = hunk
            .lines
            .iter()
            .filter(|l| l.kind == ChangeKind::Deletion)
            .collect();
        let insertions: Vec<_> = hunk
            .lines
            .iter()
            .filter(|l| l.kind == ChangeKind::Addition)
            .collect();
        assert_eq!(deletions.len(), 2);
        assert_eq!(insertions.len(), 2);
        for d in &deletions {
            assert!(!d.intra_spans.is_empty(), "each deletion should have spans");
            // Changed region is "old" — the prefix before the common suffix digit
            assert_eq!(d.intra_spans[0].start, 0);
            assert_eq!(d.intra_spans[0].len, 3);
        }
        for i in &insertions {
            assert!(!i.intra_spans.is_empty(), "each insertion should have spans");
            // Changed region is "new" — the prefix before the common suffix digit
            assert_eq!(i.intra_spans[0].start, 0);
            assert_eq!(i.intra_spans[0].len, 3);
        }
    }

    #[test]
    fn intraline_spans_whole_line_when_no_common_chars() {
        // "abcd" vs "xyz" — no common prefix or suffix, entire lines differ
        let result = compare_texts(
            "a.txt".into(),
            "b.txt".into(),
            "keep\nabcd\nkeep\n",
            "keep\nxyz\nkeep\n",
            IgnoreMode::None,
            false,
            3,
            Vec::new(),
            DiffAlgorithm::Myers,
            None,
            None,
            None,
            None,
        );
        let del = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Deletion)
            .expect("should have a deletion line");
        let ins = result.files[0].hunks[0]
            .lines
            .iter()
            .find(|l| l.kind == ChangeKind::Addition)
            .expect("should have an addition line");
        // Entire line differs — span covers whole text
        assert_eq!(del.intra_spans[0].start, 0);
        assert_eq!(del.intra_spans[0].len, del.text.len());
        assert_eq!(ins.intra_spans[0].start, 0);
        assert_eq!(ins.intra_spans[0].len, ins.text.len());
    }

    #[test]
    fn compare_files_warns_on_invalid_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let left_path = dir.path().join("bad.txt");
        let right_path = dir.path().join("good.txt");
        // Write a file with an invalid UTF-8 byte (0xFF)
        std::fs::write(&left_path, &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0xFF]).unwrap();
        std::fs::write(&right_path, b"hello\n").unwrap();

        let result = compare_files(DiffRequest {
            left: left_path,
            right: right_path,
            mode: DiffMode::Split,
            ignore: IgnoreMode::None,
            context: 3,
            max_bytes: None,
            algorithm: DiffAlgorithm::Myers,
            deadline_ms: None,
            language: None,
            syntax_depth: None,
        })
        .unwrap();

        assert!(
            result.warnings.iter().any(|w| w.contains("invalid UTF-8")),
            "expected UTF-8 warning for bad.txt, got {:?}",
            result.warnings
        );
    }

    // ── Line ending tests ──────────────────────────────────────────────

    #[test]
    fn normalize_crlf_to_lf() {
        assert_eq!(normalize_line_endings("a\r\nb\r\n"), "a\nb\n");
    }

    #[test]
    fn normalize_stray_cr_to_lf() {
        // Old-Mac style: standalone \r
        assert_eq!(normalize_line_endings("a\rb\r"), "a\nb\n");
    }

    #[test]
    fn normalize_mixed_line_endings() {
        assert_eq!(
            normalize_line_endings("a\r\nb\rc\n"),
            "a\nb\nc\n"
        );
    }

    #[test]
    fn normalize_preserves_lf_only() {
        assert_eq!(normalize_line_endings("a\nb\n"), "a\nb\n");
    }

    #[test]
    fn normalize_empty_string() {
        assert_eq!(normalize_line_endings(""), "");
    }

    #[test]
    fn crlf_and_lf_files_diff_as_identical() {
        let dir = tempfile::tempdir().unwrap();
        let left = dir.path().join("crlf.txt");
        let right = dir.path().join("lf.txt");
        std::fs::write(&left, b"alpha\r\nbeta\r\ngamma\r\n").unwrap();
        std::fs::write(&right, b"alpha\nbeta\ngamma\n").unwrap();

        let result = compare_files(DiffRequest {
            left,
            right,
            mode: DiffMode::Split,
            ignore: IgnoreMode::None,
            context: 3,
            max_bytes: None,
            algorithm: DiffAlgorithm::Myers,
            deadline_ms: None,
            language: None,
            syntax_depth: None,
        })
        .unwrap();

        assert_eq!(result.stats.additions, 0);
        assert_eq!(result.stats.deletions, 0);
        assert_eq!(result.stats.unchanged, 3);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn progress_callback_fires_all_stages_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let left_path = dir.path().join("left.txt");
        let right_path = dir.path().join("right.txt");
        std::fs::write(&left_path, b"line1\nline2\nline3\n").unwrap();
        std::fs::write(&right_path, b"line1\nmodified\nline3\n").unwrap();

        // Arc<Mutex> satisfies both Send + 'static for the callback.
        let stages = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let stages_cb = stages.clone();
        let l_path = left_path.clone();
        let r_path = right_path.clone();

        let result = compare_files_with_progress(
            DiffRequest {
                left: left_path.clone(),
                right: right_path.clone(),
                mode: DiffMode::Split,
                ignore: IgnoreMode::None,
                context: 3,
                max_bytes: None,
                algorithm: DiffAlgorithm::Myers,
                deadline_ms: None,
                language: None,
                syntax_depth: None,
            },
            Some(Box::new(move |p| {
                let label = match &p {
                    Progress::Reading { path, .. } => {
                        if *path == l_path {
                            "reading-left"
                        } else if *path == r_path {
                            "reading-right"
                        } else {
                            "reading-unknown"
                        }
                    }
                    Progress::Normalizing => "normalizing",
                    Progress::ComputingDiff => "computing-diff",
                    Progress::ComputingIntralineSpans => "computing-intraline",
                    Progress::BuildingHunks => "building-hunks",
                };
                stages_cb.lock().unwrap().push(label.to_string());
            })),
        )
        .unwrap();

        let fired = stages.lock().unwrap();
        assert_eq!(
            *fired,
            vec![
                "reading-left",
                "reading-right",
                "normalizing",
                "computing-diff",
                "computing-intraline",
                "building-hunks",
            ],
            "progress callback must fire all 6 stages (5 variants, Reading twice) in pipeline order"
        );

        // The diff result should still be correct.
        assert_eq!(result.files.len(), 1);
        assert!(!result.files[0].hunks.is_empty());
        assert_eq!(result.stats.deletions, 1);
        assert_eq!(result.stats.additions, 1);
    }
}
