//! Integration tests for diff and doctor commands.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn probe(dir: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_probe"));
    cmd.arg("--repo").arg(dir);
    cmd
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn json_output(output: Output, label: &str) -> serde_json::Value {
    assert_success(&output, label);
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "{label} returned invalid json: {err}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn write_rust_project(dir: &Path) {
    fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "sample"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(dir.join("Cargo.lock"), "# baseline lock\n").unwrap();
}

#[test]
fn diff_reports_lockfile_drift_against_latest_snapshot() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);

    assert_success(
        &probe(dir).arg("snapshot").output().unwrap(),
        "snapshot baseline",
    );

    fs::write(dir.join("Cargo.lock"), "# changed lock\n").unwrap();

    let diff = json_output(
        probe(dir)
            .args(["--format", "json", "diff"])
            .output()
            .unwrap(),
        "diff latest",
    );

    assert!(diff["diff"]["summary"]["changes"].as_u64().unwrap() > 0);
    let changes = diff["diff"]["changes"].as_array().unwrap();
    assert!(changes.iter().any(|change| {
        change["kind"] == "lockfile"
            && change["field"] == "Cargo.lock.hash"
            && change["severity"] == "warning"
    }));
}

#[test]
fn diff_text_reports_no_changes_after_snapshot() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);

    assert_success(
        &probe(dir).arg("snapshot").output().unwrap(),
        "snapshot baseline",
    );

    let output = probe(dir).arg("diff").output().unwrap();
    assert_success(&output, "diff text");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("probe diff"));
    assert!(stdout.contains("No drift detected"));
}

#[test]
fn doctor_blocks_when_required_tools_are_missing() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);

    let doctor = json_output(
        probe(dir)
            .env("PATH", "")
            .args(["--format", "json", "doctor"])
            .output()
            .unwrap(),
        "doctor json",
    );

    assert_eq!(doctor["doctor"]["status"], "blocked");
    let blockers = doctor["doctor"]["blockers"].as_array().unwrap();
    assert!(blockers.iter().any(|issue| issue["code"] == "tool_missing"
        && issue["message"].as_str().unwrap().contains("cargo")));
}

#[test]
fn doctor_warns_about_dirty_git_state_and_suggests_commands() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);

    assert_success(
        &Command::new("git")
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap(),
        "git init",
    );

    let doctor = json_output(
        probe(dir)
            .args(["--format", "json", "doctor"])
            .output()
            .unwrap(),
        "doctor json",
    );

    assert_eq!(doctor["doctor"]["status"], "caution");
    assert!(doctor["doctor"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "git_dirty"));
    assert!(doctor["doctor"]["next_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|command| command["command"] == "cargo test"));
}
