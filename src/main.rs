use std::io::{stderr, stdout};

use anyhow::Result;
use test_runner::TestRunnerType;

use clap::Parser;
use error::AppError;

mod error;
mod git;
mod metadata;
mod reporting;
mod test_runner;
mod testing;

use reporting::Reporter;

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
    test_runner: TestRunnerType,

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

    /// Output in JSON format for machine consumption
    #[arg(long, short)]
    json: bool,

    /// Additional arguments to pass to the test runner
    #[arg(last = true)]
    test_runner_args: Vec<String>,
}

fn main() {
    match run() {
        Ok(_) => (),
        Err(err) => {
            let mut reporter = reporting::console::ConsoleReporter::new(stderr(), false);
            err.report(&mut reporter);
            std::process::exit(err.exit_code());
        }
    }
}

fn run() -> Result<(), AppError> {
    let CargoCli::TestChanged(args) = CargoCli::parse();

    // Get workspace and repository information
    let workspace_root = git::get_workspace_root()?;
    let changed_files = git::get_changed_files(&workspace_root)?;
    let metadata = metadata::get_workspace_metadata(&workspace_root)?;

    // Identify which crates need testing
    let changed_crates = metadata::find_changed_crates(&metadata, &changed_files)?;
    let dependent_crates = metadata::find_dependent_crates(&metadata, &changed_crates)?;

    // Get the appropriate test runner
    let runner = args.test_runner.create();

    // Create a reporter
    let mut reporter = if args.json {
        Box::new(reporting::json::JsonReporter::new(stdout())) as Box<dyn Reporter>
    } else {
        Box::new(reporting::console::ConsoleReporter::new(
            stdout(),
            args.verbose,
        )) as Box<dyn Reporter>
    };

    // Execute the tests
    testing::run_tests(
        &workspace_root,
        runner.as_ref(),
        &changed_crates,
        &dependent_crates,
        args.skip_dependents,
        !args.no_fail_fast,
        args.verbose,
        args.test_runner_args,
        args.dry_run,
        reporter.as_mut(),
    )
}
