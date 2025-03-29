use colored::Colorize;
use std::fmt;

use crate::reporting::Reporter;

#[derive(Debug)]
pub enum AppError {
    TestRunnerNotInstalled {
        runner_name: String,
        installation_tip: String,
    },
    TestsFailed,
    GitDiscoveryFailed {
        reason: String,
    },
    MetadataFailed {
        reason: String,
    },
    GitOperationFailed {
        operation: String,
        reason: String,
    },
    CommandFailed {
        command: String,
        reason: String,
    },
    Other(anyhow::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::TestRunnerNotInstalled { runner_name, .. } => {
                write!(f, "test runner '{}' is not installed", runner_name)
            }
            AppError::TestsFailed { .. } => {
                write!(f, "test failed")
            }
            AppError::GitDiscoveryFailed { reason } => {
                write!(f, "failed to discover git repository: {}", reason)
            }
            AppError::MetadataFailed { reason } => {
                write!(f, "failed to retrieve cargo metadata: {}", reason)
            }
            AppError::GitOperationFailed { operation, reason } => {
                write!(f, "git operation '{}' failed: {}", operation, reason)
            }
            AppError::CommandFailed { command, reason } => {
                write!(f, "command '{}' failed: {}", command, reason)
            }
            AppError::Other(err) => {
                write!(f, "{}", err)
            }
        }
    }
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
            AppError::TestsFailed { .. } => {
                reporter.error("test failed");
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
