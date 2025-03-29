use anyhow::Result;
use std::io::{Read, Write};
use std::process::Stdio;
use std::time::Instant;

use super::plan::{TestCrate, TestPlan};
use super::result::{TestResult, TestResults};
use crate::error::AppError;
use crate::reporting::Reporter;
use crate::test_runner::TestRunner;

pub struct TestExecutor<'a> {
    plan: &'a TestPlan,
    runner: &'a dyn TestRunner,
    reporter: &'a mut dyn Reporter,
}

impl<'a> TestExecutor<'a> {
    pub fn new(
        plan: &'a TestPlan,
        runner: &'a dyn TestRunner,
        reporter: &'a mut dyn Reporter,
    ) -> Self {
        TestExecutor {
            plan,
            runner,
            reporter,
        }
    }

    pub fn execute(&mut self) -> Result<TestResults, AppError> {
        let mut results = TestResults::new();
        let start_time = Instant::now();

        if !self.runner.is_installed() {
            return Err(AppError::TestRunnerNotInstalled {
                runner_name: self.runner.name().to_string(),
                installation_tip: self.runner.installation_instructions(),
            });
        }

        let crates_to_test = &self.plan.get_crates_to_test();
        for (index, test_crate) in crates_to_test.iter().enumerate() {
            let result = self.execute_single_test(test_crate, index + 1, crates_to_test.len())?;

            let should_stop = !result.success && self.plan.fail_fast;
            results.add_result(result);

            if should_stop {
                break;
            }
        }

        results.duration = start_time.elapsed();
        Ok(results)
    }

    fn execute_single_test(
        &mut self,
        test_crate: &TestCrate,
        test_number: usize,
        total_tests: usize,
    ) -> Result<TestResult, AppError> {
        self.reporter
            .test_start(&test_crate.name, test_number, total_tests);

        std::io::stdout().flush().unwrap();

        let crate_start = Instant::now();
        let mut cmd = self.runner.command(&test_crate.name);
        cmd.args(&self.plan.runner_args);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .current_dir(&self.plan.workspace_root)
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

            if self.plan.verbose {
                while let Some((byte_result, _is_stderr)) = merged_output.next() {
                    match byte_result {
                        Ok(byte) => {
                            std::io::stdout().write_all(&[byte]).map_err(|e| {
                                AppError::CommandFailed {
                                    command: format!("{:?}", cmd),
                                    reason: format!("Failed to write to stdout: {}", e),
                                }
                            })?;
                            std::io::stdout().flush().unwrap();
                            output_capture.push(byte);
                        }
                        Err(e) => {
                            if e.kind() != std::io::ErrorKind::BrokenPipe {
                                return Err(AppError::CommandFailed {
                                    command: format!("{:?}", cmd),
                                    reason: format!("Failed to read output: {}", e),
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
                                    reason: format!("Failed to read output: {}", e),
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

        let success = status.success();
        let duration = crate_start.elapsed();

        self.reporter
            .test_result(&test_crate.name, success, duration.as_millis() as u64);

        Ok(TestResult {
            crate_name: test_crate.name.clone(),
            success,
            output: String::from_utf8_lossy(&output_capture).into_owned(),
        })
    }
}
