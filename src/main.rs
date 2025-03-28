use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_metadata::{CargoOpt, MetadataCommand};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use error::AppError;
use gix::bstr::ByteSlice;
use indexmap::IndexSet;

pub mod error;
mod format;

#[derive(Debug, Clone, ValueEnum)]
enum TestRunner {
    Cargo,
    Nextest,
}

impl TestRunner {
    fn command(&self, crate_name: &str) -> Command {
        match self {
            TestRunner::Cargo => {
                let mut cmd = Command::new("cargo");
                cmd.args(vec!["test", "-p", crate_name]);
                cmd
            }
            TestRunner::Nextest => {
                let mut cmd = Command::new("cargo");
                cmd.args(vec!["nextest", "run", "-p", crate_name]);
                cmd
            }
        }
    }

    fn is_installed(&self) -> bool {
        match self {
            TestRunner::Cargo => true,
            TestRunner::Nextest => std::process::Command::new("cargo")
                .args(["nextest", "--version"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false),
        }
    }

    fn installation_instructions(&self) -> String {
        match self {
            TestRunner::Cargo => {
                "This shouldn't be possible if you're invoking this binary via cargo.".to_string()
            }
            TestRunner::Nextest => format!(
                "to install nextest, run '{}'",
                "cargo install cargo-nextest".bold().yellow()
            )
            .to_string(),
        }
    }
}

/// Configuration for the changed tests subcommand
#[derive(Parser)]
#[command(
    name = "cargo",
    bin_name = "cargo",
    styles = clap_cargo::style::CLAP_STYLING,
)]
enum CargoCli {
    TestChanged(TestChangedArgs),
}

#[derive(clap::Args)]
#[command(
    version,
    about = "Run tests only for crates that have been modified in the current workspace"
)]
struct TestChangedArgs {
    /// Specify a custom test runner (default is cargo test)
    #[arg(long, short, default_value = "cargo")]
    test_runner: TestRunner,

    /// Skip testing dependent crates
    #[arg(long, short)]
    skip_dependents: bool,

    /// Skip running tests, only print the crates that would be tested
    #[arg(long, short)]
    dry_run: bool,
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => {
            err.report();
            std::process::exit(err.exit_code());
        }
    }
}

fn run() -> Result<(), AppError> {
    let CargoCli::TestChanged(args) = CargoCli::parse();

    let workspace_root = get_workspace_root()?;
    let metadata = get_workspace_metadata(&workspace_root)?;
    let changed_files = get_changed_files(&workspace_root)?;
    let changed_crates = find_changed_crates(&metadata, &changed_files)?;
    let dependent_crates = find_dependent_crates(&metadata, &changed_crates)?;

    run_tests(&args, &workspace_root, &changed_crates, &dependent_crates)
}

/// Retrieve the workspace root directory
fn get_workspace_root() -> Result<PathBuf, AppError> {
    let repo = gix::discover(".").map_err(|e| AppError::GitDiscoveryFailed {
        reason: e.to_string(),
    })?;

    repo.work_dir()
        .ok_or_else(|| AppError::GitDiscoveryFailed {
            reason: "Failed to get repository root".to_string(),
        })
        .map(|p| p.to_path_buf())
}

/// Get workspace metadata using cargo metadata
fn get_workspace_metadata(workspace_root: &Path) -> Result<cargo_metadata::Metadata, AppError> {
    MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .features(CargoOpt::AllFeatures)
        .no_deps()
        .exec()
        .map_err(|e| AppError::MetadataFailed {
            reason: e.to_string(),
        })
}

/// Get list of changed files from Git repository
fn get_changed_files(workspace_root: &Path) -> Result<Vec<PathBuf>, AppError> {
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
        .into_iter(vec![])
        .map_err(|e| AppError::GitOperationFailed {
            operation: "status iteration".to_string(),
            reason: e.to_string(),
        })?;

    for change in changes {
        let change = change.map_err(|e| AppError::GitOperationFailed {
            operation: "process change".to_string(),
            reason: e.to_string(),
        })?;

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
fn find_dependent_crates(
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

/// Run tests for specified crates
fn run_tests(
    args: &TestChangedArgs,
    workspace_root: &Path,
    changed_crates: &IndexSet<String>,
    dependent_crates: &IndexSet<String>,
) -> Result<(), AppError> {
    if changed_crates.is_empty() {
        println!("no crates to test");
        return Ok(());
    }

    print!(
        "discovered {} changed {}",
        changed_crates.len(),
        format::pluralize!("crate", changed_crates.len())
    );
    if !dependent_crates.is_empty() {
        print!(
            "; {} dependent {}",
            dependent_crates.len(),
            format::pluralize!("crate", dependent_crates.len())
        );
    }
    println!("\n");

    if args.dry_run {
        format::note!("dry run mode enabled, skipping actual tests");
        return Ok(());
    }

    if !args.test_runner.is_installed() {
        return Err(AppError::TestRunnerNotInstalled {
            runner_name: format!("{:?}", args.test_runner),
            installation_tip: args.test_runner.installation_instructions(),
        });
    }

    for crate_name in changed_crates.iter().chain(dependent_crates.iter()) {
        println!("running tests for crate: {}", crate_name);

        let mut cmd = args.test_runner.command(crate_name);
        let cmd_string = format!("{:?}", cmd);

        let status =
            cmd.current_dir(workspace_root)
                .status()
                .map_err(|e| AppError::CommandFailed {
                    command: cmd_string.clone(),
                    reason: e.to_string(),
                })?;

        if !status.success() {
            return Err(AppError::TestsFailed {
                crate_name: crate_name.clone(),
            });
        }
    }

    Ok(())
}
