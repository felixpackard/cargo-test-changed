use std::fs;
use std::process::Command;
use tempfile::TempDir;

use crate::error::AppError;
use crate::vcs::test_utils;
use crate::vcs::{ChangeType, FileType, GitVcs, Vcs};

mod workspace_root_tests {
    use super::*;

    #[test]
    fn test_get_workspace_root_from_repo_root() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create GitVcs instance and get workspace root
        let git_vcs = GitVcs;
        let workspace_root = git_vcs.get_workspace_root(&test_repo.repo_path)?;

        // The result should match the canonical repo path
        assert_eq!(workspace_root, test_repo.repo_path.canonicalize()?);

        Ok(())
    }

    #[test]
    fn test_get_workspace_root_from_subdirectory() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create a subdirectory in the repo
        let subdir_path = test_repo.repo_path.join("subdir");
        fs::create_dir(&subdir_path)?;

        // Create GitVcs instance and get workspace root
        let git_vcs = GitVcs;
        let workspace_root = git_vcs.get_workspace_root(&subdir_path)?;

        // The result should match the canonical repo path, not the subdirectory
        assert_eq!(workspace_root, test_repo.repo_path.canonicalize()?);

        Ok(())
    }

    #[test]
    fn test_get_workspace_root_not_in_git_repo() {
        // Create a temporary directory that is not a git repo
        let temp_dir = TempDir::new().unwrap();

        // Try to get workspace root
        let git_vcs = GitVcs;
        let result = git_vcs.get_workspace_root(temp_dir.path());

        // Should return an error since this is not a git repo
        assert!(result.is_err());

        // Verify it's the expected error type
        if let Err(AppError::GitDiscoveryFailed { reason: _ }) = result {
            // Test passed
        } else {
            panic!("Expected GitDiscoveryFailed error, got: {:?}", result);
        }
    }
}

mod uncommitted_changes_tests {
    use super::*;

    // Basic scenarios
    #[test]
    fn test_no_changes_in_clean_repo() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a clean test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create and commit a file so the repo isn't empty
        test_repo.create_and_commit_file("file.txt", "content")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should be empty since everything is committed
        assert!(changes.is_empty());

        Ok(())
    }

    #[test]
    fn test_added_file() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create an untracked file
        test_repo.create_file("new.txt", "new content")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one added file
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert_eq!(change.file_type, FileType::File);
        assert!(change.current_path.ends_with("new.txt"));
        assert!(change.old_path.is_none());

        Ok(())
    }

    #[test]
    fn test_modified_file() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create and commit a file
        test_repo.create_and_commit_file("file.txt", "initial content")?;

        // Modify the file without committing
        test_repo.modify_file("file.txt", "modified content")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one modified file
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Modified);
        assert_eq!(change.file_type, FileType::File);
        assert!(change.current_path.ends_with("file.txt"));
        assert!(change.old_path.is_none());

        Ok(())
    }

    #[test]
    fn test_deleted_file() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create and commit a file
        test_repo.create_and_commit_file("to_delete.txt", "content")?;

        // Delete the file without committing
        fs::remove_file(test_repo.repo_path.join("to_delete.txt"))?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one deleted file
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Removed);
        assert!(change.current_path.ends_with("to_delete.txt"));

        Ok(())
    }

    #[test]
    fn test_renamed_file() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create and commit a file
        test_repo.create_and_commit_file("original.txt", "content")?;

        // Rename the file using git mv
        Command::new("git")
            .args(["mv", "original.txt", "renamed.txt"])
            .current_dir(&test_repo.repo_path)
            .output()?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one renamed file
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Modified);
        assert!(change.current_path.ends_with("renamed.txt"));
        assert!(change.old_path.is_some());
        if let Some(old_path) = &change.old_path {
            assert!(old_path.ends_with("original.txt"));
        }

        Ok(())
    }

    // Complex scenarios
    #[test]
    fn test_multiple_changes() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create and commit initial files
        test_repo.create_and_commit_file("keep.txt", "initial content")?;
        test_repo.create_and_commit_file("modify.txt", "content to change")?;
        test_repo.create_and_commit_file("delete.txt", "content to delete")?;

        // Make multiple changes
        test_repo.create_file("new.txt", "new content")?;
        test_repo.modify_file("modify.txt", "modified content")?;
        fs::remove_file(test_repo.repo_path.join("delete.txt"))?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have three changes
        assert_eq!(changes.len(), 3);

        // Find each change by path
        let added = changes.iter().find(|c| c.current_path.ends_with("new.txt"));
        let modified = changes
            .iter()
            .find(|c| c.current_path.ends_with("modify.txt"));
        let deleted = changes
            .iter()
            .find(|c| c.current_path.ends_with("delete.txt"));

        // Verify all changes were found
        assert!(added.is_some());
        assert!(modified.is_some());
        assert!(deleted.is_some());

        // Check the properties of each change
        assert_eq!(added.unwrap().change_type, ChangeType::Added);
        assert_eq!(modified.unwrap().change_type, ChangeType::Modified);
        assert_eq!(deleted.unwrap().change_type, ChangeType::Removed);

        Ok(())
    }

    #[test]
    fn test_symlink_change() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create a target file to link to
        test_repo.create_and_commit_file("target.txt", "target content")?;

        // Create a symlink to the target file
        test_repo.create_symlink("link.txt", "target.txt")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one symlink change
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert_eq!(change.file_type, FileType::Symlink);
        assert!(change.current_path.ends_with("link.txt"));
        assert!(change.old_path.is_none());

        Ok(())
    }

    #[test]
    fn test_directory_changes() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create a new directory with a file in it
        let dir_path = test_repo.repo_path.join("new_dir");
        fs::create_dir(&dir_path)?;
        test_repo.create_file("new_dir/file_in_dir.txt", "content in dir")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one file change in the directory
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert_eq!(change.file_type, FileType::File);
        assert!(change.current_path.ends_with("new_dir/file_in_dir.txt"));

        Ok(())
    }

    #[test]
    fn test_ignored_files() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create a .gitignore file to ignore *.log files
        test_repo.create_and_commit_file(".gitignore", "*.log\n")?;

        // Create an ignored file
        test_repo.create_file("ignored.log", "log content")?;

        // Create a non-ignored file for comparison
        test_repo.create_file("tracked.txt", "tracked content")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should only include the non-ignored file
        assert_eq!(changes.len(), 1);

        // Verify the only change is the tracked file, not the ignored one
        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert!(change.current_path.ends_with("tracked.txt"));

        Ok(())
    }

    #[test]
    fn test_file_mode_change() -> Result<(), Box<dyn std::error::Error>> {
        // Skip this test on Windows as file permissions work differently
        if cfg!(windows) {
            return Ok(());
        }

        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create and commit a file
        test_repo.create_and_commit_file("script.sh", "#!/bin/sh\necho 'Hello'")?;

        // Change file permissions to make it executable
        let file_path = test_repo.repo_path.join("script.sh");
        Command::new("chmod")
            .args(["+x", file_path.to_str().unwrap()])
            .output()?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one modified file with changed permissions
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Modified);
        assert!(change.current_path.ends_with("script.sh"));

        Ok(())
    }

    // Uncommitted changes specific tests
    #[test]
    fn test_staged_changes() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create and stage a new file (but don't commit)
        test_repo.create_file("staged.txt", "staged content")?;
        test_repo.stage_file("staged.txt")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one staged file
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert_eq!(change.file_type, FileType::File);
        assert!(change.current_path.ends_with("staged.txt"));

        Ok(())
    }

    #[test]
    fn test_unstaged_and_staged_changes() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create files and stage only one
        test_repo.create_file("staged.txt", "staged content")?;
        test_repo.create_file("unstaged.txt", "unstaged content")?;
        test_repo.stage_file("staged.txt")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have two files (one staged, one unstaged)
        assert_eq!(changes.len(), 2);

        // Find each change by path
        let staged = changes
            .iter()
            .find(|c| c.current_path.ends_with("staged.txt"));
        let unstaged = changes
            .iter()
            .find(|c| c.current_path.ends_with("unstaged.txt"));

        // Verify both changes were found
        assert!(staged.is_some());
        assert!(unstaged.is_some());

        // Both should be "Added" type
        assert_eq!(staged.unwrap().change_type, ChangeType::Added);
        assert_eq!(unstaged.unwrap().change_type, ChangeType::Added);

        Ok(())
    }

    #[test]
    fn test_empty_repository() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository without any commits
        let test_repo = test_utils::TestRepo::new()?;

        // Add a file but don't commit it
        test_repo.create_file("uncommitted.txt", "content")?;

        // Get uncommitted changes
        let git_vcs = GitVcs;
        let changes = git_vcs.get_uncommitted_changes(&test_repo.repo_path)?;

        // Should have one added file
        assert_eq!(changes.len(), 1);
        assert!(changes[0].current_path.ends_with("uncommitted.txt"));
        assert_eq!(changes[0].change_type, ChangeType::Added);

        Ok(())
    }
}

mod commit_diff_tests {
    use super::*;

    // Basic scenarios
    #[test]
    fn test_no_changes_between_same_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create and commit a file
        let commit_hash = test_repo.create_and_commit_file("file.txt", "content")?;

        // Get changes between the same commit
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit_hash, Some(&commit_hash))?;

        // Should be empty - no changes between the same commit
        assert!(changes.is_empty());

        Ok(())
    }

    #[test]
    fn test_added_file_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // First commit
        let commit1 = test_repo.create_and_commit_file("file1.txt", "content")?;

        // Add a new file and commit
        test_repo.create_file("file2.txt", "content of file 2")?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Second commit")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should have one added file
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert_eq!(change.file_type, FileType::File);
        assert!(change.current_path.ends_with("file2.txt"));
        assert!(change.old_path.is_none());

        Ok(())
    }

    #[test]
    fn test_modified_file_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // First commit
        let commit1 = test_repo.create_and_commit_file("file.txt", "initial content")?;

        // Modify the file and commit
        test_repo.modify_file("file.txt", "modified content")?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Modify file")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should have one modified file
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Modified);
        assert_eq!(change.file_type, FileType::File);
        assert!(change.current_path.ends_with("file.txt"));
        assert!(change.old_path.is_none());

        Ok(())
    }

    #[test]
    fn test_deleted_file_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // First commit with two files
        test_repo.create_file("keep.txt", "content to keep")?;
        test_repo.create_file("delete.txt", "content to delete")?;
        test_repo.stage_all()?;
        let commit1 = test_repo.commit("Initial commit")?;

        // Delete one file and commit
        fs::remove_file(test_repo.repo_path.join("delete.txt"))?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Delete file")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should have one deleted file
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Removed);
        assert!(change.current_path.ends_with("delete.txt"));

        Ok(())
    }

    #[test]
    fn test_renamed_file_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // First commit
        test_repo.create_file("original.txt", "content")?;
        test_repo.stage_all()?;
        let commit1 = test_repo.commit("Initial commit")?;

        // Rename file and commit
        Command::new("git")
            .args(["mv", "original.txt", "renamed.txt"])
            .current_dir(&test_repo.repo_path)
            .output()?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Rename file")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should have one changed file that was renamed
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Modified);
        assert!(change.current_path.ends_with("renamed.txt"));
        assert!(change.old_path.is_some());
        if let Some(old_path) = &change.old_path {
            assert!(old_path.ends_with("original.txt"));
        }

        Ok(())
    }

    // Complex scenarios
    #[test]
    fn test_multiple_changes_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // First commit with initial files
        test_repo.create_file("keep.txt", "content to keep")?;
        test_repo.create_file("modify.txt", "content to change")?;
        test_repo.create_file("delete.txt", "content to delete")?;
        test_repo.stage_all()?;
        let commit1 = test_repo.commit("Initial commit")?;

        // Make multiple changes
        test_repo.create_file("new.txt", "new content")?;
        test_repo.modify_file("modify.txt", "modified content")?;
        fs::remove_file(test_repo.repo_path.join("delete.txt"))?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Multiple changes")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should have three changes
        assert_eq!(changes.len(), 3);

        // Find each change by path
        let added = changes.iter().find(|c| c.current_path.ends_with("new.txt"));
        let modified = changes
            .iter()
            .find(|c| c.current_path.ends_with("modify.txt"));
        let deleted = changes
            .iter()
            .find(|c| c.current_path.ends_with("delete.txt"));

        // Verify all changes were found
        assert!(added.is_some());
        assert!(modified.is_some());
        assert!(deleted.is_some());

        // Check the properties of each change
        assert_eq!(added.unwrap().change_type, ChangeType::Added);
        assert_eq!(modified.unwrap().change_type, ChangeType::Modified);
        assert_eq!(deleted.unwrap().change_type, ChangeType::Removed);

        Ok(())
    }

    #[test]
    fn test_symlink_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // First commit with a regular file
        test_repo.create_file("target.txt", "target content")?;
        test_repo.stage_all()?;
        let commit1 = test_repo.commit("Add target file")?;

        // Create a symlink to the target file
        test_repo.create_symlink("link.txt", "target.txt")?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Add symlink")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should have one added symlink
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert_eq!(change.file_type, FileType::Symlink);
        assert!(change.current_path.ends_with("link.txt"));
        assert!(change.old_path.is_none());

        Ok(())
    }

    #[test]
    fn test_directory_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // First commit
        let commit1 = test_repo.create_and_commit_file("file.txt", "content")?;

        // Create a new directory with a file in it
        let dir_path = test_repo.repo_path.join("new_dir");
        fs::create_dir(&dir_path)?;
        test_repo.create_file("new_dir/file_in_dir.txt", "content in dir")?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Add directory with file")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should have one file change in the directory
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert_eq!(change.file_type, FileType::File);
        assert!(change.current_path.ends_with("new_dir/file_in_dir.txt"));

        Ok(())
    }

    #[test]
    fn test_ignored_file_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create a .gitignore file in the first commit
        test_repo.create_file(".gitignore", "*.log\n")?;
        test_repo.stage_all()?;
        let commit1 = test_repo.commit("Add gitignore")?;

        // Add an ignored file and a tracked file
        test_repo.create_file("ignored.log", "log content")?;
        test_repo.create_file("tracked.txt", "tracked content")?;

        // Stage and commit only the tracked file
        // (the ignored file should not appear in git status)
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Add tracked file")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should only include the tracked file, not the ignored one
        assert_eq!(changes.len(), 1);

        // Verify the only change is the tracked file, not the ignored one
        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Added);
        assert!(change.current_path.ends_with("tracked.txt"));

        Ok(())
    }

    #[test]
    fn test_file_mode_change_between_commits() -> Result<(), Box<dyn std::error::Error>> {
        // Skip this test on Windows as file permissions work differently
        if cfg!(windows) {
            return Ok(());
        }

        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create a file in the first commit
        test_repo.create_file("script.sh", "#!/bin/sh\necho 'Hello'")?;
        test_repo.stage_all()?;
        let commit1 = test_repo.commit("Add script")?;

        // Change file permissions to make it executable
        let file_path = test_repo.repo_path.join("script.sh");
        Command::new("chmod")
            .args(["+x", file_path.to_str().unwrap()])
            .output()?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Make script executable")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let changes =
            git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // Should have one modified file with changed permissions
        assert_eq!(changes.len(), 1);

        let change = &changes[0];
        assert_eq!(change.change_type, ChangeType::Modified);
        assert!(change.current_path.ends_with("script.sh"));

        Ok(())
    }

    // Commit diff specific tests
    #[test]
    fn test_changes_with_default_head() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // First commit
        let commit1 = test_repo.create_and_commit_file("file1.txt", "content")?;

        // Add another file and commit
        test_repo.create_file("file2.txt", "content")?;
        test_repo.stage_all()?;
        test_repo.commit("Second commit")?;

        // Get changes between first commit and HEAD (without specifying to_ref)
        let git_vcs = GitVcs;
        let changes = git_vcs.get_changes_between(&test_repo.repo_path, &commit1, None)?;

        // Should have one added file
        assert_eq!(changes.len(), 1);
        assert!(changes[0].current_path.ends_with("file2.txt"));
        assert_eq!(changes[0].change_type, ChangeType::Added);

        Ok(())
    }

    #[test]
    fn test_changes_on_different_branch() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Initial commit on main branch
        let main_commit = test_repo.create_and_commit_file("main.txt", "main content")?;

        // Create and checkout a new branch
        Command::new("git")
            .args(["checkout", "-b", "feature-branch"])
            .current_dir(&test_repo.repo_path)
            .output()?;

        // Add a file only on the feature branch
        test_repo.create_file("feature.txt", "feature content")?;
        test_repo.stage_all()?;
        let feature_commit = test_repo.commit("Feature commit")?;

        // Get changes between main commit and feature branch commit
        let git_vcs = GitVcs;
        let changes = git_vcs.get_changes_between(
            &test_repo.repo_path,
            &main_commit,
            Some(&feature_commit),
        )?;

        // Should have one added file on the feature branch
        assert_eq!(changes.len(), 1);
        assert!(changes[0].current_path.ends_with("feature.txt"));
        assert_eq!(changes[0].change_type, ChangeType::Added);

        Ok(())
    }

    #[test]
    fn test_changes_with_merge_commit() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Initial commit on main branch
        let main_commit = test_repo.create_and_commit_file("main.txt", "main content")?;

        // Create and checkout a feature branch
        Command::new("git")
            .args(["checkout", "-b", "feature-branch"])
            .current_dir(&test_repo.repo_path)
            .output()?;

        // Add a file on the feature branch
        test_repo.create_file("feature.txt", "feature content")?;
        test_repo.stage_all()?;
        let _ = test_repo.commit("Feature commit")?;

        // Go back to main and add a different file
        Command::new("git")
            .args(["checkout", "master"])
            .current_dir(&test_repo.repo_path)
            .output()?;
        test_repo.create_file("main2.txt", "more main content")?;
        test_repo.stage_all()?;
        test_repo.commit("Second main commit")?;

        // Merge feature branch into main
        Command::new("git")
            .args(["merge", "feature-branch", "-m", "Merge feature branch"])
            .current_dir(&test_repo.repo_path)
            .output()?;

        // Get commit hash of the merge commit (HEAD)
        let merge_commit = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&test_repo.repo_path)
            .output()?;
        let merge_commit_hash = String::from_utf8(merge_commit.stdout)?.trim().to_string();

        // Get changes between initial commit and merge commit
        let git_vcs = GitVcs;
        let changes = git_vcs.get_changes_between(
            &test_repo.repo_path,
            &main_commit,
            Some(&merge_commit_hash),
        )?;

        // Should have two added files (main2.txt and feature.txt)
        assert_eq!(changes.len(), 2);

        // Find each change by path
        let main_file = changes
            .iter()
            .find(|c| c.current_path.ends_with("main2.txt"));
        let feature_file = changes
            .iter()
            .find(|c| c.current_path.ends_with("feature.txt"));

        // Verify both changes were found
        assert!(main_file.is_some());
        assert!(feature_file.is_some());

        // Both should be "Added" type
        assert_eq!(main_file.unwrap().change_type, ChangeType::Added);
        assert_eq!(feature_file.unwrap().change_type, ChangeType::Added);

        Ok(())
    }

    #[test]
    fn test_changes_with_invalid_reference() {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new().unwrap();

        // Try to get changes with an invalid reference
        let git_vcs = GitVcs;
        let result = git_vcs.get_changes_between(&test_repo.repo_path, "non-existent-ref", None);

        // Should return an error
        assert!(result.is_err());

        // Verify it's the expected error type
        if let Err(AppError::GitOperationFailed {
            operation: _,
            reason: _,
        }) = result
        {
            // Test passed
        } else {
            panic!("Expected GitOperationFailed error, got: {:?}", result);
        }
    }

    #[test]
    fn test_changes_with_ancestry_path() -> Result<(), Box<dyn std::error::Error>> {
        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Initial commit
        let initial_commit = test_repo.create_and_commit_file("common.txt", "common content")?;

        // Create two branches with different files

        // First branch
        Command::new("git")
            .args(["checkout", "-b", "branch1"])
            .current_dir(&test_repo.repo_path)
            .output()?;
        test_repo.create_file("branch1.txt", "branch1 content")?;
        test_repo.stage_all()?;
        let branch1_commit = test_repo.commit("Branch1 commit")?;

        // Go back to initial commit and create second branch
        Command::new("git")
            .args(["checkout", &initial_commit])
            .current_dir(&test_repo.repo_path)
            .output()?;
        Command::new("git")
            .args(["checkout", "-b", "branch2"])
            .current_dir(&test_repo.repo_path)
            .output()?;
        test_repo.create_file("branch2.txt", "branch2 content")?;
        test_repo.stage_all()?;
        let branch2_commit = test_repo.commit("Branch2 commit")?;

        // Get changes between the two branch tips
        let git_vcs = GitVcs;
        let changes = git_vcs.get_changes_between(
            &test_repo.repo_path,
            &branch1_commit,
            Some(&branch2_commit),
        )?;

        // Should show changes in branch2 compared to branch1
        // Since they diverge, this will typically show both:
        // 1. The added file in branch2
        // 2. The "removal" of the branch1 file (since it doesn't exist in branch2)
        assert_eq!(changes.len(), 2);

        // Find each change by type and path
        let branch1_file = changes
            .iter()
            .find(|c| c.current_path.ends_with("branch1.txt"));
        let branch2_file = changes
            .iter()
            .find(|c| c.current_path.ends_with("branch2.txt"));

        // Verify both changes were found
        assert!(branch1_file.is_some());
        assert!(branch2_file.is_some());

        // The branch1 file should be removed, the branch2 file should be added
        assert_eq!(branch1_file.unwrap().change_type, ChangeType::Removed);
        assert_eq!(branch2_file.unwrap().change_type, ChangeType::Added);

        Ok(())
    }

    #[test]
    fn test_submodule_changes() -> Result<(), Box<dyn std::error::Error>> {
        // This test is more complex and may require setup of a separate repository
        // to use as a submodule. For simplicity, we'll just check if the function
        // doesn't crash when a submodule might be present.

        // Skip if git version is too old for submodule support
        // (This is a simplified version, in practice you might want to check the actual git version)
        let git_version = Command::new("git").args(["--version"]).output()?;
        if !String::from_utf8_lossy(&git_version.stdout).contains("git version") {
            return Ok(());
        }

        // Setup a test repository
        let test_repo = test_utils::TestRepo::new()?;

        // Create a commit so we have something to compare against
        let commit1 = test_repo.create_and_commit_file("file.txt", "content")?;

        // Try adding an empty directory that would be a submodule mount point
        // (This won't actually create a submodule but tests the code path)
        let submodule_dir = test_repo.repo_path.join("submodule");
        fs::create_dir(&submodule_dir)?;
        test_repo.create_file("submodule/.git", "gitdir: ../.git/modules/submodule")?;
        test_repo.stage_all()?;
        let commit2 = test_repo.commit("Add potential submodule")?;

        // Get changes between the two commits
        let git_vcs = GitVcs;
        let _ = git_vcs.get_changes_between(&test_repo.repo_path, &commit1, Some(&commit2))?;

        // We don't assert specific behavior since we're not actually creating a submodule,
        // we just ensure the function runs without crashing

        Ok(())
    }
}
