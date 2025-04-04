use anyhow::Result;
use std::io::{Read, Write};
use std::process::Stdio;
use std::time::Instant;

use super::plan::TestPlan;
use super::result::{TestResult, TestResults};
use crate::error::AppError;
use crate::reporting::Reporter;
use crate::test_runner::TestRunner;

pub struct TestExecutor<'a> {
    test_plan: &'a TestPlan,
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
            test_plan: plan,
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

        let crates_to_test = &self.test_plan.get_crates_to_test();
        for (index, test_crate) in crates_to_test.iter().enumerate() {
            let result = self.execute_single_test(test_crate, index + 1, crates_to_test.len())?;

            let should_stop = !result.success && self.test_plan.fail_fast;
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
        crate_name: &str,
        test_number: usize,
        total_tests: usize,
    ) -> Result<TestResult, AppError> {
        self.reporter
            .test_start(crate_name, test_number, total_tests);

        let _ = std::io::stdout().flush();

        let crate_start = Instant::now();
        let mut cmd = self.runner.command(crate_name);
        cmd.args(&self.test_plan.test_runner_args);

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .current_dir(&self.test_plan.workspace_root)
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

            if self.test_plan.verbose {
                for (byte_result, _is_stderr) in merged_output.by_ref() {
                    match byte_result {
                        Ok(byte) => {
                            std::io::stdout().write_all(&[byte]).map_err(|e| {
                                AppError::CommandFailed {
                                    command: format!("{:?}", cmd),
                                    reason: format!("Failed to write to stdout: {}", e),
                                }
                            })?;
                            let _ = std::io::stdout().flush();
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
                for (byte_result, _is_stderr) in merged_output {
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
            .test_result(crate_name, success, duration.as_millis() as u64);

        Ok(TestResult {
            crate_name: crate_name.to_string(),
            success,
            output: String::from_utf8_lossy(&output_capture).into_owned(),
        })
    }
}
