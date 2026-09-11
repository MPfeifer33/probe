use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

use crate::detect::DetectedProject;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: Option<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteToolInfo {
    pub name: String,
    pub binary: String,
    pub role: String,
    pub installed: bool,
    pub version: Option<String>,
    pub state_path: Option<String>,
    pub initialized: bool,
    pub state: String,
}

pub fn detect_tools(projects: &[DetectedProject]) -> Vec<ToolInfo> {
    let mut tools = Vec::new();
    let mut checked = std::collections::HashSet::new();

    // Always check git
    tools.push(check_tool("git", &["--version"]));
    checked.insert("git");

    for project in projects {
        match project.kind.as_str() {
            "rust" | "tauri" => {
                if checked.insert("rustc") {
                    tools.push(check_tool("rustc", &["--version"]));
                }
                if checked.insert("cargo") {
                    tools.push(check_tool("cargo", &["--version"]));
                }
                if checked.insert("rustfmt") {
                    tools.push(check_tool("rustfmt", &["--version"]));
                }
                if checked.insert("clippy-driver") {
                    tools.push(check_tool_named(
                        "clippy",
                        "cargo",
                        &["clippy", "--version"],
                    ));
                }
            }
            "node" => {
                if checked.insert("node") {
                    tools.push(check_tool("node", &["--version"]));
                }
                let pm = project
                    .metadata
                    .get("package_manager")
                    .and_then(|v| v.as_str())
                    .unwrap_or("npm");
                if checked.insert(pm) {
                    tools.push(check_tool(pm, &["--version"]));
                }
            }
            "python" => {
                if checked.insert("python3") {
                    tools.push(check_tool("python3", &["--version"]));
                }
                if checked.insert("pip") {
                    tools.push(check_tool("pip", &["--version"]));
                }
            }
            "go" if checked.insert("go") => {
                tools.push(check_tool("go", &["version"]));
            }
            "go" => {}
            _ => {}
        }
    }

    tools
}

pub fn detect_suite_tools(repo: &Path) -> Vec<SuiteToolInfo> {
    suite_tool_definitions()
        .into_iter()
        .map(|definition| {
            let detected = check_suite_binary(definition.binary);
            let state_path = definition.state_path.map(|path| path.to_string());
            let initialized = state_path
                .as_ref()
                .map(|path| repo.join(path).exists())
                .unwrap_or(detected.available);
            let state = if !detected.available {
                "missing"
            } else if initialized {
                "linked"
            } else {
                "available"
            };

            SuiteToolInfo {
                name: definition.name.to_string(),
                binary: definition.binary.to_string(),
                role: definition.role.to_string(),
                installed: detected.available,
                version: detected.version,
                state_path,
                initialized,
                state: state.to_string(),
            }
        })
        .collect()
}

struct SuiteToolDefinition {
    name: &'static str,
    binary: &'static str,
    role: &'static str,
    state_path: Option<&'static str>,
}

/// The live suite as of 2026-09-11. Archived tools (atlas, stitch, trail,
/// harbor, loom, mender) are intentionally absent.
fn suite_tool_definitions() -> Vec<SuiteToolDefinition> {
    vec![
        SuiteToolDefinition {
            name: "probe",
            binary: "probe",
            role: "project preflight and drift scanner",
            state_path: Some(".agent-probe"),
        },
        SuiteToolDefinition {
            name: "latch",
            binary: "latch",
            role: "repo-local coordination ledger",
            state_path: Some(".agent-workspace/workspace.sqlite"),
        },
        SuiteToolDefinition {
            name: "sentinel",
            binary: "sentinel",
            role: "regression risk watcher",
            state_path: Some(".agent-sentinel/matrix.json"),
        },
        SuiteToolDefinition {
            name: "witness",
            binary: "witness",
            role: "command evidence recorder",
            state_path: Some(".agent-witness"),
        },
        SuiteToolDefinition {
            name: "switchboard",
            binary: "switchboard",
            role: "human/agent operations room",
            state_path: None,
        },
        SuiteToolDefinition {
            name: "sieve",
            binary: "sieve",
            role: "test impact recommender",
            state_path: None,
        },
        SuiteToolDefinition {
            name: "rivet",
            binary: "rivet",
            role: "patch intent verifier",
            state_path: None,
        },
        SuiteToolDefinition {
            name: "acurl",
            binary: "acurl",
            role: "agent-first HTTP client",
            state_path: Some(".agent-acurl"),
        },
        SuiteToolDefinition {
            name: "quarry",
            binary: "quarry",
            role: "dependency audit and update planner",
            state_path: None,
        },
    ]
}

fn check_tool(name: &str, args: &[&str]) -> ToolInfo {
    check_tool_named(name, name, args)
}

fn check_suite_binary(binary: &str) -> ToolInfo {
    let version_check = check_tool(binary, &["--version"]);
    if version_check.available {
        return version_check;
    }

    match Command::new(binary).arg("--help").output() {
        Ok(output) if output.status.success() => ToolInfo {
            name: binary.to_string(),
            version: None,
            available: true,
        },
        _ => version_check,
    }
}

fn check_tool_named(display_name: &str, binary: &str, args: &[&str]) -> ToolInfo {
    match Command::new(binary).args(args).output() {
        Ok(output) if output.status.success() => {
            let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let version = extract_version(&raw);
            ToolInfo {
                name: display_name.to_string(),
                version: Some(version),
                available: true,
            }
        }
        _ => ToolInfo {
            name: display_name.to_string(),
            version: None,
            available: false,
        },
    }
}

fn extract_version(raw: &str) -> String {
    // Try to extract just the version number from output like "rustc 1.79.0" or "v20.11.0"
    let first_line = raw.lines().next().unwrap_or(raw);
    // Look for a version-like pattern
    for word in first_line.split_whitespace() {
        let word = word.trim_start_matches('v');
        if word.chars().next().is_some_and(|c| c.is_ascii_digit()) && word.contains('.') {
            return word.to_string();
        }
    }
    first_line.to_string()
}
