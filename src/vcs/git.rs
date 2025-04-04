use gix::{
    bstr::{BString, ByteSlice},
    objs::tree::EntryKind,
    Commit, Repository, Tree,
};
use std::path::{Path, PathBuf};

use crate::error::AppError;

use super::{ChangeType, ChangedFile, FileType, Vcs};

struct GitPathInfo {
    current_path: Option<BString>,
    old_path: Option<BString>,
}

struct GitChangeInfo {
    path_info: GitPathInfo,
    file_type: Option<FileType>,
    change_type: ChangeType,
}

pub struct GitVcs;

impl Vcs for GitVcs {
    fn get_workspace_root(&self, path: &Path) -> Result<PathBuf, AppError> {
        let repo = discover_repo(path)?;

        repo.work_dir()
            .ok_or_else(|| AppError::GitDiscoveryFailed {
                reason: "Failed to get repository root".to_string(),
            })
            .and_then(|p| {
                p.canonicalize().map_err(|e| AppError::GitDiscoveryFailed {
                    reason: e.to_string(),
                })
            })
    }

    fn get_uncommitted_changes(&self, workspace_root: &Path) -> Result<Vec<ChangedFile>, AppError> {
        let repo = discover_repo(workspace_root)?;
        let changes = collect_status_changes(&repo)?;

        // Convert git changes to ChangedFile objects
        let changed_files = changes
            .into_iter()
            .filter_map(|change| {
                // Try to extract git change info
                let git_info = GitChangeInfo::try_from(&change).ok()?;

                // Convert to ChangedFile
                convert_to_changed_file(git_info, workspace_root).ok()
            })
            .collect();

        Ok(changed_files)
    }

    fn get_changes_between(
        &self,
        workspace_root: &Path,
        from_ref: &str,
        to_ref: Option<&str>,
    ) -> Result<Vec<ChangedFile>, AppError> {
        let repo = discover_repo(workspace_root)?;
        let from_commit = resolve_commit(&repo, from_ref)?;
        let to_commit = resolve_commit(&repo, to_ref.unwrap_or("HEAD"))?;

        // Get trees from both commits
        let from_tree = get_commit_tree(&from_commit)?;
        let to_tree = get_commit_tree(&to_commit)?;

        let diff = repo
            .diff_tree_to_tree(Some(&from_tree), Some(&to_tree), None)
            .map_err(|e| AppError::GitOperationFailed {
                operation: "diff between commits".to_string(),
                reason: e.to_string(),
            })?;

        // Process diff changes
        let changed_files: Vec<ChangedFile> = diff
            .into_iter()
            .filter_map(|change| {
                // Convert diff change to GitChangeInfo
                let git_info = GitChangeInfo::try_from_diff_change(&change, workspace_root).ok()?;

                // Convert to ChangedFile
                convert_to_changed_file(git_info, workspace_root).ok()
            })
            .filter(|c| matches!(c.file_type, FileType::File | FileType::Symlink))
            .collect();

        Ok(changed_files)
    }
}

fn discover_repo(workspace_root: &Path) -> Result<Repository, AppError> {
    gix::discover(workspace_root).map_err(|e| AppError::GitDiscoveryFailed {
        reason: e.to_string(),
    })
}

fn collect_status_changes(repo: &Repository) -> Result<Vec<gix::status::Item>, AppError> {
    repo.status(gix::features::progress::Discard)
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
        .collect::<Result<Vec<gix::status::Item>, _>>()
        .map_err(|e| AppError::GitOperationFailed {
            operation: "process change".to_string(),
            reason: e.to_string(),
        })
}

fn resolve_commit<'a>(repo: &'a Repository, reference: &str) -> Result<Commit<'a>, AppError> {
    repo.rev_parse_single(reference)
        .map_err(|e| AppError::GitOperationFailed {
            operation: format!("resolve reference '{}'", reference),
            reason: e.to_string(),
        })?
        .object()
        .map_err(|e| AppError::GitOperationFailed {
            operation: format!("convert '{}' to commit", reference),
            reason: e.to_string(),
        })?
        .try_into_commit()
        .map_err(|e| AppError::GitOperationFailed {
            operation: format!("convert '{}' to commit", reference),
            reason: e.to_string(),
        })
}

fn get_commit_tree<'a>(commit: &'a Commit<'a>) -> Result<Tree<'a>, AppError> {
    commit.tree().map_err(|e| AppError::GitOperationFailed {
        operation: format!("get tree for commit {}", commit.id()),
        reason: e.to_string(),
    })
}

fn convert_to_changed_file(
    git_info: GitChangeInfo,
    workspace_root: &Path,
) -> Result<ChangedFile, AppError> {
    let current_path = match git_info
        .path_info
        .current_path
        .map(|p| convert_path(p, workspace_root))
        .transpose()?
    {
        Some(p) => p,
        None => {
            return Err(AppError::GitOperationFailed {
                operation: "convert path".to_string(),
                reason: "Missing required path".to_string(),
            })
        }
    };

    let old_path = git_info
        .path_info
        .old_path
        .map(|p| convert_path(p, workspace_root))
        .transpose()?;

    Ok(ChangedFile {
        current_path,
        old_path,
        file_type: git_info.file_type.unwrap_or(FileType::Other),
        change_type: git_info.change_type,
    })
}

fn convert_path(path: BString, workspace_root: &Path) -> Result<PathBuf, AppError> {
    if let Ok(path_str) = path.to_path() {
        Ok(workspace_root.join(path_str))
    } else {
        Err(AppError::GitOperationFailed {
            operation: "convert path".to_string(),
            reason: "Invalid UTF-8 in path".to_string(),
        })
    }
}

impl TryFrom<&gix::status::Item> for GitChangeInfo {
    type Error = AppError;

    fn try_from(item: &gix::status::Item) -> Result<Self, Self::Error> {
        let path_info = GitPathInfo::try_from(item)?;
        let file_type = FileType::from_git_status(item);
        let change_type = ChangeType::from(item);

        Ok(GitChangeInfo {
            path_info,
            file_type,
            change_type,
        })
    }
}

impl GitChangeInfo {
    fn try_from_diff_change(
        change: &gix::diff::tree_with_rewrites::Change,
        _workspace_root: &Path,
    ) -> Result<Self, AppError> {
        let (current_path, old_path, file_type, change_type) = match change {
            gix::diff::tree_with_rewrites::Change::Addition {
                location,
                entry_mode,
                ..
            } => (
                Some(location.clone()),
                None,
                Some(FileType::from_entry_kind(entry_mode.kind())),
                ChangeType::Added,
            ),
            gix::diff::tree_with_rewrites::Change::Modification {
                location,
                entry_mode,
                ..
            } => (
                Some(location.clone()),
                None,
                Some(FileType::from_entry_kind(entry_mode.kind())),
                ChangeType::Modified,
            ),
            gix::diff::tree_with_rewrites::Change::Rewrite {
                source_location,
                location,
                entry_mode,
                ..
            } => (
                Some(location.clone()),
                Some(source_location.clone()),
                Some(FileType::from_entry_kind(entry_mode.kind())),
                ChangeType::Modified,
            ),
            gix::diff::tree_with_rewrites::Change::Deletion {
                location,
                entry_mode,
                ..
            } => (
                Some(location.clone()),
                None,
                Some(FileType::from_entry_kind(entry_mode.kind())),
                ChangeType::Removed,
            ),
        };

        Ok(GitChangeInfo {
            path_info: GitPathInfo {
                current_path,
                old_path,
            },
            file_type,
            change_type,
        })
    }
}

impl TryFrom<&gix::status::Item> for GitPathInfo {
    type Error = AppError;

    fn try_from(item: &gix::status::Item) -> Result<Self, Self::Error> {
        match item {
            gix::status::Item::IndexWorktree(item) => GitPathInfo::try_from(item),
            gix::status::Item::TreeIndex(change_ref) => GitPathInfo::try_from(change_ref),
        }
    }
}

impl TryFrom<&gix::status::index_worktree::Item> for GitPathInfo {
    type Error = AppError;

    fn try_from(item: &gix::status::index_worktree::Item) -> Result<Self, Self::Error> {
        match item {
            gix::status::index_worktree::Item::Modification { rela_path, .. } => Ok(GitPathInfo {
                current_path: Some(rela_path.clone()),
                old_path: None,
            }),
            gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                match entry.status {
                    gix::dir::entry::Status::Ignored(_) => Ok(GitPathInfo {
                        current_path: None,
                        old_path: None,
                    }),
                    _ => Ok(GitPathInfo {
                        current_path: Some(entry.rela_path.clone()),
                        old_path: None,
                    }),
                }
            }
            gix::status::index_worktree::Item::Rewrite {
                source,
                dirwalk_entry,
                copy,
                ..
            } => {
                let old_path =
                    if *copy {
                        None
                    } else {
                        match source {
                            gix::status::index_worktree::RewriteSource::RewriteFromIndex {
                                source_rela_path, ..
                            } => Some(source_rela_path.clone()),
                            gix::status::index_worktree::RewriteSource::CopyFromDirectoryEntry {
                                source_dirwalk_entry, ..
                            } => Some(source_dirwalk_entry.rela_path.clone()),
                        }
                    };

                Ok(GitPathInfo {
                    current_path: Some(dirwalk_entry.rela_path.clone()),
                    old_path,
                })
            }
        }
    }
}

impl<'l, 'r> TryFrom<&gix::diff::index::ChangeRef<'l, 'r>> for GitPathInfo {
    type Error = AppError;

    fn try_from(change_ref: &gix::diff::index::ChangeRef) -> Result<Self, Self::Error> {
        match change_ref {
            gix::diff::index::ChangeRef::Addition { location, .. } => Ok(GitPathInfo {
                current_path: Some(location.clone().into_owned()),
                old_path: None,
            }),
            gix::diff::index::ChangeRef::Deletion { location, .. } => Ok(GitPathInfo {
                current_path: Some(location.clone().into_owned()),
                old_path: None,
            }),
            gix::diff::index::ChangeRef::Modification { location, .. } => Ok(GitPathInfo {
                current_path: Some(location.clone().into_owned()),
                old_path: None,
            }),
            gix::diff::index::ChangeRef::Rewrite {
                source_location,
                location,
                copy,
                ..
            } => {
                let old_path = if *copy {
                    None
                } else {
                    Some(source_location.clone().into_owned())
                };

                Ok(GitPathInfo {
                    current_path: Some(location.clone().into_owned()),
                    old_path,
                })
            }
        }
    }
}

impl From<&gix::status::Item> for ChangeType {
    fn from(item: &gix::status::Item) -> Self {
        match item {
            gix::status::Item::IndexWorktree(item) => ChangeType::from(item),
            gix::status::Item::TreeIndex(change_ref) => ChangeType::from(change_ref),
        }
    }
}

impl From<&gix::status::index_worktree::Item> for ChangeType {
    fn from(item: &gix::status::index_worktree::Item) -> Self {
        if let Some(summary) = item.summary() {
            match summary {
                gix::status::index_worktree::iter::Summary::Added
                | gix::status::index_worktree::iter::Summary::IntentToAdd => ChangeType::Added,
                gix::status::index_worktree::iter::Summary::Modified
                | gix::status::index_worktree::iter::Summary::TypeChange
                | gix::status::index_worktree::iter::Summary::Renamed
                | gix::status::index_worktree::iter::Summary::Copied
                | gix::status::index_worktree::iter::Summary::Conflict => ChangeType::Modified,
                gix::status::index_worktree::iter::Summary::Removed => ChangeType::Removed,
            }
        } else {
            ChangeType::Modified
        }
    }
}

impl<'l, 'r> From<&gix::diff::index::ChangeRef<'l, 'r>> for ChangeType {
    fn from(change_ref: &gix::diff::index::ChangeRef) -> Self {
        match change_ref {
            gix::diff::index::ChangeRef::Addition { .. } => ChangeType::Added,
            gix::diff::index::ChangeRef::Modification { .. }
            | gix::diff::index::ChangeRef::Rewrite { .. } => ChangeType::Modified,
            gix::diff::index::ChangeRef::Deletion { .. } => ChangeType::Removed,
        }
    }
}

impl FileType {
    fn from_git_status(item: &gix::status::Item) -> Option<Self> {
        match item {
            gix::status::Item::IndexWorktree(item) => match item {
                gix::status::index_worktree::Item::Modification { entry, .. } => {
                    Some(entry.mode.into())
                }
                gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                    entry.disk_kind.map(Into::into)
                }
                gix::status::index_worktree::Item::Rewrite { dirwalk_entry, .. } => {
                    dirwalk_entry.disk_kind.map(Into::into)
                }
            },
            gix::status::Item::TreeIndex(change_ref) => match change_ref {
                gix::diff::index::ChangeRef::Addition { entry_mode, .. } => {
                    Some(entry_mode.clone().into())
                }
                gix::diff::index::ChangeRef::Deletion { entry_mode, .. } => {
                    Some(entry_mode.clone().into())
                }
                gix::diff::index::ChangeRef::Modification { entry_mode, .. } => {
                    Some(entry_mode.clone().into())
                }
                gix::diff::index::ChangeRef::Rewrite { entry_mode, .. } => {
                    Some(entry_mode.clone().into())
                }
            },
        }
    }

    fn from_entry_kind(kind: EntryKind) -> Self {
        match kind {
            EntryKind::Tree => Self::Directory,
            EntryKind::Blob | EntryKind::BlobExecutable => Self::File,
            EntryKind::Link => Self::Symlink,
            EntryKind::Commit => Self::Other,
        }
    }
}

impl From<gix::index::entry::Mode> for FileType {
    fn from(value: gix::index::entry::Mode) -> Self {
        match value {
            gix::index::entry::Mode::FILE | gix::index::entry::Mode::FILE_EXECUTABLE => Self::File,
            gix::index::entry::Mode::DIR => Self::Directory,
            gix::index::entry::Mode::SYMLINK => Self::Symlink,
            _ => Self::Other,
        }
    }
}

impl From<gix::dir::entry::Kind> for FileType {
    fn from(value: gix::dir::entry::Kind) -> Self {
        match value {
            gix::dir::entry::Kind::File => Self::File,
            gix::dir::entry::Kind::Directory => Self::Directory,
            gix::dir::entry::Kind::Symlink => Self::Symlink,
            _ => Self::Other,
        }
    }
}
