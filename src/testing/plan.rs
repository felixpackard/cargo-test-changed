use indexmap::IndexSet;

#[derive(Debug)]
pub struct TestPlan {
    pub workspace_root: std::path::PathBuf,
    pub crates: TestCrates,
    pub with_dependents: bool,
    pub fail_fast: bool,
    pub verbose: bool,
    pub test_runner_args: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DiscoveryType {
    Modified,
    Dependent,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ManualTestCrate {
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct DiscoveredTestCrate {
    pub name: String,
    pub discovery_type: DiscoveryType,
}

#[derive(Debug)]
pub enum TestCrates {
    Manual(IndexSet<ManualTestCrate>),
    Discovered(IndexSet<DiscoveredTestCrate>),
}

impl TestPlan {
    pub fn get_crates_to_test(&self) -> Vec<&String> {
        match &self.crates {
            TestCrates::Manual(crates) => crates.iter().map(|c| &c.name).collect(),
            TestCrates::Discovered(crates) => {
                if self.with_dependents {
                    crates.iter().map(|c| &c.name).collect()
                } else {
                    crates
                        .iter()
                        .filter(|c| matches!(c.discovery_type, DiscoveryType::Modified))
                        .map(|c| &c.name)
                        .collect()
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.get_crates_to_test().is_empty()
    }
}
