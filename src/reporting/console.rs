use crate::{
    testing::{
        plan::{DiscoveryType, TestCrates, TestPlan},
        result::TestResult,
    },
    vcs::ChangedFile,
};

use super::{pluralize, Reporter};
use colored::Colorize;
use std::{
    io::{self, Write},
    path::Path,
};

pub struct ConsoleReporter<W: Write> {
    writer: W,
    verbose: bool,
}

impl<W: Write> ConsoleReporter<W> {
    pub fn new(writer: W, verbose: bool) -> Self {
        ConsoleReporter { writer, verbose }
    }

    /// Write formatted output to the console and handle errors
    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> io::Result<()> {
        match self.writer.write_fmt(args) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("Write error: {}", e);
                Err(e)
            }
        }
    }

    /// Write formatted output to the console and discard errors
    fn try_write(&mut self, args: std::fmt::Arguments<'_>) {
        let _ = self.write_fmt(args);
    }

    /// Write formatted output to the console with a newline and discard errors
    fn try_writeln(&mut self, args: std::fmt::Arguments<'_>) {
        self.try_write(args);
        self.try_write(format_args!("\n"));
    }
}

impl<W: Write> Reporter for ConsoleReporter<W> {
    fn note(&mut self, message: &str) {
        self.try_writeln(format_args!("{}: {}", "note".bold().cyan(), message));
    }

    fn tip(&mut self, message: &str) {
        self.try_writeln(format_args!("  {}: {}", "tip".bold().cyan(), message));
    }

    fn error(&mut self, message: &str) {
        self.try_writeln(format_args!("{}: {}", "error".bold().red(), message));
    }

    fn changed_files(&mut self, changed_files: &[ChangedFile], workspace_root: &Path) {
        if !self.verbose {
            return;
        }

        let files_count = changed_files.len();
        let files_word = pluralize(files_count, "file", "files");

        self.try_writeln(format_args!(
            "discovered {} changed {}:",
            files_count, files_word,
        ));

        for change in changed_files.iter() {
            let symbol = match change.change_type {
                crate::vcs::ChangeType::Added => "+".bold().green(),
                crate::vcs::ChangeType::Modified => "*".bold().yellow(),
                crate::vcs::ChangeType::Removed => "-".bold().red(),
            };

            let relative_path = pathdiff::diff_paths(&change.current_path, workspace_root);

            self.try_writeln(format_args!(
                "  {} {}",
                symbol,
                relative_path
                    .as_ref()
                    .unwrap_or(&change.current_path)
                    .display()
            ));
        }

        self.try_write(format_args!("\n"));
    }

    fn test_start(&mut self, crate_name: &str, test_number: usize, total_tests: usize) {
        let width = total_tests.to_string().len();
        let prefix = if self.verbose { "📦 " } else { "" };

        self.try_write(format_args!(
            "{}{:width$}/{} test crate {}",
            prefix,
            test_number,
            total_tests,
            crate_name,
            width = width
        ));

        if self.verbose {
            self.try_write(format_args!("\n"));
        } else {
            self.try_write(format_args!(" ... "));
        }

        let _ = self.flush();
    }

    fn test_result(&mut self, _: &str, success: bool, _: u64) {
        if self.verbose {
            self.try_write(format_args!("\n"));
        } else if success {
            self.try_writeln(format_args!("{}", "ok".bold().green()));
        } else {
            self.try_writeln(format_args!("{}", "FAILED".bold().red()));
        }
    }

    fn test_summary(&mut self, passed: usize, failed: usize, duration_secs: f64) {
        if !self.verbose {
            self.try_write(format_args!("\n"));
        }

        let status = if failed == 0 {
            "ok".bold().green()
        } else {
            "FAILED".bold().red()
        };

        self.try_writeln(format_args!(
            "test result: {}. {} passed; {} failed; finished in {:.2}s\n",
            status, passed, failed, duration_secs
        ));
    }

    fn plan_summary(&mut self, test_plan: &TestPlan) {
        match &test_plan.crates {
            TestCrates::Manual(crates) => {
                let word = pluralize(crates.len(), "crate", "crates");
                self.try_writeln(format_args!("manually testing {} {}\n", crates.len(), word));
            }
            TestCrates::Discovered(crates) => {
                let (modified, dependent) = crates.iter().partition::<Vec<_>, _>(|c| {
                    matches!(c.discovery_type, DiscoveryType::Modified)
                });

                let (modified_count, dependent_count) = (modified.len(), dependent.len());
                let modified_word = pluralize(modified_count, "crate", "crates");

                self.try_write(format_args!(
                    "discovered {} changed {}",
                    modified_count, modified_word
                ));

                if test_plan.with_dependents {
                    let dependent_word = pluralize(dependent_count, "crate", "crates");
                    self.try_write(format_args!(
                        "; {} dependent {}",
                        dependent_count, dependent_word
                    ));
                }

                if self.verbose {
                    self.try_writeln(format_args!(":"));

                    let test_crates = crates.iter().filter(|c| {
                        test_plan.with_dependents
                            || matches!(c.discovery_type, DiscoveryType::Modified)
                    });

                    for test_crate in test_crates {
                        let symbol = match test_crate.discovery_type {
                            DiscoveryType::Modified => "*".bold().yellow(),
                            DiscoveryType::Dependent => ">".bold().red(),
                        };
                        self.try_writeln(format_args!("  {} {}", symbol, test_crate.name));
                    }
                } else {
                    self.try_write(format_args!("\n"));
                }

                self.try_write(format_args!("\n"));
            }
        }
    }

    fn test_failures(&mut self, failures: &Vec<TestResult>) {
        self.try_writeln(format_args!("\nfailed crate output:\n"));

        for failure in failures.iter() {
            self.test_failure_details(&failure.crate_name, &failure.output);
        }

        self.try_writeln(format_args!("\nfailed crates:"));

        for failure in failures.iter() {
            self.try_writeln(format_args!("    {}", failure.crate_name));
        }
    }

    fn test_failure_details(&mut self, crate_name: &str, output: &str) {
        self.try_writeln(format_args!(
            "---- 📦 {} output ----\n{}\n",
            crate_name, output
        ));
    }

    fn no_tests(&mut self) {
        self.try_writeln(format_args!("no crates to test"));
    }

    fn dry_run(&mut self) {
        self.note("dry run mode enabled, skipping actual tests");
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
