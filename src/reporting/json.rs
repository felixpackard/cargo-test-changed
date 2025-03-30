use crate::testing::{
    plan::{DiscoveryType, TestCrates, TestPlan},
    result::TestResult,
};

use super::Reporter;
use serde::Serialize;
use std::io::{self, Write};

#[derive(Serialize)]
struct JsonEvent {
    event_type: String,
    payload: serde_json::Value,
    timestamp: u128,
}

pub struct JsonReporter<W: Write> {
    writer: W,
}

impl<W: Write> JsonReporter<W> {
    pub fn new(writer: W) -> Self {
        JsonReporter { writer }
    }

    fn emit_event(&mut self, event_type: &str, payload: serde_json::Value) {
        let event = JsonEvent {
            event_type: event_type.to_string(),
            payload,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        };

        writeln!(self.writer, "{}", serde_json::to_string(&event).unwrap()).unwrap();
        self.flush().unwrap();
    }
}

impl<W: Write> Reporter for JsonReporter<W> {
    fn note(&mut self, message: &str) {
        self.emit_event("note", serde_json::json!({ "message": message }));
    }

    fn tip(&mut self, message: &str) {
        self.emit_event("tip", serde_json::json!({ "message": message }));
    }

    fn error(&mut self, message: &str) {
        self.emit_event("error", serde_json::json!({ "message": message }));
    }

    fn test_start(&mut self, crate_name: &str, test_number: usize, total_tests: usize) {
        self.emit_event("test_start", serde_json::json!({ "crate": crate_name, "test_number": test_number, "total_tests": total_tests }));
    }

    fn test_result(&mut self, crate_name: &str, success: bool, duration_ms: u64) {
        self.emit_event(
            "test_result",
            serde_json::json!({
                "crate": crate_name,
                "success": success,
                "duration_ms": duration_ms
            }),
        );
    }

    fn test_summary(&mut self, passed: usize, failed: usize, duration_secs: f64) {
        self.emit_event(
            "test_summary",
            serde_json::json!({
                "passed": passed,
                "failed": failed,
                "duration_secs": duration_secs
            }),
        );
    }

    fn plan_summary(&mut self, test_plan: &TestPlan) {
        match &test_plan.crates {
            TestCrates::Manual(crates) => {
                self.emit_event(
                    "plan_summary",
                    serde_json::json!({
                        "run_type": "manual",
                        "crate_count": crates.len(),
                    }),
                );
            }
            TestCrates::Discovered(crates) => {
                let (modified, dependent) = crates.iter().partition::<Vec<_>, _>(|c| {
                    matches!(c.discovery_type, DiscoveryType::Modified)
                });

                let (modified_count, dependent_count) = (modified.len(), dependent.len());

                self.emit_event(
                    "plan_summary",
                    serde_json::json!({
                        "run_type": "discovered",
                        "modified_count": modified_count,
                        "dependent_count": dependent_count,
                        "with_dependents": test_plan.with_dependents,
                    }),
                );
            }
        }
    }

    fn test_failures(&mut self, failures: &Vec<TestResult>) {
        for failure in failures.iter() {
            self.test_failure_details(&failure.crate_name, &failure.output);
        }
    }

    fn test_failure_details(&mut self, crate_name: &str, output: &str) {
        self.emit_event(
            "test_failure",
            serde_json::json!({
                "crate": crate_name,
                "output": output
            }),
        );
    }

    fn no_tests(&mut self) {
        self.emit_event("no_tests", serde_json::json!({}));
    }

    fn dry_run(&mut self) {
        self.emit_event("dry_run", serde_json::json!({}));
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
