use colored::Colorize;
use std::fmt;

use crate::format;

#[derive(Debug)]
pub enum AppError {
    TestRunnerNotInstalled {
        runner_name: String,
        installation_tip: String,
    },
    TestsFailed {
        crate_name: String,
    },
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

// Implement Display for uncolored error messages (for logging, etc.)
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::TestRunnerNotInstalled { runner_name, .. } => {
                write!(f, "test runner '{}' is not installed", runner_name)
            }
            AppError::TestsFailed { crate_name } => {
                write!(f, "tests failed for crate: {}", crate_name)
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

// Map error types to exit codes and provide colorized output
impl AppError {
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

    // Create a colored version of the error message
    fn colorized_message(&self) -> String {
        match self {
            AppError::TestRunnerNotInstalled { runner_name, .. } => {
                format!(
                    "test runner '{}' is not installed",
                    runner_name.bold().yellow()
                )
            }

            AppError::TestsFailed { crate_name } => {
                format!("tests failed for crate: {}", crate_name.bold().yellow())
            }

            AppError::GitDiscoveryFailed { reason } => {
                format!("failed to discover git repository: {}", reason.bold())
            }

            AppError::MetadataFailed { reason } => {
                format!("failed to retrieve cargo metadata: {}", reason.bold())
            }

            AppError::GitOperationFailed { operation, reason } => {
                format!(
                    "git operation '{}' failed: {}",
                    operation.bold().yellow(),
                    reason.bold()
                )
            }

            AppError::CommandFailed { command, reason } => {
                format!(
                    "command '{}' failed: {}",
                    command.bold().yellow(),
                    reason.bold()
                )
            }

            AppError::Other(err) => {
                format!("{}", err)
            }
        }
    }

    // Handle printing the error and any additional context
    pub fn report(&self) {
        format::error!(self.colorized_message());

        // Add additional context for specific error types
        match self {
            AppError::TestRunnerNotInstalled {
                installation_tip, ..
            } => {
                println!();
                format::tip!(installation_tip);
            }
            _ => {}
        }
    }
}

// Implement conversion from anyhow::Error to AppError
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Other(err)
    }
}
