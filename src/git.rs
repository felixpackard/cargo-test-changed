use std::path::{Path, PathBuf};

use gix::bstr::ByteSlice;
use indexmap::IndexSet;

use crate::error::AppError;

/// Retrieve the workspace root directory
pub fn get_workspace_root() -> Result<PathBuf, AppError> {
    let repo = gix::discover(".").map_err(|e| AppError::GitDiscoveryFailed {
        reason: e.to_string(),
    })?;

    repo.work_dir()
        .ok_or_else(|| AppError::GitDiscoveryFailed {
            reason: "Failed to get repository root".to_string(),
        })
        .map(|p| p.to_path_buf())
}

/// Get list of changed files from Git repository
pub fn get_changed_files(workspace_root: &Path) -> Result<Vec<PathBuf>, AppError> {
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
        .into_iter([])
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
        .collect::<Result<Vec<_>, AppError>>()?
        .into_iter()
        .filter(|change| {
            matches!(change,
                gix::status::Item::IndexWorktree(item) if
                item.summary().map_or(true, |summary|
                    summary != gix::status::index_worktree::iter::Summary::Removed)
            )
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
