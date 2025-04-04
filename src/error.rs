use colored::Colorize;
use thiserror::Error;

use crate::reporting::Reporter;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("test runner '{runner_name}' is not installed")]
    TestRunnerNotInstalled {
        runner_name: String,
        installation_tip: String,
    },
    #[error("test failed")]
    TestsFailed { failed_crates: Vec<String> },
    #[error("failed to discover git repository: {reason}")]
    GitDiscoveryFailed { reason: String },
    #[error("failed to retrieve cargo metadata: {reason}")]
    MetadataFailed { reason: String },
    #[error("git operation '{operation}' failed: {reason}")]
    GitOperationFailed { operation: String, reason: String },
    #[error("command '{command}' failed: {reason}")]
    CommandFailed { command: String, reason: String },
    #[error("unknown crate '{crate_name}'")]
    UnknownCrate { crate_name: String },
    #[error("invalid arguments: {reason}")]
    InvalidArguments { reason: String },
    #[error("{0}")]
    Other(anyhow::Error),
}

impl AppError {
    /// Map error types to exit codes
    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::TestRunnerNotInstalled { .. } => 10,
            AppError::TestsFailed { .. } => 20,
            AppError::GitDiscoveryFailed { .. } => 30,
            AppError::MetadataFailed { .. } => 40,
            AppError::GitOperationFailed { .. } => 50,
            AppError::CommandFailed { .. } => 60,
            AppError::UnknownCrate { .. } => 70,
            AppError::InvalidArguments { .. } => 80,
            AppError::Other(_) => 1,
        }
    }

    /// Handle printing the error and any additional context
    pub fn report<R: Reporter>(&self, reporter: &mut R) {
        match self {
            AppError::TestRunnerNotInstalled {
                runner_name,
                installation_tip,
            } => {
                reporter.error(&format!("test runner '{}' is not installed", runner_name));
                reporter.tip(installation_tip);
            }
            AppError::TestsFailed { failed_crates } => {
                let rerun_command = format!("-c {}", failed_crates.join(","));
                reporter.error(&format!(
                    "test failed, to rerun pass `{}`",
                    rerun_command.bold().yellow(),
                ));
            }
            AppError::GitDiscoveryFailed { reason } => {
                reporter.error(&format!(
                    "failed to discover git repository: {}",
                    reason.bold()
                ));
            }
            AppError::MetadataFailed { reason } => {
                reporter.error(&format!(
                    "failed to retrieve cargo metadata: {}",
                    reason.bold()
                ));
            }
            AppError::GitOperationFailed { operation, reason } => {
                reporter.error(&format!(
                    "git operation '{}' failed: {}",
                    operation.bold().yellow(),
                    reason.bold()
                ));
            }
            AppError::CommandFailed { command, reason } => {
                reporter.error(&format!(
                    "command '{}' failed: {}",
                    command.bold().yellow(),
                    reason.bold()
                ));
            }
            AppError::UnknownCrate { crate_name } => {
                reporter.error(&format!("unknown crate '{}'", crate_name.bold().yellow()));
            }
            AppError::InvalidArguments { reason } => {
                reporter.error(&format!("invalid arguments: {}", reason.bold().yellow()));
            }
            AppError::Other(err) => {
                reporter.error(&format!("{}", err));
            }
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Other(err)
    }
}
