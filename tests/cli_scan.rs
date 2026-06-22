//! Integration tests for probe scan command

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
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    ).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
}

fn create_node_project(dir: &std::path::Path) {
    std::fs::write(
        dir.join("package.json"),
        r#"{"name": "test-app", "version": "1.0.0"}"#,
    ).unwrap();
}

fn init_git(dir: &std::path::Path) {
    Command::new("git").args(["init"]).current_dir(dir).output().unwrap();
    Command::new("git").args(["add", "-A"]).current_dir(dir).output().unwrap();
    Command::new("git").args(["commit", "-m", "init", "--allow-empty"]).current_dir(dir).output().unwrap();
}

#[test]
fn scan_detects_rust_project() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);

    let output = probe(dir).args(["scan", "--format", "json"]).output().unwrap();
    assert!(output.status.success(), "scan failed: {}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["ok"], true);

    let projects = json["scan"]["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["kind"], "rust");
    assert_eq!(projects[0]["metadata"]["name"], "test-project");
}

#[test]
fn scan_detects_node_project() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_node_project(dir);

    let output = probe(dir).args(["scan", "--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let projects = json["scan"]["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["kind"], "node");
    assert_eq!(projects[0]["metadata"]["name"], "test-app");
}

#[test]
fn scan_detects_multiple_stacks() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);
    create_node_project(dir);

    let output = probe(dir).args(["scan", "--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let projects = json["scan"]["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    let kinds: Vec<&str> = projects.iter().map(|p| p["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"rust"));
    assert!(kinds.contains(&"node"));
}

#[test]
fn scan_reports_git_state() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);
    init_git(dir);

    let output = probe(dir).args(["scan", "--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let git = &json["scan"]["git"];
    assert!(!git.is_null());
    assert!(!git["branch"].as_str().unwrap().is_empty());
    assert!(!git["head_sha"].as_str().unwrap().is_empty());
}

#[test]
fn scan_no_git_reports_null() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);
    // No git init

    let output = probe(dir).args(["scan", "--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["scan"]["git"].is_null());
}

#[test]
fn scan_reports_tools() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);

    let output = probe(dir).args(["scan", "--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let tools = json["scan"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty());

    // Should have git, rustc, cargo at minimum
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"git"));
    assert!(names.contains(&"rustc"));
    assert!(names.contains(&"cargo"));
}

#[test]
fn scan_suggests_commands_for_rust() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);

    let output = probe(dir).args(["scan", "--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let commands = json["scan"]["suggested_commands"].as_array().unwrap();
    let actions: Vec<&str> = commands.iter().map(|c| c["action"].as_str().unwrap()).collect();
    assert!(actions.contains(&"check"));
    assert!(actions.contains(&"test"));
    assert!(actions.contains(&"build"));
}

#[test]
fn scan_text_output_contains_project_info() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);

    let output = probe(dir).args(["scan"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("rust"));
    assert!(stdout.contains("test-project"));
    assert!(stdout.contains("cargo check"));
}

#[test]
fn scan_detects_lockfile_with_hash() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    create_rust_project(dir);
    std::fs::write(dir.join("Cargo.lock"), "# dummy lockfile\n").unwrap();

    let output = probe(dir).args(["scan", "--format", "json"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let lockfiles = json["scan"]["lockfiles"].as_array().unwrap();
    assert_eq!(lockfiles.len(), 1);
    assert_eq!(lockfiles[0]["path"], "Cargo.lock");
    assert!(!lockfiles[0]["hash"].as_str().unwrap().is_empty());
}
