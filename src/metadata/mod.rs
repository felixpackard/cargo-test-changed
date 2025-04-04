use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::Result;
use cargo_metadata::{CargoOpt, Metadata, MetadataCommand};
use indexmap::IndexSet;

use crate::{error::AppError, vcs::ChangedFile};

#[cfg(test)]
mod tests;

/// Represents a collection of crates in a workspace
#[derive(Debug)]
pub struct Crates(HashSet<CrateInfo>);

/// Represents a single crate in a workspace
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CrateInfo {
    pub name: String,
    pub path: PathBuf,
}

/// Get workspace metadata using cargo metadata
pub fn get_workspace_metadata(workspace_root: &Path) -> Result<Metadata, AppError> {
    let metadata = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .features(CargoOpt::AllFeatures)
        .no_deps()
        .exec()
        .map_err(|e| AppError::MetadataFailed {
            reason: e.to_string(),
        })?;

    Ok(metadata)
}

/// Get workspace crates using cargo metadata
pub fn get_workspace_crates(metadata: &Metadata) -> Result<Crates, AppError> {
    let mut crates = HashSet::new();

    for package in &metadata.packages {
        let manifest_dir =
            Path::new(&package.manifest_path)
                .parent()
                .ok_or_else(|| AppError::MetadataFailed {
                    reason: "Failed to get parent directory".to_string(),
                })?;

        crates.insert(CrateInfo {
            name: package.name.clone(),
            path: manifest_dir.to_path_buf(),
        });
    }

    Ok(Crates(crates))
}

/// Find the crate name for a given file path
fn find_crate_for_file<'a>(file_path: &Path, crates: &'a Crates) -> Option<&'a CrateInfo> {
    let mut best_match: Option<&CrateInfo> = None;
    let mut best_match_components = 0;

    for crate_info in &crates.0 {
        if file_path.starts_with(&crate_info.path) {
            let component_count = crate_info.path.components().count();
            if best_match.is_none() || component_count > best_match_components {
                best_match = Some(crate_info);
                best_match_components = component_count;
            }
        }
    }

    best_match
}

/// Find crates that have changed based on file paths
pub fn find_changed_crates<'a>(
    changed_files: &[ChangedFile],
    crates: &'a Crates,
) -> Result<IndexSet<&'a String>, AppError> {
    let mut changed_crates = IndexSet::new();

    for change in changed_files {
        if let Some(crate_info) = find_crate_for_file(&change.current_path, &crates) {
            changed_crates.insert(&crate_info.name);
        }
        if let Some(old_path) = &change.old_path {
            if let Some(crate_info) = find_crate_for_file(old_path, &crates) {
                changed_crates.insert(&crate_info.name);
            }
        }
    }

    Ok(changed_crates)
}

/// Find crates that depend on changed crates
pub fn find_dependent_crates<'a>(
    changed_crates: &IndexSet<&String>,
    metadata: &'a cargo_metadata::Metadata,
) -> Result<IndexSet<&'a String>> {
    let mut dependent_crates = IndexSet::new();

    // Find crates that depend on changed crates
    for package in &metadata.packages {
        for dep in &package.dependencies {
            if changed_crates.contains(&dep.name) {
                dependent_crates.insert(&package.name);
            }
        }
    }

    Ok(dependent_crates)
}

/// Verify that all specified crates exist in the workspace
pub fn verify_crates_exist(
    metadata: &cargo_metadata::Metadata,
    crates: &[String],
) -> Result<(), AppError> {
    for crate_name in crates {
        if !metadata.packages.iter().any(|p| p.name == *crate_name) {
            return Err(AppError::UnknownCrate {
                crate_name: crate_name.clone(),
            });
        }
    }

    Ok(())
}
