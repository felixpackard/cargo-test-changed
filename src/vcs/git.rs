use gix::{bstr::ByteSlice, status::index_worktree::iter::Summary};
use indexmap::IndexSet;

use crate::error::AppError;

use super::Vcs;

pub struct GitVcs;

impl Vcs for GitVcs {
    fn get_workspace_root(&self) -> Result<std::path::PathBuf, crate::error::AppError> {
        let repo = gix::discover(".").map_err(|e| AppError::GitDiscoveryFailed {
            reason: e.to_string(),
        })?;

        repo.work_dir()
            .ok_or_else(|| AppError::GitDiscoveryFailed {
                reason: "Failed to get repository root".to_string(),
            })
            .map(|p| p.to_path_buf())
    }

    fn get_changed_files(
        &self,
        workspace_root: &std::path::Path,
    ) -> Result<Vec<std::path::PathBuf>, crate::error::AppError> {
        let repo = gix::discover(workspace_root).map_err(|e| AppError::GitDiscoveryFailed {
            reason: e.to_string(),
        })?;

        let mut changed_files = IndexSet::new();

        let changes = repo
            .status(gix::features::progress::Discard)
            .map_err(|e| AppError::GitOperationFailed {
                operation: "status".to_string(),
                reason: e.to_string(),
            })?
            .untracked_files(gix::status::UntrackedFiles::Files)
            .into_iter(None)
            .map_err(|e| AppError::GitOperationFailed {
                operation: "status iteration".to_string(),
                reason: e.to_string(),
            })?
            .map(|change_result| {
                change_result.map_err(|e| AppError::GitOperationFailed {
                    operation: "process change".to_string(),
                    reason: e.to_string(),
                })
            })
            .map(Result::unwrap)
            .filter(|change| match change {
                gix::status::Item::IndexWorktree(item) => item
                    .summary()
                    .map_or(true, |summary| summary != Summary::Removed),
                gix::status::Item::TreeIndex(_) => true,
            })
            .collect::<Vec<gix::status::Item>>();

        for change in changes {
            let path = change.location();
            let path_str = path.to_str().map_err(|_| AppError::GitOperationFailed {
                operation: "convert path".to_string(),
                reason: "Invalid UTF-8 in path".to_string(),
            })?;

            let full_path = workspace_root.join(path_str).canonicalize().map_err(|e| {
                AppError::GitOperationFailed {
                    operation: "canonicalize path".to_string(),
                    reason: e.to_string(),
                }
            })?;

            changed_files.insert(full_path);
        }

        Ok(changed_files.into_iter().collect())
    }
}
