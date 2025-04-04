use crate::{
    testing::{
        plan::{TestCrates, TestPlan},
        result::TestResult,
    },
    vcs::ChangedFile,
};

use super::Reporter;
use serde::Serialize;
use std::{
    io::{self, Write},
    path::Path,
};

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

    /// Helper method to safely emit an event, handling all potential errors
    fn emit_event(&mut self, event_type: &str, payload: serde_json::Value) {
        let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_millis(),
            Err(e) => {
                eprintln!("Error getting system time: {}", e);
                0 // Fallback to 0 on error
            }
        };

        let event = JsonEvent {
            event_type: event_type.to_string(),
            payload,
            timestamp,
        };

        let json_string = match serde_json::to_string(&event) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("JSON serialization error: {}", e);
                return;
            }
        };

        if let Err(e) = writeln!(self.writer, "{}", json_string) {
            eprintln!("Write error: {}", e);
        }

        let _ = self.flush();
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

    fn changed_files(&mut self, changed_files: &[ChangedFile], _: &Path) {
        self.emit_event(
            "changed_files",
            serde_json::json!({ "files": changed_files }),
        );
    }

    fn test_start(&mut self, crate_name: &str, test_number: usize, total_tests: usize) {
        self.emit_event(
            "test_start",
            serde_json::json!({
                "crate": crate_name,
                "test_number": test_number,
                "total_tests": total_tests
            }),
        );
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
                        "crates": crates,
                    }),
                );
            }
            TestCrates::Discovered(crates) => {
                self.emit_event(
                    "plan_summary",
                    serde_json::json!({
                        "run_type": "discovered",
                        "with_dependents": test_plan.with_dependents,
                        "crates": crates,
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
