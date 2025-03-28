use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use cargo_metadata::{CargoOpt, MetadataCommand};
use clap::{Parser, ValueEnum};
use gix::bstr::ByteSlice;
use indexmap::IndexSet;
use itertools::Itertools;

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
            TestRunner::Nextest => "Install with 'cargo install cargo-nextest'.".to_string(),
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

/// Main entry point for the cargo subcommand
fn main() -> Result<()> {
    // Parse command line options
    let CargoCli::TestChanged(args) = CargoCli::parse();

    // Get the current workspace root
    let workspace_root = get_workspace_root()?;

    // Retrieve workspace metadata
    let metadata = get_workspace_metadata(&workspace_root)?;

    // Get changed files from Git
    let changed_files = get_changed_files(&workspace_root)?;

    // Find changed crates
    let changed_crates = find_changed_crates(&metadata, &changed_files)?;

    // Determine crates to test (including dependent crates)
    let crates_to_test = determine_crates_to_test(&args, &metadata, &changed_crates)?;

    // Run tests for identified crates
    run_tests(&args, &workspace_root, &crates_to_test)?;

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
    args: &TestChangedArgs,
    metadata: &cargo_metadata::Metadata,
    changed_crates: &IndexSet<String>,
) -> Result<IndexSet<String>> {
    let mut crates_to_test = changed_crates.clone();

    // Find crates that depend on changed crates
    if !args.skip_dependents {
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
fn run_tests(
    args: &TestChangedArgs,
    workspace_root: &Path,
    crates_to_test: &IndexSet<String>,
) -> Result<()> {
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

    if args.dry_run {
        println!("Dry run mode enabled. Skipping actual tests.");
        return Ok(());
    }

    if !args.test_runner.is_installed() {
        return Err(anyhow::anyhow!(
            "Test runner is not installed. {}",
            args.test_runner.installation_instructions()
        ));
    }

    for crate_name in crates_to_test {
        println!("Running tests for crate: {}", crate_name);

        let mut cmd = args.test_runner.command(crate_name);

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
