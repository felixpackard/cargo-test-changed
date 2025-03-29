use indexmap::IndexSet;

#[derive(Debug)]
pub struct TestPlan {
    pub workspace_root: std::path::PathBuf,
    pub crates: IndexSet<TestCrate>,
    pub skip_dependents: bool,
    pub fail_fast: bool,
    pub verbose: bool,
    pub runner_args: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TestCrate {
    pub name: String,
    pub is_direct: bool, // true for directly changed crates, false for dependents
}

impl TestPlan {
    pub fn new(
        workspace_root: std::path::PathBuf,
        changed_crates: &IndexSet<String>,
        dependent_crates: &IndexSet<String>,
        skip_dependents: bool,
        fail_fast: bool,
        verbose: bool,
        runner_args: Vec<String>,
    ) -> Self {
        let mut crates = IndexSet::new();

        crates.extend(changed_crates.iter().map(|crate_name| TestCrate {
            name: crate_name.clone(),
            is_direct: true,
        }));

        crates.extend(dependent_crates.iter().map(|crate_name| TestCrate {
            name: crate_name.clone(),
            is_direct: false,
        }));

        TestPlan {
            workspace_root,
            crates,
            skip_dependents,
            fail_fast,
            verbose,
            runner_args,
        }
    }

    pub fn get_crates_to_test(&self) -> Vec<&TestCrate> {
        if self.skip_dependents {
            self.crates.iter().filter(|c| c.is_direct).collect()
        } else {
            self.crates.iter().collect()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.get_crates_to_test().is_empty()
    }
}
