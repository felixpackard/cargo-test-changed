pub mod executor;
pub mod plan;
pub mod result;

use anyhow::Result;

use crate::error::AppError;
use crate::reporting::Reporter;
use crate::test_runner::TestRunner;
use executor::TestExecutor;
use plan::TestPlan;

pub fn run_tests(
    test_plan: TestPlan,
    runner: &dyn TestRunner,
    dry_run: bool,
    reporter: &mut dyn Reporter,
) -> Result<(), AppError> {
    if test_plan.is_empty() {
        reporter.no_tests();
        return Ok(());
    }

    reporter.plan_summary(&test_plan);

    if dry_run {
        reporter.dry_run();
        return Ok(());
    }

    let mut executor = TestExecutor::new(&test_plan, runner, reporter);
    let results = executor.execute()?;

    if !test_plan.verbose && results.has_failures() {
        reporter.test_failures(&results.failed);
    }

    reporter.test_summary(
        results.passed.len(),
        results.failed.len(),
        results.duration.as_secs_f64(),
    );

    if results.has_failures() {
        return Err(AppError::TestsFailed {
            failed_crates: results.failed.into_iter().map(|c| c.crate_name).collect(),
        });
    }

    Ok(())
}
