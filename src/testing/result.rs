use std::time::Duration;

#[derive(Debug)]
pub struct TestResults {
    pub passed: Vec<TestResult>,
    pub failed: Vec<TestResult>,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct TestResult {
    pub crate_name: String,
    pub success: bool,
    pub output: String,
}

impl TestResults {
    pub fn new() -> Self {
        TestResults {
            passed: Vec::new(),
            failed: Vec::new(),
            duration: Duration::from_secs(0),
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        if result.success {
            self.passed.push(result);
        } else {
            self.failed.push(result);
        }
    }

    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }
}
