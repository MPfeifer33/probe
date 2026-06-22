//! Integration tests for probe snapshot command

use std::process::Command;
use tempfile::TempDir;

fn probe(dir: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_probe"));
    cmd.arg("--repo").arg(dir);
    cmd
}

fn create_rust_project(dir: &std::path::Path) {
    std::fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "snap-test"
version = "0.1.0"
edition = "2021"
"#,
    ).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
}

#[test]
fn snapshot_creates_file() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);

    let output = probe(dir).args(["snapshot"]).output().unwrap();
    assert!(output.status.success(), "snapshot failed: {}", String::from_utf8_lossy(&output.stderr));

    // Check .agent-probe/snapshots/ exists and has a file
    let snapshots_dir = dir.join(".agent-probe/snapshots");
    assert!(snapshots_dir.exists());

    let entries: Vec<_> = std::fs::read_dir(&snapshots_dir).unwrap().collect();
    assert_eq!(entries.len(), 1);

    // Verify it's valid JSON
    let path = entries[0].as_ref().unwrap().path();
    let content = std::fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["projects"][0]["kind"], "rust");
}

#[test]
fn snapshot_creates_gitignore() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);

    probe(dir).args(["snapshot"]).output().unwrap();

    let gitignore = dir.join(".agent-probe/.gitignore");
    assert!(gitignore.exists());
    let content = std::fs::read_to_string(gitignore).unwrap();
    assert!(content.contains("*"));
}

#[test]
fn snapshot_json_output() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);

    let output = probe(dir).args(["snapshot", "--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["ok"], true);
}

#[test]
fn multiple_snapshots_accumulate() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);

    probe(dir).args(["snapshot"]).output().unwrap();
    // Small delay to get different timestamp
    std::thread::sleep(std::time::Duration::from_millis(1100));
    probe(dir).args(["snapshot"]).output().unwrap();

    let snapshots_dir = dir.join(".agent-probe/snapshots");
    let entries: Vec<_> = std::fs::read_dir(&snapshots_dir).unwrap().collect();
    assert_eq!(entries.len(), 2);
}
