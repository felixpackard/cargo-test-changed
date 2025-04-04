use std::path::Path;

use crate::{
    testing::{plan::TestPlan, result::TestResult},
    vcs::ChangedFile,
};

pub mod console;
pub mod json;

/// Reporter trait for different output formats
pub trait Reporter {
    /// Report a note message
    fn note(&mut self, message: &str);

    /// Report a tip message
    fn tip(&mut self, message: &str);

    /// Report an error message
    fn error(&mut self, message: &str);

    /// Report the list of changed files
    fn changed_files(&mut self, changed_files: &[ChangedFile], workspace_root: &Path);

    /// Report the test start
    fn test_start(&mut self, crate_name: &str, test_number: usize, total_tests: usize);

    /// Report a test result (success or failure)
    fn test_result(&mut self, crate_name: &str, success: bool, duration_ms: u64);

    /// Report test summary
    fn test_summary(&mut self, passed: usize, failed: usize, duration_secs: f64);

    /// Report a test plan summary
    fn plan_summary(&mut self, test_plan: &TestPlan);

    /// Report all test failures
    fn test_failures(&mut self, failures: &[TestResult]);

    /// Report a test failure details
    fn test_failure_details(&mut self, crate_name: &str, output: &str);

    /// Report that no tests will run
    fn no_tests(&mut self);

    /// Report dry run mode
    fn dry_run(&mut self);

    /// Flush any buffered output
    fn flush(&mut self) -> std::io::Result<()>;
}

/// Return the singular or plural form of a word based on the count
pub fn pluralize(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        singular.to_string()
    } else {
        plural.to_string()
    }
}
