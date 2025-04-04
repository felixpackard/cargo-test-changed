use clap::ValueEnum;
use std::process::Command;

mod cargo;
mod nextest;

pub use cargo::CargoRunner;
pub use nextest::NextestRunner;

pub trait TestRunner {
    /// Get the command to run the tests
    fn command(&self, crate_name: &str) -> Command;

    /// Check if the test runner is installed
    fn is_installed(&self) -> bool;

    /// Get the installation instructions for the test runner
    fn installation_instructions(&self) -> String;

    /// Get the name of the test runner
    fn name(&self) -> &'static str;
}

#[derive(ValueEnum, Debug, Clone, Default)]
pub enum TestRunnerType {
    #[default]
    Cargo,
    Nextest,
}

impl TestRunnerType {
    pub fn create(&self) -> Box<dyn TestRunner> {
        match self {
            TestRunnerType::Cargo => Box::new(CargoRunner),
            TestRunnerType::Nextest => Box::new(NextestRunner),
        }
    }
}
