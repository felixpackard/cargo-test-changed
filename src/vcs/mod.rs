use std::path::{Path, PathBuf};

use crate::error::AppError;

mod git;

use clap::ValueEnum;
pub use git::GitVcs;

pub trait Vcs {
    /// Retrieve the workspace root directory
    fn get_workspace_root(&self) -> Result<PathBuf, AppError>;

    /// Get list of changed files from repository
    fn get_changed_files(&self, workspace_root: &Path) -> Result<Vec<PathBuf>, AppError>;
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
