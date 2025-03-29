use super::TestRunner;
use std::process::Command;

pub struct CargoRunner;

impl TestRunner for CargoRunner {
    fn command(&self, crate_name: &str) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.args(["test", "-p", crate_name]);
        cmd
    }

    fn is_installed(&self) -> bool {
        // Cargo is always installed if we're running a cargo command
        true
    }

    fn installation_instructions(&self) -> String {
        "cargo should be available since you're running this as a cargo command".to_string()
    }

    fn name(&self) -> &'static str {
        "cargo"
    }
}
