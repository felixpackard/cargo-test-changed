use std::{fs::File, io::Write, path::PathBuf, process::Command};

use tempfile::TempDir;

pub struct TestRepo {
    #[allow(dead_code)]
    pub temp_dir: TempDir,
    pub repo_path: PathBuf,
}

impl TestRepo {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = tempfile::tempdir()?;
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Command::new("git")
            .args(&["init"])
            .current_dir(&repo_path)
            .output()?;

        // Configure git user
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()?;
        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()?;

        Ok(Self {
            temp_dir,
            repo_path,
        })
    }

    pub fn create_file(
        &self,
        filename: &str,
        content: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let file_path = self.repo_path.join(filename);
        let mut file = File::create(&file_path)?;
        writeln!(file, "{}", content)?;
        Ok(file_path)
    }

    pub fn modify_file(
        &self,
        filename: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = self.repo_path.join(filename);
        let mut file = File::create(&file_path)?;
        writeln!(file, "{}", content)?;
        Ok(())
    }

    pub fn stage_file(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        Command::new("git")
            .args(&["add", filename])
            .current_dir(&self.repo_path)
            .output()?;
        Ok(())
    }

    pub fn stage_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        Command::new("git")
            .args(&["add", "--all"])
            .current_dir(&self.repo_path)
            .output()?;
        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<String, Box<dyn std::error::Error>> {
        Command::new("git")
            .args(&["commit", "-m", message])
            .current_dir(&self.repo_path)
            .output()?;

        let output = Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .current_dir(&self.repo_path)
            .output()?;

        let commit_hash = String::from_utf8(output.stdout)?.trim().to_string();
        Ok(commit_hash)
    }

    pub fn create_and_commit_file(
        &self,
        filename: &str,
        content: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.create_file(filename, content)?;
        self.stage_file(filename)?;
        self.commit(&format!("Add {}", filename))
    }

    pub fn create_symlink(
        &self,
        link_name: &str,
        target: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let link_path = self.repo_path.join(link_name);

        #[cfg(unix)]
        std::os::unix::fs::symlink(target, &link_path)?;

        #[cfg(windows)]
        {
            let target_path = self.repo_path.join(target);
            if target_path.is_dir() {
                std::os::windows::fs::symlink_dir(target, &link_path)?;
            } else {
                std::os::windows::fs::symlink_file(target, &link_path)?;
            }
        }

        Ok(link_path)
    }
}
