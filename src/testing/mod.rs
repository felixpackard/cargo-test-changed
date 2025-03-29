pub mod executor;
pub mod plan;
pub mod result;

use anyhow::Result;
use indexmap::IndexSet;
use std::path::Path;

use crate::error::AppError;
use crate::reporting::Reporter;
use crate::test_runner::TestRunner;
use executor::TestExecutor;
use plan::TestPlan;

pub fn run_tests(
    workspace_root: &Path,
    runner: &dyn TestRunner,
    changed_crates: &IndexSet<String>,
    dependent_crates: &IndexSet<String>,
    skip_dependents: bool,
    fail_fast: bool,
    verbose: bool,
    runner_args: Vec<String>,
    dry_run: bool,
    reporter: &mut dyn Reporter,
) -> Result<(), AppError> {
    let test_plan = TestPlan::new(
        workspace_root.to_path_buf(),
        changed_crates,
        dependent_crates,
        skip_dependents,
        fail_fast,
        verbose,
        runner_args,
    );

    if test_plan.is_empty() {
        reporter.no_tests();
        return Ok(());
    }

    let (direct, indirect) = test_plan
        .crates
        .iter()
        .partition::<Vec<_>, _>(|c| c.is_direct);

    reporter.plan_summary(direct.len(), indirect.len(), skip_dependents);

    if dry_run {
        reporter.dry_run();
        return Ok(());
    }

    let mut executor = TestExecutor::new(&test_plan, runner, reporter);
    let results = executor.execute()?;

    if !verbose && results.has_failures() {
        reporter.test_failures(&results.failed);
    }

    reporter.test_summary(
        results.passed.len(),
        results.failed.len(),
        results.duration.as_secs_f64(),
    );

    if results.has_failures() {
        return Err(AppError::TestsFailed);
    }

    Ok(())
}
