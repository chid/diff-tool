use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn diff_prints_human_summary() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.txt");
    let right = dir.path().join("right.txt");
    fs::write(&left, "one\ntwo\n").unwrap();
    fs::write(&right, "one\nthree\n").unwrap();

    let mut cmd = Command::cargo_bin("dt").unwrap();
    cmd.arg("diff")
        .arg(&left)
        .arg(&right)
        .assert()
        .code(1)
        .stdout(predicate::str::contains(
            "1 additions, 1 deletions, 1 unchanged",
        ));
}

#[test]
fn diff_json_is_parseable() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("left.txt");
    let right = dir.path().join("right.txt");
    fs::write(&left, "one\ntwo\n").unwrap();
    fs::write(&right, "one\nthree\n").unwrap();

    let output = Command::cargo_bin("dt")
        .unwrap()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "diff with differences exits 1");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["stats"]["additions"], 1);
    assert_eq!(json["stats"]["deletions"], 1);
}

#[test]
fn dir_json_is_parseable() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();

    let output = Command::cargo_bin("dt")
        .unwrap()
        .args(["dir"])
        .arg(dir.path())
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 2);
}

// ── Snapshot tests ────────────────────────────────────────────────────

fn fixture_path(dir: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/cases")
        .join(dir)
}

fn cmd() -> Command {
    Command::cargo_bin("dt").expect("dt binary not found — build with `cargo build` first")
}

#[test]
fn snapshot_diff_human_basic_replacement() {
    let case = fixture_path("basic-replacement");
    let left = case.join("left.txt");
    let right = case.join("right.txt");

    let output = cmd()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "diff with differences exits 1");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    insta::assert_snapshot!("diff_human_basic_replacement", stdout);
}

#[test]
fn snapshot_diff_json_basic_replacement() {
    let case = fixture_path("basic-replacement");
    let left = case.join("left.txt");
    let right = case.join("right.txt");

    let output = cmd()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "diff with differences exits 1");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    insta::assert_snapshot!("diff_json_basic_replacement", stdout);
}

#[test]
fn snapshot_dir_human_folder_basic_left() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/folder-basic/left");

    let output = cmd()
        .args(["dir"])
        .arg(&dir)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    insta::assert_snapshot!("dir_human_folder_basic_left", stdout);
}

#[test]
fn snapshot_dir_json_folder_basic_left() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/folder-basic/left");

    let output = cmd()
        .args(["dir"])
        .arg(&dir)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    insta::assert_snapshot!("dir_json_folder_basic_left", stdout);
}

#[test]
fn snapshot_diff_json_multi_hunk() {
    // Multi-hunk output with intraline spans — the most complex JSON shape.
    let case = fixture_path("multi-hunk");
    let left = case.join("left.txt");
    let right = case.join("right.txt");

    let output = cmd()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "diff with differences exits 1");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    insta::assert_snapshot!("diff_json_multi_hunk", stdout);
}

#[test]
fn snapshot_diff_json_with_warnings() {
    // Invalid UTF-8 in one file triggers the warnings field in JSON output.
    let dir = tempdir().unwrap();
    let bad = dir.path().join("bad.txt");
    let good = dir.path().join("good.txt");
    fs::write(&bad, &[0x48, 0x65, 0x6c, 0x6c, 0x6f, 0xFF]).unwrap();
    fs::write(&good, b"hello\n").unwrap();

    let output = cmd()
        .args(["diff"])
        .arg(&bad)
        .arg(&good)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1), "diff with differences exits 1");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    // The temp path is non-deterministic; redact it before snapshotting.
    let redacted = stdout.replace(
        dir.path().to_str().unwrap(),
        "<TMPDIR>",
    );
    insta::assert_snapshot!("diff_json_with_warnings", redacted);
}

// ── Exit code tests ───────────────────────────────────────────────────

#[test]
fn exit_0_when_identical() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("a.txt");
    let right = dir.path().join("b.txt");
    fs::write(&left, "same\ncontent\n").unwrap();
    fs::write(&right, "same\ncontent\n").unwrap();

    cmd()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .assert()
        .code(0);
}

#[test]
fn exit_1_when_different() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("a.txt");
    let right = dir.path().join("b.txt");
    fs::write(&left, "old\n").unwrap();
    fs::write(&right, "new\n").unwrap();

    cmd()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .assert()
        .code(1);
}

#[test]
fn exit_2_when_missing_file() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("a.txt");
    let right = dir.path().join("b.txt");
    fs::write(&left, "hello\n").unwrap();
    // right does not exist

    cmd()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read"));
}

#[test]
fn exit_2_when_binary() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("a.bin");
    let right = dir.path().join("b.txt");
    fs::write(&left, &[0x00, 0x01, 0x02]).unwrap();
    fs::write(&right, b"hello\n").unwrap();

    cmd()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("binary"));
}

#[test]
fn dir_exit_0_on_success() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("f.txt"), "hi\n").unwrap();

    cmd()
        .args(["dir"])
        .arg(dir.path())
        .assert()
        .code(0);
}

#[test]
fn dir_exit_2_when_missing() {
    // scan_tree previously returned 0 entries for nonexistent roots;
    // the CLI now explicitly checks and errors before invoking scan_tree.
    let missing = PathBuf::from("/nonexistent/dir/path");

    cmd()
        .args(["dir"])
        .arg(&missing)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn dir_exit_2_when_file_not_dir() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("not-a-dir");
    fs::write(&file, "hello\n").unwrap();

    cmd()
        .args(["dir"])
        .arg(&file)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not a directory"));
}

#[test]
fn merge_exit_2_not_implemented() {
    let dir = tempdir().unwrap();
    let base = dir.path().join("base.txt");
    let ours = dir.path().join("ours.txt");
    let theirs = dir.path().join("theirs.txt");
    let out = dir.path().join("out.txt");
    fs::write(&base, "base\n").unwrap();
    fs::write(&ours, "ours\n").unwrap();
    fs::write(&theirs, "theirs\n").unwrap();

    cmd()
        .args(["merge"])
        .arg("--base")
        .arg(&base)
        .arg("--ours")
        .arg(&ours)
        .arg("--theirs")
        .arg(&theirs)
        .arg("--out")
        .arg(&out)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not implemented"));
}

#[test]
fn git_install_exit_2_not_implemented() {
    cmd()
        .args(["git", "install"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("not implemented"));
}

#[test]
fn exit_0_when_identical_json() {
    let dir = tempdir().unwrap();
    let left = dir.path().join("a.txt");
    let right = dir.path().join("b.txt");
    fs::write(&left, "same\n").unwrap();
    fs::write(&right, "same\n").unwrap();

    let output = cmd()
        .args(["diff"])
        .arg(&left)
        .arg(&right)
        .arg("--json")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0), "identical files exit 0");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["stats"]["additions"], 0);
    assert_eq!(json["stats"]["deletions"], 0);
}
