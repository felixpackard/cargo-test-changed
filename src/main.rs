use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

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
                cmd.args(["test", "-p", crate_name]);
                cmd
            }
            TestRunner::Nextest => {
                let mut cmd = Command::new("cargo");
                cmd.args(["nextest", "run", "--no-tests", "pass", "-p", crate_name]);
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
                "this shouldn't be possible if you're invoking this binary via cargo.".to_string()
            }
            TestRunner::Nextest => format!(
                "to install nextest, run '{}'",
                "cargo install cargo-nextest".bold().yellow()
            )
            .to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    crate_name: String,
    stdout: String,
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
    /// Specify a custom test runner
    #[arg(long, short, default_value = "cargo")]
    test_runner: TestRunner,

    /// Skip dependent crates, only test crates with changes
    #[arg(long, short)]
    skip_dependents: bool,

    /// Skip running tests, only print the crates that would be tested
    #[arg(long, short)]
    dry_run: bool,

    /// Display full output while running tests
    #[arg(long, short)]
    verbose: bool,

    /// Run tests for all crates regardless of failure
    #[arg(long, short)]
    no_fail_fast: bool,

    /// Additional arguments to pass to the test runner
    #[arg(last = true)]
    test_runner_args: Vec<String>,
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
        .into_iter([])
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

    println!(
        "discovered {} changed {}; {}{} dependent {}\n",
        changed_crates.len(),
        format::pluralize!("crate", changed_crates.len()),
        if args.skip_dependents {
            "skipping "
        } else {
            ""
        },
        dependent_crates.len(),
        format::pluralize!("crate", dependent_crates.len())
    );

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

    let crates_to_test: Vec<&String> = if args.skip_dependents {
        changed_crates.iter().collect()
    } else {
        changed_crates
            .iter()
            .chain(dependent_crates.iter())
            .collect()
    };

    let mut passed = 0;
    let mut failures = Vec::new();

    let start = Instant::now();

    for crate_name in crates_to_test {
        print!("test crate {}", crate_name);

        if args.verbose {
            println!();
        } else {
            print!(" ... ");
        }

        std::io::stdout().flush().unwrap();

        let mut cmd = args.test_runner.command(crate_name);
        cmd.args(args.test_runner_args.iter());

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child =
            cmd.current_dir(workspace_root)
                .spawn()
                .map_err(|e| AppError::CommandFailed {
                    command: format!("{:?}", cmd),
                    reason: e.to_string(),
                })?;

        let mut output_capture = Vec::new();

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        if let (Some(stdout), Some(stderr)) = (stdout, stderr) {
            let mut merged_output = std::io::BufReader::new(stdout)
                .bytes()
                .map(|r| (r, false))
                .chain(std::io::BufReader::new(stderr).bytes().map(|r| (r, true)));

            if args.verbose {
                while let Some((byte_result, _is_stderr)) = merged_output.next() {
                    match byte_result {
                        Ok(byte) => {
                            std::io::stdout().write_all(&[byte]).map_err(|e| {
                                AppError::CommandFailed {
                                    command: format!("{:?}", cmd),
                                    reason: format!("failed to write to stdout: {}", e),
                                }
                            })?;
                            std::io::stdout().flush().unwrap();
                            output_capture.push(byte);
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::BrokenPipe {
                                return Err(AppError::CommandFailed {
                                    command: format!("{:?}", cmd),
                                    reason: format!("failed to read output: {}", e),
                                });
                            }
                            break;
                        }
                    }
                }
            } else {
                while let Some((byte_result, _is_stderr)) = merged_output.next() {
                    match byte_result {
                        Ok(byte) => {
                            output_capture.push(byte);
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::BrokenPipe {
                                return Err(AppError::CommandFailed {
                                    command: format!("{:?}", cmd),
                                    reason: format!("failed to read output: {}", e),
                                });
                            }
                            break;
                        }
                    }
                }
            }
        }

        let status = child.wait().map_err(|e| AppError::CommandFailed {
            command: format!("{:?}", cmd),
            reason: e.to_string(),
        })?;

        if status.success() {
            passed += 1;
            if args.verbose {
                println!();
            } else {
                println!("{}", "ok".bold().green());
            }
        } else {
            if args.verbose {
                println!();
            } else {
                println!("{}", "FAILED".bold().red());
            }

            failures.push(TestFailure {
                crate_name: crate_name.clone(),
                stdout: String::from_utf8_lossy(&output_capture).into_owned(),
            });

            if !args.no_fail_fast {
                if args.verbose {
                    println!();
                }
                break;
            }
        }
    }

    if !args.verbose {
        println!();
    }

    let end = Instant::now();

    if !failures.is_empty() {
        if !args.verbose {
            println!("\ncrate failures:\n");
            for failure in failures.iter() {
                println!(
                    "---- {} output ----\n{}\n",
                    failure.crate_name, failure.stdout
                );
            }
        }
        println!("crate failures:");
        for failure in failures.iter() {
            println!("    {}", failure.crate_name);
        }
        println!();
    }

    println!(
        "test result: {}. {} passed; {} failed; finished in {:.2}s\n",
        if failures.is_empty() {
            "ok".bold().green()
        } else {
            "FAILED".bold().red()
        },
        passed,
        failures.len(),
        end.duration_since(start).as_secs_f64()
    );

    if failures.is_empty() {
        Ok(())
    } else {
        Err(AppError::TestsFailed { failures })
    }
}
