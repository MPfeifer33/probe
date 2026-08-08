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

fn write_nested_tauri_project(dir: &Path) {
    fs::create_dir_all(dir.join("app/src-tauri")).unwrap();
    fs::write(
        dir.join("app/package.json"),
        r#"{"name": "nested-app", "version": "0.1.0", "scripts": {"build": "vite build"}}"#,
    )
    .unwrap();
    fs::write(dir.join("app/package-lock.json"), "{}").unwrap();
    fs::write(
        dir.join("app/src-tauri/Cargo.toml"),
        r#"[package]
name = "nested-tauri"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
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
fn diff_accepts_legacy_snapshots_without_new_suite_fields() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);

    let snapshot_dir = dir.join(".agent-probe/snapshots");
    fs::create_dir_all(&snapshot_dir).unwrap();
    let snapshot_path = snapshot_dir.join("legacy.json");
    fs::write(
        &snapshot_path,
        format!(
            r#"{{
  "timestamp": "2026-06-22T03:39:00Z",
  "repo_path": "{}",
  "projects": [{{"kind":"rust","root":".","manifest":"Cargo.toml","metadata":{{"name":"sample"}}}}],
  "git": null,
  "tools": [],
  "lockfiles": [],
  "suggested_commands": [{{"action":"test","command":"cargo test","confidence":"high"}}]
}}"#,
            dir.display()
        ),
    )
    .unwrap();

    let output = probe(dir)
        .args([
            "--format",
            "json",
            "diff",
            ".agent-probe/snapshots/legacy.json",
        ])
        .output()
        .unwrap();

    assert_success(&output, "diff legacy snapshot");
}

#[test]
fn diff_reports_suite_tool_linkage_drift() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_rust_project(dir);

    let snapshot = json_output(
        probe(dir)
            .args(["--format", "json", "snapshot"])
            .output()
            .unwrap(),
        "snapshot json",
    );
    let snapshot_name = snapshot["snapshot"].as_str().unwrap();
    let snapshot_path = dir.join(".agent-probe/snapshots").join(snapshot_name);

    let content = fs::read_to_string(&snapshot_path).unwrap();
    let mut baseline: serde_json::Value = serde_json::from_str(&content).unwrap();
    let suite_tools = baseline["suite_tools"].as_array_mut().unwrap();
    let tool = suite_tools.first_mut().unwrap();
    let tool_name = tool["name"].as_str().unwrap().to_string();
    let current_initialized = tool["initialized"].as_bool().unwrap();
    let installed = tool["installed"].as_bool().unwrap();

    tool["initialized"] = serde_json::json!(!current_initialized);
    tool["state"] = serde_json::json!(if !current_initialized {
        "linked"
    } else if installed {
        "available"
    } else {
        "missing"
    });

    fs::write(
        &snapshot_path,
        serde_json::to_string_pretty(&baseline).unwrap(),
    )
    .unwrap();

    let diff = json_output(
        probe(dir)
            .args(["--format", "json", "diff", snapshot_path.to_str().unwrap()])
            .output()
            .unwrap(),
        "diff suite tools",
    );

    assert!(diff["diff"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|change| {
            change["kind"] == "suite_tool" && change["field"] == format!("{tool_name}.initialized")
        }));
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
    assert_eq!(doctor["doctor"]["action_level"], "stop");
    assert_eq!(doctor["doctor"]["schema_version"], "probe.doctor.v1");
    let blockers = doctor["doctor"]["blockers"].as_array().unwrap();
    assert!(blockers.iter().any(|issue| issue["code"] == "tool_missing"
        && issue["message"].as_str().unwrap().contains("cargo")));
    assert!(doctor["doctor"]["gates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|gate| gate["name"] == "tools" && gate["status"] == "stop"));
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
    assert_eq!(doctor["doctor"]["action_level"], "review");
    assert!(doctor["doctor"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "git_dirty"));
    assert!(doctor["doctor"]["recommended_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|command| {
            command["command"] == "cargo test"
                && command["argv"].as_array().unwrap() == &vec!["cargo", "test"]
                && command["reason"]
                    .as_str()
                    .is_some_and(|reason| !reason.is_empty())
        }));
    assert!(doctor["doctor"]["next_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|command| command["command"] == "cargo test"));
}

#[test]
fn doctor_reports_structured_gates_and_suite_tools_when_ready() {
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
    assert_success(
        &Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .output()
            .unwrap(),
        "git add",
    );
    assert_success(
        &Command::new("git")
            .args(["commit", "-m", "init", "--allow-empty"])
            .current_dir(dir)
            .output()
            .unwrap(),
        "git commit",
    );

    let doctor = json_output(
        probe(dir)
            .args(["--format", "json", "doctor"])
            .output()
            .unwrap(),
        "doctor json",
    );

    assert_eq!(doctor["doctor"]["status"], "ready");
    assert_eq!(doctor["doctor"]["action_level"], "validate");
    let gates = doctor["doctor"]["gates"].as_array().unwrap();
    for expected_gate in [
        "project",
        "git_state",
        "tools",
        "lockfiles",
        "commands",
        "suite_tools",
    ] {
        assert!(gates.iter().any(|gate| gate["name"] == expected_gate));
    }
    assert!(doctor["doctor"]["suite_tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "witness"));
}

#[test]
fn doctor_detects_nested_project_manifests_from_repo_root() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    write_nested_tauri_project(dir);

    assert_success(
        &Command::new("git")
            .arg("init")
            .current_dir(dir)
            .output()
            .unwrap(),
        "git init",
    );
    assert_success(
        &Command::new("git")
            .args(["add", "-A"])
            .current_dir(dir)
            .output()
            .unwrap(),
        "git add",
    );
    assert_success(
        &Command::new("git")
            .args(["commit", "-m", "init", "--allow-empty"])
            .current_dir(dir)
            .output()
            .unwrap(),
        "git commit",
    );

    let doctor = json_output(
        probe(dir)
            .args(["--format", "json", "doctor"])
            .output()
            .unwrap(),
        "doctor json",
    );

    assert_ne!(doctor["doctor"]["status"], "caution");
    assert!(!doctor["doctor"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|issue| issue["code"] == "no_projects_detected"));

    let gates = doctor["doctor"]["gates"].as_array().unwrap();
    assert!(gates.iter().any(|gate| {
        gate["name"] == "project"
            && gate["status"] == "ok"
            && gate["summary"]
                .as_str()
                .is_some_and(|summary| summary.contains("supported project stack"))
    }));
    assert!(doctor["doctor"]["recommended_commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|command| {
            command["cwd"] == "app" && command["command"] == "cd app && npm run build"
        }));
}
