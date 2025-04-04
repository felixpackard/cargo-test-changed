use clap::ValueEnum;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub use git::GitVcs;

use crate::error::AppError;

mod git;
#[cfg(test)]
mod git_tests;
#[cfg(test)]
mod test_utils;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Added,
    Modified,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ChangedFile {
    pub current_path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub file_type: FileType,
    pub change_type: ChangeType,
}

pub trait Vcs {
    /// Retrieve the workspace root directory
    fn get_workspace_root(&self, path: &Path) -> Result<PathBuf, AppError>;

    /// Get list of uncommitted files (both staged and unstaged changes)
    fn get_uncommitted_changes(&self, workspace_root: &Path) -> Result<Vec<ChangedFile>, AppError>;

    /// Get list of files changed between two points in history
    ///
    /// `from_ref` - The starting reference point
    /// `to_ref` - The ending reference point (defaults to current state if None)
    fn get_changes_between(
        &self,
        workspace_root: &Path,
        from_ref: &str,
        to_ref: Option<&str>,
    ) -> Result<Vec<ChangedFile>, AppError>;
}

#[derive(Debug, Clone, ValueEnum)]
pub enum VcsType {
    Git,
}

impl VcsType {
    pub fn create(&self) -> Box<dyn Vcs> {
        match self {
            VcsType::Git => Box::new(GitVcs),
        }
    }
}
