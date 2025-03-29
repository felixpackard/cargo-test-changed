use crate::testing::result::TestResult;

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

    fn test_start(&mut self, crate_name: &str) {
        write!(
            self.writer,
            "{}test crate {}",
            if self.verbose { "📦 " } else { "" },
            crate_name
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

    fn plan_summary(&mut self, direct_count: usize, dependent_count: usize, skip_dependents: bool) {
        let direct_word = pluralize(direct_count, "crate", "crates");
        let dependent_word = pluralize(dependent_count, "crate", "crates");

        writeln!(
            self.writer,
            "discovered {} changed {}; {}{} dependent {}\n",
            direct_count,
            direct_word,
            if skip_dependents { "skipping " } else { "" },
            dependent_count,
            dependent_word
        )
        .unwrap();
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
        writeln!(self.writer, "---- {} output ----\n{}\n", crate_name, output).unwrap();
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
