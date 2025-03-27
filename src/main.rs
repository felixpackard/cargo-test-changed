use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use cargo_metadata::{CargoOpt, MetadataCommand};
use clap::Parser;
use gix::bstr::ByteSlice;
use indexmap::IndexSet;
use itertools::Itertools;

/// Configuration for the changed tests subcommand
#[derive(Debug, Parser)]
#[command(
    name = "cargo-changed-tests",
    about = "A Cargo subcommand to run tests for changed crates and their dependents"
)]
struct Opt {
    /// Specify a custom test runner (default is cargo test)
    #[arg(long, short)]
    test_runner: Option<String>,

    /// Skip testing dependent crates
    #[arg(long)]
    skip_dependents: bool,

    /// Skip running tests, only print the crates that would be tested
    #[arg(long)]
    dry_run: bool,
}

/// Main entry point for the cargo subcommand
fn main() -> Result<()> {
    // Parse command line options
    let opt = Opt::parse();

    // Get the current workspace root
    let workspace_root = get_workspace_root()?;

    // Retrieve workspace metadata
    let metadata = get_workspace_metadata(&workspace_root)?;

    // Get changed files from Git
    let changed_files = get_changed_files(&workspace_root)?;

    // Find changed crates
    let changed_crates = find_changed_crates(&metadata, &changed_files)?;

    // Determine crates to test (including dependent crates)
    let crates_to_test = determine_crates_to_test(&opt, &metadata, &changed_crates)?;

    // Run tests for identified crates
    run_tests(&opt, &workspace_root, &crates_to_test)?;

    Ok(())
}

/// Retrieve the workspace root directory
fn get_workspace_root() -> Result<PathBuf> {
    let repo = gix::discover(".")?;

    repo.work_dir()
        .context("Failed to get repository root")
        .map(|p| p.to_path_buf())
}

/// Get workspace metadata using cargo metadata
fn get_workspace_metadata(workspace_root: &Path) -> Result<cargo_metadata::Metadata> {
    MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .features(CargoOpt::AllFeatures)
        .no_deps()
        .exec()
        .context("Failed to retrieve cargo metadata")
}

/// Get list of changed files from Git repository
fn get_changed_files(workspace_root: &Path) -> Result<Vec<PathBuf>> {
    let repo = gix::discover(workspace_root)?;

    let mut changed_files = IndexSet::new();

    let changes = repo
        .status(gix::features::progress::Discard)?
        .into_iter(vec![])?;

    for change in changes {
        let change = change?;
        let path = change.location();
        changed_files.insert(workspace_root.join(path.to_str()?).canonicalize()?);
    }

    Ok(changed_files.into_iter().collect())
}

/// Find crates that have changed based on file paths
fn find_changed_crates(
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
fn determine_crates_to_test(
    opt: &Opt,
    metadata: &cargo_metadata::Metadata,
    changed_crates: &IndexSet<String>,
) -> Result<IndexSet<String>> {
    let mut crates_to_test = changed_crates.clone();

    // Find crates that depend on changed crates
    if !opt.skip_dependents {
        for package in &metadata.packages {
            for dep in &package.dependencies {
                if changed_crates.contains(&dep.name) {
                    crates_to_test.insert(package.name.clone());
                }
            }
        }
    }

    Ok(crates_to_test)
}

/// Run tests for specified crates
fn run_tests(opt: &Opt, workspace_root: &Path, crates_to_test: &IndexSet<String>) -> Result<()> {
    if crates_to_test.is_empty() {
        println!("No crates to test.");
        return Ok(());
    }

    println!(
        "Queueing tests for {} crate{}: {}\n",
        crates_to_test.len(),
        if crates_to_test.len() > 1 { "s" } else { "" },
        crates_to_test.iter().join(", ")
    );

    if opt.dry_run {
        println!("Dry run mode enabled. Skipping actual tests.");
        return Ok(());
    }

    // Default to cargo test if no custom runner specified
    let test_runner = opt.test_runner.as_deref().unwrap_or("cargo");

    for crate_name in crates_to_test {
        println!("Running tests for crate: {}", crate_name);

        let mut cmd = Command::new(test_runner);

        if test_runner == "cargo" {
            cmd.arg("test").arg("-p").arg(crate_name);
        }

        let status = cmd
            .current_dir(workspace_root)
            .status()
            .context("Failed to run tests")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Tests failed for crate: {}", crate_name));
        }
    }

    Ok(())
}
