//! Integration tests for the `probe brief` cold-start command.

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

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_git(dir: &Path) {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "probe@example.com"]);
    git(dir, &["config", "user.name", "Probe Test"]);
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "init: scaffold brief fixture"]);
}

fn write_rust_project(dir: &Path) {
    fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "brief-sample"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    fs::write(dir.join("Cargo.lock"), "# lock\n").unwrap();
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/main.rs"),
        "fn main() {\n    // TODO: wire the real entrypoint\n    // FIXME: remove placeholder\n}\n",
    )
    .unwrap();
}

fn write_project_md(dir: &Path) {
    fs::write(
        dir.join("PROJECT.md"),
        r#"# PROJECT.md — brief-sample

**What:** A fixture crate that exists so probe brief has something to summarize.

**Status:** Skeleton only; nothing runs yet.

**Tech:** Rust 2021.

## Layout
- `src/main.rs` — entrypoint

## Build
cargo build

## Last Updated
2026-09-11 — Fixture created for brief tests.
"#,
    )
    .unwrap();
}

fn write_readme(dir: &Path) {
    fs::write(
        dir.join("README.md"),
        r#"# brief-sample

A tiny crate used to exercise the cold-start brief when no project doc exists.
It has exactly one purpose: be summarized.

## Install

cargo install --path .

## Usage

Run it.
"#,
    )
    .unwrap();
}

#[test]
fn brief_json_with_project_md_merges_docs_scan_and_health() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);
    write_project_md(dir);
    init_git(dir);

    let json = json_output(
        probe(dir)
            .args(["brief", "--format", "json"])
            .output()
            .unwrap(),
        "brief json",
    );
    assert_eq!(json["ok"], true);
    let brief = &json["brief"];
    assert_eq!(brief["schema_version"], "probe.brief.v1");
    assert_eq!(brief["name"], dir.file_name().unwrap().to_str().unwrap());
    assert!(brief["timestamp"].as_str().is_some_and(|t| !t.is_empty()));

    // Docs: PROJECT.md is parsed for the front-matter lines and headings.
    let project_md = &brief["docs"]["project_md"];
    assert_eq!(project_md["path"], "PROJECT.md");
    assert!(project_md["what"]
        .as_str()
        .unwrap()
        .starts_with("A fixture crate"));
    assert_eq!(project_md["status"], "Skeleton only; nothing runs yet.");
    assert_eq!(
        project_md["last_updated"],
        "2026-09-11 — Fixture created for brief tests."
    );
    let headings: Vec<&str> = project_md["headings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h.as_str().unwrap())
        .collect();
    assert_eq!(headings, vec!["Layout", "Build", "Last Updated"]);
    assert!(brief["docs"]["readme"].is_null());

    // Stack + commands come from the existing scan/doctor composition.
    let projects = brief["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["kind"], "rust");
    assert_eq!(projects[0]["name"], "brief-sample");
    assert_eq!(projects[0]["root"], ".");
    let commands = brief["commands"].as_array().unwrap();
    assert!(commands
        .iter()
        .any(|c| c["action"] == "test" && c["command"] == "cargo test" && c["cwd"] == "."));

    // Git: branch, head, recent commits and total commit count.
    let git_state = &brief["git"];
    assert!(!git_state["branch"].as_str().unwrap().is_empty());
    assert!(!git_state["head_sha"].as_str().unwrap().is_empty());
    assert_eq!(git_state["commit_count"], 1);
    assert_eq!(git_state["dirty_count"], 0);
    assert_eq!(git_state["untracked_count"], 0);
    assert_eq!(git_state["changed_files"].as_array().unwrap().len(), 0);
    let recent = git_state["recent_commits"].as_array().unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0]["message"], "init: scaffold brief fixture");

    // Health mirrors doctor without repeating its gates.
    assert!(brief["health"]["status"].as_str().is_some());
    assert!(brief["health"]["action_level"].as_str().is_some());
    assert!(brief["health"]["blockers"].is_array());
    assert!(brief["health"]["warnings"].is_array());
    assert!(brief["health"].get("gates").is_none());

    // TODO markers are counted, with the hottest files listed.
    assert_eq!(brief["todos"]["total"], 2);
    assert_eq!(brief["todos"]["truncated"], false);
    let top = brief["todos"]["top_files"].as_array().unwrap();
    assert_eq!(top[0]["path"], "src/main.rs");
    assert_eq!(top[0]["count"], 2);

    // Lockfiles + tools are summarized, not hashed.
    assert_eq!(brief["lockfiles"]["tracked"], 1);
    assert!(brief["lockfiles"]["stale"].is_array());
    assert!(brief["tools"]["missing"].is_array());
    assert!(brief["tools"]["available"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t == "cargo"));
    assert!(brief["suite_tools"]["linked"].is_array());
    assert!(brief["suite_tools"]["missing"].is_array());
    assert!(brief["sentinel"].is_null());
}

#[test]
fn brief_text_with_project_md_is_compact_and_pasteable() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);
    write_project_md(dir);
    init_git(dir);

    let output = probe(dir).arg("brief").output().unwrap();
    assert_success(&output, "brief text");
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.starts_with("probe brief:"), "got: {stdout}");
    assert!(stdout.contains("What: A fixture crate"));
    assert!(stdout.contains("Status: Skeleton only"));
    assert!(stdout.contains("Last updated: 2026-09-11"));
    assert!(stdout.contains("PROJECT.md"));
    assert!(stdout.contains("rust \"brief-sample\""));
    assert!(stdout.contains("init: scaffold brief fixture"));
    assert!(stdout.contains("cargo test"));
    assert!(stdout.contains("TODO markers: 2"));
    assert!(stdout.contains("src/main.rs"));
    assert!(stdout.contains("Health:"));
    assert!(
        stdout.contains("probe scan"),
        "should point at the detailed view"
    );

    // Must not leak scan-only detail.
    assert!(!stdout.contains("Schema:"));
    assert!(!stdout.contains("Gates:"));

    let line_count = stdout.lines().count();
    assert!(
        line_count <= 60,
        "brief text should be pasteable (<= 60 lines), got {line_count}:\n{stdout}"
    );
}

#[test]
fn brief_without_project_md_falls_back_to_readme() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);
    write_readme(dir);
    init_git(dir);

    let json = json_output(
        probe(dir)
            .args(["brief", "--format", "json"])
            .output()
            .unwrap(),
        "brief json (readme only)",
    );
    let brief = &json["brief"];
    assert!(brief["docs"]["project_md"].is_null());
    let readme = &brief["docs"]["readme"];
    assert_eq!(readme["path"], "README.md");
    assert_eq!(readme["title"], "brief-sample");
    assert!(readme["summary"]
        .as_str()
        .unwrap()
        .starts_with("A tiny crate used to exercise"));
    let headings: Vec<&str> = readme["headings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h.as_str().unwrap())
        .collect();
    assert_eq!(headings, vec!["Install", "Usage"]);

    let output = probe(dir).arg("brief").output().unwrap();
    assert_success(&output, "brief text (readme only)");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("README.md"));
    assert!(stdout.contains("A tiny crate used to exercise"));
    assert!(!stdout.contains("PROJECT.md"));
}

#[test]
fn brief_lists_changed_files_when_dirty() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);
    write_project_md(dir);
    init_git(dir);
    fs::write(dir.join("src/main.rs"), "fn main() { println!(\"hi\"); }\n").unwrap();
    fs::write(dir.join("notes.txt"), "scratch\n").unwrap();

    let json = json_output(
        probe(dir)
            .args(["brief", "--format", "json"])
            .output()
            .unwrap(),
        "brief json (dirty)",
    );
    let git_state = &json["brief"]["git"];
    assert_eq!(git_state["dirty_count"], 1);
    assert_eq!(git_state["untracked_count"], 1);
    let changed = git_state["changed_files"].as_array().unwrap();
    assert!(changed
        .iter()
        .any(|c| c["path"] == "src/main.rs" && c["status"] == "M"));
    assert!(changed
        .iter()
        .any(|c| c["path"] == "notes.txt" && c["status"] == "??"));
    assert_eq!(git_state["changed_files_truncated"], false);

    let output = probe(dir).arg("brief").output().unwrap();
    assert_success(&output, "brief text (dirty)");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Changed:"));
    assert!(stdout.contains("src/main.rs"));
    assert!(stdout.contains("notes.txt"));
}

#[test]
fn brief_degrades_gracefully_without_stack_or_git() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    // Unity-shaped: no supported manifest, no git, but recognizable markers.
    fs::create_dir_all(dir.join("ProjectSettings")).unwrap();
    fs::write(
        dir.join("ProjectSettings/ProjectVersion.txt"),
        "m_EditorVersion: 6000.0.82f1\n",
    )
    .unwrap();
    fs::create_dir_all(dir.join("Assets/Scripts")).unwrap();
    fs::write(
        dir.join("Assets/Scripts/Bootstrap.cs"),
        "// TODO: build the scene at runtime\nclass Bootstrap {}\n",
    )
    .unwrap();
    // Library is generated and must not be walked for markers.
    fs::create_dir_all(dir.join("Library")).unwrap();
    fs::write(dir.join("Library/cache.txt"), "TODO TODO TODO\n").unwrap();
    fs::write(
        dir.join("PROJECT.md"),
        "# idle-sandbox\n\n**Purpose:** Unity sandbox for concepts.\n\n**Status:** scaffolded.\n",
    )
    .unwrap();

    let json = json_output(
        probe(dir)
            .args(["brief", "--format", "json"])
            .output()
            .unwrap(),
        "brief json (unity)",
    );
    let brief = &json["brief"];
    assert_eq!(brief["projects"].as_array().unwrap().len(), 0);
    assert!(brief["git"].is_null());
    assert_eq!(brief["commands"].as_array().unwrap().len(), 0);
    assert_eq!(brief["health"]["status"], "caution");
    let markers: Vec<&str> = brief["markers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    assert!(
        markers.iter().any(|m| m.contains("Unity")),
        "markers: {markers:?}"
    );
    // `**Purpose:**` is accepted as the "what" line.
    assert_eq!(
        brief["docs"]["project_md"]["what"],
        "Unity sandbox for concepts."
    );
    assert_eq!(brief["todos"]["total"], 1);

    let output = probe(dir).arg("brief").output().unwrap();
    assert_success(&output, "brief text (unity)");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Unity"));
    assert!(stdout.contains("not a git repository"));
    assert!(stdout.contains("none detected"));
    assert!(stdout.lines().count() <= 60);
}

#[test]
fn brief_reads_sentinel_summary_when_present() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);
    fs::create_dir_all(dir.join(".agent-sentinel")).unwrap();
    fs::write(
        dir.join(".agent-sentinel/matrix.json"),
        r#"{"summary": {"tracked_files": 12, "high_risk": 2, "medium_risk": 3}}"#,
    )
    .unwrap();

    let json = json_output(
        probe(dir)
            .args(["brief", "--format", "json"])
            .output()
            .unwrap(),
        "brief json (sentinel)",
    );
    let sentinel = &json["brief"]["sentinel"];
    assert_eq!(sentinel["tracked_files"], 12);
    assert_eq!(sentinel["high_risk"], 2);
    assert_eq!(sentinel["medium_risk"], 3);

    let output = probe(dir).arg("brief").output().unwrap();
    assert_success(&output, "brief text (sentinel)");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sentinel: 12 tracked, 2 high-risk, 3 medium-risk"));
}
