use super::TestRunner;
use colored::Colorize;
use std::process::Command;

pub struct NextestRunner;

impl TestRunner for NextestRunner {
    fn command(&self, crate_name: &str) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.args(["nextest", "run", "--no-tests", "pass", "-p", crate_name]);
        cmd
    }

    fn is_installed(&self) -> bool {
        std::process::Command::new("cargo")
            .args(["nextest", "--version"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn installation_instructions(&self) -> String {
        format!(
            "to install nextest, run '{}'",
            "cargo install cargo-nextest".bold().yellow()
        )
    }

    fn name(&self) -> &'static str {
        "nextest"
    }
}
