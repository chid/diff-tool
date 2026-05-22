use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
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
        .success()
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

    assert!(output.status.success());
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
