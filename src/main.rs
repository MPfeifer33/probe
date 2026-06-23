mod cli;
mod detect;
mod diff;
mod doctor;
mod git;
mod report;
mod scan;
mod snapshot;
mod tools;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();
    let result = run(&cli);
    match result {
        Ok(()) => {}
        Err(e) => {
            let code = e.exit_code();
            if cli.is_json() {
                let err_json = serde_json::json!({
                    "ok": false,
                    "error": {
                        "code": e.error_code(),
                        "message": e.to_string(),
                    }
                });
                eprintln!("{}", serde_json::to_string_pretty(&err_json).unwrap_or_else(|_| format!("{{\"ok\":false,\"error\":{{\"message\":\"{e}\"}}}}")));
            } else {
                eprintln!("error: {e}");
            }
            std::process::exit(code);
        }
    }
}

fn run(cli: &Cli) -> Result<(), ProbeError> {
    match &cli.command {
        Command::Scan => {
            let repo = cli.resolve_repo()?;
            let result = scan::run_scan(&repo)?;
            report::print_scan(&result, cli.is_json())?;
            Ok(())
        }
        Command::Snapshot => {
            let repo = cli.resolve_repo()?;
            let result = scan::run_scan(&repo)?;
            let filename = snapshot::save(&repo, &result)?;
            if cli.is_json() {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "message": "Snapshot saved",
                        "snapshot": filename,
                    }))?
                );
            } else {
                println!("Snapshot saved: {filename}");
            }
            Ok(())
        }
        Command::Diff { against } => {
            let repo = cli.resolve_repo()?;
            let current = scan::run_scan(&repo)?;
            let baseline_path = resolve_snapshot_path(&repo, against)?;
            let baseline = snapshot::load_snapshot(&baseline_path)?;
            let diff = diff::build_report(&current, &baseline, &baseline_path);
            report::print_diff(&diff, cli.is_json())?;
            Ok(())
        }
        Command::Doctor => {
            let repo = cli.resolve_repo()?;
            let result = scan::run_scan(&repo)?;
            let doctor = doctor::build_report(&result);
            report::print_doctor(&doctor, cli.is_json())?;
            Ok(())
        }
    }
}

fn resolve_snapshot_path(
    repo: &std::path::Path,
    against: &str,
) -> Result<std::path::PathBuf, ProbeError> {
    if against == "latest" {
        return snapshot::latest_snapshot_path(repo).ok_or_else(|| {
            ProbeError::Validation("No snapshots found; run `probe snapshot` first".into())
        });
    }

    let path = std::path::PathBuf::from(against);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(repo.join(path))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("{0}")]
    Validation(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ProbeError {
    pub fn exit_code(&self) -> i32 {
        match self {
            ProbeError::Validation(_) => 1,
            ProbeError::Io(_) => 2,
            ProbeError::Json(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            ProbeError::Validation(_) => "validation_error",
            ProbeError::Io(_) => "io_error",
            ProbeError::Json(_) => "json_error",
        }
    }
}
