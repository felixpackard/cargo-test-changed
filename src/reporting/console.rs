use crate::testing::{
    plan::{DiscoveryType, TestCrates, TestPlan},
    result::TestResult,
};

use super::{pluralize, Reporter};
use colored::Colorize;
use std::io::{self, Write};

pub struct ConsoleReporter<W: Write> {
    writer: W,
    verbose: bool,
}

impl<W: Write> ConsoleReporter<W> {
    pub fn new(writer: W, verbose: bool) -> Self {
        ConsoleReporter { writer, verbose }
    }
}

impl<W: Write> Reporter for ConsoleReporter<W> {
    fn note(&mut self, message: &str) {
        writeln!(self.writer, "{}: {}", "note".bold().cyan(), message).unwrap();
    }

    fn tip(&mut self, message: &str) {
        writeln!(self.writer, "  {}: {}", "tip".bold().cyan(), message).unwrap();
    }

    fn error(&mut self, message: &str) {
        writeln!(self.writer, "{}: {}", "error".bold().red(), message).unwrap();
    }

    fn test_start(&mut self, crate_name: &str, test_number: usize, total_tests: usize) {
        write!(
            self.writer,
            "{}{:width$}/{} test crate {}",
            if self.verbose { "📦 " } else { "" },
            test_number,
            total_tests,
            crate_name,
            width = total_tests.to_string().len()
        )
        .unwrap();

        if self.verbose {
            writeln!(self.writer).unwrap();
        } else {
            write!(self.writer, " ... ").unwrap();
        }

        self.flush().unwrap();
    }

    fn test_result(&mut self, _: &str, success: bool, _: u64) {
        if self.verbose {
            writeln!(self.writer).unwrap();
        } else {
            if success {
                writeln!(self.writer, "{}", "ok".bold().green()).unwrap();
            } else {
                writeln!(self.writer, "{}", "FAILED".bold().red()).unwrap();
            }
        }
    }

    fn test_summary(&mut self, passed: usize, failed: usize, duration_secs: f64) {
        if !self.verbose {
            writeln!(self.writer).unwrap();
        }

        writeln!(
            self.writer,
            "test result: {}. {} passed; {} failed; finished in {:.2}s\n",
            if failed == 0 {
                "ok".bold().green()
            } else {
                "FAILED".bold().red()
            },
            passed,
            failed,
            duration_secs
        )
        .unwrap();
    }

    fn plan_summary(&mut self, test_plan: &TestPlan) {
        match &test_plan.crates {
            TestCrates::Manual(crates) => {
                let word = pluralize(crates.len(), "crate", "crates");
                writeln!(self.writer, "manually testing {} {}\n", crates.len(), word).unwrap();
            }
            TestCrates::Discovered(crates) => {
                let (modified, dependent) = crates.iter().partition::<Vec<_>, _>(|c| {
                    matches!(c.discovery_type, DiscoveryType::Modified)
                });

                let (modified_count, dependent_count) = (modified.len(), dependent.len());

                let modified_word = pluralize(modified_count, "crate", "crates");
                let dependent_word = pluralize(dependent_count, "crate", "crates");

                write!(
                    self.writer,
                    "discovered {} changed {}",
                    modified_count, modified_word
                )
                .unwrap();

                if test_plan.with_dependents {
                    write!(
                        self.writer,
                        "; {} dependent {}",
                        dependent_count, dependent_word
                    )
                    .unwrap();
                }

                writeln!(self.writer, "\n").unwrap();
            }
        }
    }

    fn test_failures(&mut self, failures: &Vec<TestResult>) {
        writeln!(self.writer, "\nfailed crate output:\n").unwrap();
        for failure in failures.iter() {
            self.test_failure_details(&failure.crate_name, &failure.output);
        }
        writeln!(self.writer, "\nfailed crates:").unwrap();
        for failure in failures.iter() {
            writeln!(self.writer, "    {}", failure.crate_name).unwrap();
        }
    }

    fn test_failure_details(&mut self, crate_name: &str, output: &str) {
        writeln!(
            self.writer,
            "---- 📦 {} output ----\n{}\n",
            crate_name, output
        )
        .unwrap();
    }

    fn no_tests(&mut self) {
        writeln!(self.writer, "no crates to test").unwrap();
    }

    fn dry_run(&mut self) {
        self.note("dry run mode enabled, skipping actual tests");
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
