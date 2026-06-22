use std::process::Command;
use serde::{Deserialize, Serialize};

use crate::detect::DetectedProject;

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub version: Option<String>,
    pub available: bool,
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
                    tools.push(check_tool_named("clippy", "cargo", &["clippy", "--version"]));
                }
            }
            "node" => {
                if checked.insert("node") {
                    tools.push(check_tool("node", &["--version"]));
                }
                let pm = project.metadata.get("package_manager")
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
            "go" => {
                if checked.insert("go") {
                    tools.push(check_tool("go", &["version"]));
                }
            }
            _ => {}
        }
    }

    tools
}

fn check_tool(name: &str, args: &[&str]) -> ToolInfo {
    check_tool_named(name, name, args)
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
        if word.chars().next().map_or(false, |c| c.is_ascii_digit())
            && word.contains('.')
        {
            return word.to_string();
        }
    }
    first_line.to_string()
}
