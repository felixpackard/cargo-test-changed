use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cargo_metadata::{CargoOpt, MetadataCommand};
use indexmap::IndexSet;

use crate::error::AppError;

/// Get workspace metadata using cargo metadata
pub fn get_workspace_metadata(workspace_root: &Path) -> Result<cargo_metadata::Metadata, AppError> {
    MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .features(CargoOpt::AllFeatures)
        .no_deps()
        .exec()
        .map_err(|e| AppError::MetadataFailed {
            reason: e.to_string(),
        })
}

/// Find crates that have changed based on file paths
pub fn find_changed_crates(
    metadata: &cargo_metadata::Metadata,
    changed_files: &[PathBuf],
) -> Result<IndexSet<String>> {
    let mut changed_crates = IndexSet::new();

    for package in &metadata.packages {
        let pkg_path = package
            .manifest_path
            .parent()
            .context("Failed to get package parent path")?;

        if changed_files
            .iter()
            .any(|file| file.starts_with(pkg_path) && !file.ends_with("Cargo.toml"))
        {
            changed_crates.insert(package.name.clone());
        }
    }

    Ok(changed_crates)
}

/// Determine which crates need testing (including dependencies)
pub fn find_dependent_crates(
    metadata: &cargo_metadata::Metadata,
    changed_crates: &IndexSet<String>,
) -> Result<IndexSet<String>> {
    let mut dependent_crates = IndexSet::new();

    // Find crates that depend on changed crates
    for package in &metadata.packages {
        for dep in &package.dependencies {
            if changed_crates.contains(&dep.name) {
                dependent_crates.insert(package.name.clone());
            }
        }
    }

    Ok(dependent_crates)
}
