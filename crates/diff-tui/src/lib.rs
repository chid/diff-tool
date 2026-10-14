use diff_core::{ChangeKind, DiffResult, FileDiff, LineChange};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuiRenderOptions {
    pub width: usize,
    pub max_hunks: usize,
    pub max_lines_per_hunk: usize,
}

impl Default for TuiRenderOptions {
    fn default() -> Self {
        Self {
            width: 120,
            max_hunks: 8,
            max_lines_per_hunk: 80,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuiFeature {
    pub name: &'static str,
    pub status: TuiFeatureStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiFeatureStatus {
    Scaffolded,
    Planned,
}

pub const FEATURE_BASELINE: &[TuiFeature] = &[
    TuiFeature {
        name: "side-by-side panes",
        status: TuiFeatureStatus::Scaffolded,
    },
    TuiFeature {
        name: "line numbers",
        status: TuiFeatureStatus::Scaffolded,
    },
    TuiFeature {
        name: "hunk navigation",
        status: TuiFeatureStatus::Planned,
    },
    TuiFeature {
        name: "search",
        status: TuiFeatureStatus::Planned,
    },
    TuiFeature {
        name: "syntax highlighting",
        status: TuiFeatureStatus::Planned,
    },
    TuiFeature {
        name: "word-level highlights",
        status: TuiFeatureStatus::Planned,
    },
    TuiFeature {
        name: "foldable context",
        status: TuiFeatureStatus::Planned,
    },
    TuiFeature {
        name: "copy/apply change actions",
        status: TuiFeatureStatus::Planned,
    },
];

pub fn render_preview(result: &DiffResult, options: TuiRenderOptions) -> String {
    let width = options.width.max(80);
    let max_hunks = options.max_hunks.max(1);
    let max_lines_per_hunk = options.max_lines_per_hunk.max(1);
    let pane_width = ((width.saturating_sub(9)) / 2).max(30);
    let mut out = String::new();

    push_rule(&mut out, width);
    out.push_str("Diff Tool TUI scaffold\n");
    out.push_str(&format!(
        "Stats: +{} -{} ={} | files {} | keys: n/p hunk  / search  s split/unified  q quit\n",
        result.stats.additions,
        result.stats.deletions,
        result.stats.unchanged,
        result.files.len()
    ));
    if result.truncated {
        out.push_str("Warning: diff computation was truncated.\n");
    }
    for warning in &result.warnings {
        out.push_str("Warning: ");
        out.push_str(warning);
        out.push('\n');
    }
    push_rule(&mut out, width);

    if result.files.is_empty() {
        out.push_str("No file diffs to display.\n");
        push_rule(&mut out, width);
        return out;
    }

    for (file_index, file) in result.files.iter().enumerate() {
        if file_index > 0 {
            push_rule(&mut out, width);
        }
        render_file(
            &mut out,
            file,
            width,
            pane_width,
            max_hunks,
            max_lines_per_hunk,
        );
    }

    push_rule(&mut out, width);
    out.push_str("Scaffolded now: static split-pane render. Planned: interactive terminal backend, search index, syntax/theme layer, and apply/copy actions.\n");
    out
}

fn render_file(
    out: &mut String,
    file: &FileDiff,
    width: usize,
    pane_width: usize,
    max_hunks: usize,
    max_lines_per_hunk: usize,
) {
    out.push_str(&format!(
        "File: {} -> {}\n",
        file.left.display(),
        file.right.display()
    ));
    out.push_str(&format!(
        "{:<left_width$} | {:<right_width$}\n",
        "Original",
        "Modified",
        left_width = pane_width + 6,
        right_width = pane_width + 6
    ));
    push_rule(out, width);

    for (hunk_index, hunk) in file.hunks.iter().take(max_hunks).enumerate() {
        out.push_str(&format!(
            "Hunk {}: -{},{} +{},{}\n",
            hunk_index + 1,
            hunk.old_range.start,
            hunk.old_range.len,
            hunk.new_range.start,
            hunk.new_range.len
        ));

        let mut rendered_lines = 0;
        for line in &hunk.lines {
            if rendered_lines >= max_lines_per_hunk {
                out.push_str("  ... more lines hidden in scaffold preview\n");
                break;
            }
            push_line(out, line, pane_width);
            rendered_lines += 1;
        }
    }

    if file.hunks.len() > max_hunks {
        out.push_str(&format!(
            "... {} more hunks hidden in scaffold preview\n",
            file.hunks.len() - max_hunks
        ));
    }
}

fn push_line(out: &mut String, line: &LineChange, pane_width: usize) {
    match line.kind {
        ChangeKind::Context => {
            let text = truncate(&line.text, pane_width);
            out.push_str(&format!(
                "{:>4}   {:<pane_width$} | {:>4}   {:<pane_width$}\n",
                line.old_no
                    .map(format_line_no)
                    .unwrap_or_else(|| ".".to_string()),
                text,
                line.new_no
                    .map(format_line_no)
                    .unwrap_or_else(|| ".".to_string()),
                text,
                pane_width = pane_width
            ));
        }
        ChangeKind::Deletion => {
            out.push_str(&format!(
                "{:>4} - {:<pane_width$} | {:>4}   {:<pane_width$}\n",
                line.old_no
                    .map(format_line_no)
                    .unwrap_or_else(|| ".".to_string()),
                truncate(&line.text, pane_width),
                ".",
                "",
                pane_width = pane_width
            ));
        }
        ChangeKind::Addition => {
            out.push_str(&format!(
                "{:>4}   {:<pane_width$} | {:>4} + {:<pane_width$}\n",
                ".",
                "",
                line.new_no
                    .map(format_line_no)
                    .unwrap_or_else(|| ".".to_string()),
                truncate(&line.text, pane_width),
                pane_width = pane_width
            ));
        }
    }
}

fn push_rule(out: &mut String, width: usize) {
    out.push_str(&"-".repeat(width.min(160)));
    out.push('\n');
}

fn format_line_no(line: usize) -> String {
    line.to_string()
}

fn truncate(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut s: String = text.chars().take(keep).collect();
    s.push('~');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use diff_core::{DiffStats, FileDiff, Hunk, LineRange};
    use std::path::PathBuf;

    #[test]
    fn preview_renders_split_pane_landmarks() {
        let result = DiffResult {
            files: vec![FileDiff {
                left: PathBuf::from("left.txt"),
                right: PathBuf::from("right.txt"),
                hunks: vec![Hunk {
                    old_range: LineRange { start: 1, len: 2 },
                    new_range: LineRange { start: 1, len: 2 },
                    foldable_context: false,
                    lines: vec![
                        line(ChangeKind::Context, Some(1), Some(1), "same"),
                        line(ChangeKind::Deletion, Some(2), None, "old"),
                        line(ChangeKind::Addition, None, Some(2), "new"),
                    ],
                }],
            }],
            stats: DiffStats {
                additions: 1,
                deletions: 1,
                unchanged: 1,
            },
            warnings: vec![],
            truncated: false,
        };

        let rendered = render_preview(
            &result,
            TuiRenderOptions {
                width: 88,
                max_hunks: 4,
                max_lines_per_hunk: 20,
            },
        );

        assert!(rendered.contains("Diff Tool TUI scaffold"));
        assert!(rendered.contains("Original"));
        assert!(rendered.contains("Modified"));
        assert!(rendered.contains("keys: n/p hunk  / search"));
        assert!(rendered.contains("left.txt -> right.txt"));
        assert!(rendered.contains("old"));
        assert!(rendered.contains("new"));
    }

    #[test]
    fn feature_baseline_tracks_scaffold_and_planned_work() {
        assert!(FEATURE_BASELINE.iter().any(|feature| {
            feature.name == "side-by-side panes" && feature.status == TuiFeatureStatus::Scaffolded
        }));
        assert!(FEATURE_BASELINE.iter().any(|feature| {
            feature.name == "search" && feature.status == TuiFeatureStatus::Planned
        }));
        assert!(FEATURE_BASELINE.iter().any(|feature| {
            feature.name == "syntax highlighting" && feature.status == TuiFeatureStatus::Planned
        }));
    }

    fn line(
        kind: ChangeKind,
        old_no: Option<usize>,
        new_no: Option<usize>,
        text: &str,
    ) -> LineChange {
        LineChange {
            kind,
            old_no,
            new_no,
            text: text.to_string(),
            intra_spans: vec![],
        }
    }
}
