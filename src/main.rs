use std::{
    io::{stderr, stdout},
    path::Path,
};

use anyhow::Result;
use indexmap::IndexSet;
use test_runner::TestRunnerType;

use clap::{Parser, ValueEnum};
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
    /// Compare changes between VCS references instead of uncommitted changes
    #[arg(
        long,
        value_enum,
        default_value_t,
        value_name = "MODE",
        requires = "from"
    )]
    changes: ChangeDetectionMode,

    /// Starting reference point for comparison (required when using --changes)
    #[arg(long, requires = "changes")]
    from: Option<String>,

    /// Ending reference point (defaults to current state when using --changes)
    #[arg(long, requires = "from")]
    to: Option<String>,

    /// Specify a custom test runner
    #[arg(short = 'r', value_enum, default_value_t)]
    test_runner: TestRunnerType,

    /// Include tests for crates dependent on the changed crates in the test run
    #[arg(short = 'd', long)]
    with_dependents: bool,

    /// Skip running tests, only print the crates that would be tested
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Display full output while running tests
    #[arg(short = 'v', long)]
    verbose: bool,

    /// Run tests for all crates regardless of failure
    #[arg(short = 'k', long)]
    no_fail_fast: bool,

    /// Specify a set of crates to run tests for, typically for re-running failed tests
    #[arg(short = 'c', long, value_delimiter = ',')]
    crates: Vec<String>,

    /// Output in JSON format for machine consumption
    #[arg(short = 'j', long)]
    json: bool,

    /// Additional arguments to pass to the test runner
    #[arg(last = true)]
    test_runner_args: Vec<String>,
}

#[derive(ValueEnum, Clone, Debug, Default)]
enum ChangeDetectionMode {
    /// Use uncommitted changes in working directory (default)
    #[default]
    Working,
    /// Compare changes between specific references
    Refs,
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

    // Create a reporter
    let mut reporter = if args.json {
        Box::new(reporting::json::JsonReporter::new(stdout())) as Box<dyn Reporter>
    } else {
        Box::new(reporting::console::ConsoleReporter::new(
            stdout(),
            args.verbose,
        )) as Box<dyn Reporter>
    };

    // Get workspace and repository information
    let vcs = VcsType::Git.create();
    let workspace_root = vcs.get_workspace_root(Path::new("."))?;

    let changed_files = match &args.changes {
        ChangeDetectionMode::Working => vcs.get_uncommitted_changes(&workspace_root)?,
        ChangeDetectionMode::Refs => {
            let from_ref = args
                .from
                .as_deref()
                .ok_or_else(|| AppError::InvalidArguments {
                    reason: "--from is required when using --changes=refs".to_string(),
                })?;

            vcs.get_changes_between(&workspace_root, from_ref, args.to.as_deref())?
        }
    };

    reporter.changed_files(changed_files.as_slice(), &workspace_root);

    let metadata = metadata::get_workspace_metadata(&workspace_root)?;
    let crates = metadata::get_workspace_crates(&metadata)?;

    // Identify which crates need testing
    let crates = if args.crates.is_empty() {
        let changed_crates = metadata::find_changed_crates(&changed_files, &crates)?;

        let mut crates_to_test = IndexSet::new();

        crates_to_test.extend(
            changed_crates
                .iter()
                .map(|name| DiscoveredTestCrate {
                    name: name.to_string(),
                    discovery_type: DiscoveryType::Modified,
                })
                .collect::<Vec<_>>(),
        );

        crates_to_test.extend(
            metadata::find_dependent_crates(&changed_crates, &metadata)?
                .into_iter()
                .map(|name| DiscoveredTestCrate {
                    name: name.to_string(),
                    discovery_type: DiscoveryType::Dependent,
                })
                .collect::<Vec<_>>(),
        );

        TestCrates::Discovered(crates_to_test)
    } else {
        metadata::verify_crates_exist(&metadata, args.crates.as_slice())?;
        TestCrates::Manual(IndexSet::from_iter(
            args.crates.into_iter().map(|name| ManualTestCrate { name }),
        ))
    };

    // Get the appropriate test runner
    let runner = args.test_runner.create();

    // Execute the tests
    let test_plan = TestPlan {
        workspace_root,
        crates,
        with_dependents: args.with_dependents,
        fail_fast: !args.no_fail_fast,
        verbose: args.verbose,
        test_runner_args: args.test_runner_args,
    };

    testing::run_tests(test_plan, runner.as_ref(), args.dry_run, reporter.as_mut())
}
