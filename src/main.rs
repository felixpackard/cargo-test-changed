use std::io::{stderr, stdout};

use anyhow::Result;
use indexmap::IndexSet;
use test_runner::TestRunnerType;

use clap::Parser;
use error::AppError;

mod error;
mod metadata;
mod reporting;
mod test_runner;
mod testing;
mod vcs;

use reporting::Reporter;
use testing::plan::{DiscoveredTestCrate, DiscoveryType, ManualTestCrate, TestCrates, TestPlan};
use vcs::VcsType;

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

    /// Specify a set of crates to run tests for, typically for re-running failed tests
    #[arg(long, short, value_delimiter = ',')]
    crates: Vec<String>,

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
    let vcs = VcsType::Git.create();
    let workspace_root = vcs.get_workspace_root()?;
    let changed_files = vcs.get_changed_files(&workspace_root)?;
    let metadata = metadata::get_workspace_metadata(&workspace_root)?;

    // Identify which crates need testing
    let crates = if args.crates.is_empty() {
        let mut crates = IndexSet::new();
        let changed_crates = metadata::find_changed_crates(&metadata, &changed_files)?;

        crates.extend(
            changed_crates
                .iter()
                .map(|name| DiscoveredTestCrate {
                    name: name.clone(),
                    discovery_type: DiscoveryType::Modified,
                })
                .collect::<Vec<_>>(),
        );

        crates.extend(
            metadata::find_dependent_crates(&metadata, &changed_crates)?
                .into_iter()
                .map(|name| DiscoveredTestCrate {
                    name,
                    discovery_type: DiscoveryType::Dependent,
                })
                .collect::<Vec<_>>(),
        );

        TestCrates::Discovered(crates)
    } else {
        metadata::verify_crates_exist(&metadata, args.crates.as_slice())?;
        TestCrates::Manual(IndexSet::from_iter(
            args.crates.into_iter().map(|name| ManualTestCrate { name }),
        ))
    };

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
    let test_plan = TestPlan {
        workspace_root,
        crates,
        skip_dependents: args.skip_dependents,
        fail_fast: !args.no_fail_fast,
        verbose: args.verbose,
        test_runner_args: args.test_runner_args,
    };

    testing::run_tests(test_plan, runner.as_ref(), args.dry_run, reporter.as_mut())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_run_tests() {
        assert_eq!(1 + 1, 3);
    }
}
