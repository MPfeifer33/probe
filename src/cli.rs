use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::ProbeError;

#[derive(Parser, Debug)]
#[command(name = "probe", version, about = "Agent preflight and drift scanner")]
pub struct Cli {
    /// Project root override
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn resolve_repo(&self) -> Result<PathBuf, ProbeError> {
        if let Some(ref repo) = self.repo {
            return Ok(repo.clone());
        }
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok(PathBuf::from(path));
            }
        }
        std::env::current_dir().map_err(ProbeError::Io)
    }

    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Text,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scan the project: detect stack, git state, tools, lockfiles
    Scan,

    /// Save current scan as a snapshot for later diff
    Snapshot,

    /// Compare current state against a saved snapshot
    Diff {
        /// Snapshot to compare against (path or "latest")
        #[arg(default_value = "latest")]
        against: String,
    },

    /// Actionable preflight summary: blockers, warnings, suggested commands
    Doctor,
}
