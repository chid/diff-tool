use diff_core::{DiffMode, DiffRequest, IgnoreMode};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct CaseMeta {
    name: String,
    ignore: IgnoreMode,
    expected: ExpectedStats,
}

#[derive(Debug, Deserialize)]
struct ExpectedStats {
    additions: usize,
    deletions: usize,
    unchanged: usize,
    truncated: bool,
}

#[test]
fn fixture_cases_match_expected_stats() {
    for case_dir in fixture_case_dirs() {
        let meta: CaseMeta =
            serde_json::from_str(&fs::read_to_string(case_dir.join("case.json")).unwrap()).unwrap();
        let result = diff_core::compare_files(DiffRequest {
            left: case_dir.join("left.txt"),
            right: case_dir.join("right.txt"),
            mode: DiffMode::Split,
            ignore: meta.ignore,
            context: 3,
            max_bytes: None,
        })
        .unwrap();

        assert_eq!(
            result.stats.additions, meta.expected.additions,
            "{}",
            meta.name
        );
        assert_eq!(
            result.stats.deletions, meta.expected.deletions,
            "{}",
            meta.name
        );
        assert_eq!(
            result.stats.unchanged, meta.expected.unchanged,
            "{}",
            meta.name
        );
        assert_eq!(result.truncated, meta.expected.truncated, "{}", meta.name);
    }
}

#[test]
fn diff_result_json_shape_is_stable() {
    let case_dir = fixture_root().join("basic-replacement");
    let result = diff_core::compare_files(DiffRequest {
        left: case_dir.join("left.txt"),
        right: case_dir.join("right.txt"),
        mode: DiffMode::Split,
        ignore: IgnoreMode::None,
        context: 3,
        max_bytes: None,
    })
    .unwrap();

    let json = serde_json::to_value(&result).unwrap();
    assert!(json.get("files").is_some());
    assert_eq!(json["stats"]["additions"], 1);
    assert_eq!(json["stats"]["deletions"], 1);
    assert_eq!(json["truncated"], false);
}

#[test]
fn max_bytes_marks_truncated_results() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.txt");
    let right = dir.path().join("right.txt");
    fs::write(&left, "alpha\nbeta\ngamma\n").unwrap();
    fs::write(&right, "alpha\nbeta\ndelta\n").unwrap();

    let result = diff_core::compare_files(DiffRequest {
        left,
        right,
        mode: DiffMode::Split,
        ignore: IgnoreMode::None,
        context: 3,
        max_bytes: Some(8),
    })
    .unwrap();

    assert!(result.truncated);
}

fn fixture_case_dirs() -> Vec<PathBuf> {
    let mut dirs = fs::read_dir(fixture_root())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("case.json").exists())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/cases")
}
