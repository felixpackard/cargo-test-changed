use clap::ValueEnum;
use std::process::Command;

mod cargo;
mod nextest;

pub use cargo::CargoRunner;
pub use nextest::NextestRunner;

pub trait TestRunner {
    fn command(&self, crate_name: &str) -> Command;
    fn is_installed(&self) -> bool;
    fn installation_instructions(&self) -> String;
    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone, ValueEnum)]
pub enum TestRunnerType {
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
