mod cli;
mod scan;
mod snapshot;
mod detect;
mod git;
mod tools;
mod report;

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
                eprintln!("{}", serde_json::to_string_pretty(&err_json).unwrap());
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
            snapshot::save(&repo, &result)?;
            if cli.is_json() {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "ok": true,
                    "message": "Snapshot saved",
                }))?);
            } else {
                println!("Snapshot saved.");
            }
            Ok(())
        }
        Command::Diff { against } => {
            let repo = cli.resolve_repo()?;
            let _current = scan::run_scan(&repo)?;
            // Stub: Bjarn will implement diff
            let _ = against;
            Err(ProbeError::NotImplemented("diff not yet implemented".into()))
        }
        Command::Doctor => {
            let repo = cli.resolve_repo()?;
            let _result = scan::run_scan(&repo)?;
            // Stub: Bjarn will implement doctor
            Err(ProbeError::NotImplemented("doctor not yet implemented".into()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotImplemented(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ProbeError {
    pub fn exit_code(&self) -> i32 {
        match self {
            ProbeError::Validation(_) => 1,
            ProbeError::NotImplemented(_) => 1,
            ProbeError::Io(_) => 2,
            ProbeError::Json(_) => 1,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            ProbeError::Validation(_) => "validation_error",
            ProbeError::NotImplemented(_) => "not_implemented",
            ProbeError::Io(_) => "io_error",
            ProbeError::Json(_) => "json_error",
        }
    }
}
