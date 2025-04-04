use super::*;

use cargo_metadata::semver::Version;
use cargo_metadata::{DependencyBuilder, MetadataBuilder, PackageBuilder, PackageId};
use std::path::PathBuf;

use crate::vcs::{ChangeType, FileType};

fn create_test_crate(name: &str, path: &str) -> CrateInfo {
    CrateInfo {
        name: name.to_string(),
        path: PathBuf::from(path),
    }
}

fn create_test_metadata() -> Metadata {
    let pkg1 = PackageBuilder::new(
        "crate1",
        Version::new(1, 0, 0),
        PackageId {
            repr: "crate1".to_string(),
        },
        "/workspace/crate1/Cargo.toml",
    )
    .build()
    .unwrap();

    let dep = DependencyBuilder::default()
        .name("crate1")
        .kind(cargo_metadata::DependencyKind::Normal)
        .req(cargo_metadata::semver::VersionReq::parse("1.0.0").unwrap())
        .optional(false)
        .uses_default_features(true)
        .source(None)
        .target(None)
        .features(vec![])
        .rename(None)
        .registry(None)
        .path(None)
        .build()
        .unwrap();

    let pkg2 = PackageBuilder::new(
        "crate2",
        Version::new(1, 0, 0),
        PackageId {
            repr: "crate2".to_string(),
        },
        "/workspace/crate2/Cargo.toml",
    )
    .dependencies(vec![dep])
    .build()
    .unwrap();

    let pkg3 = PackageBuilder::new(
        "crate3",
        Version::new(1, 0, 0),
        PackageId {
            repr: "crate3".to_string(),
        },
        "/workspace/crate3/Cargo.toml",
    )
    .build()
    .unwrap();

    MetadataBuilder::default()
        .packages(vec![pkg1, pkg2, pkg3])
        .workspace_root("/workspace")
        .target_directory("/workspace/target")
        .workspace_members(vec![])
        .workspace_default_members(cargo_metadata::WorkspaceDefaultMembers::default())
        .workspace_metadata(serde_json::Value::Null)
        .resolve(None)
        .version(4usize)
        .build()
        .unwrap()
}

#[test]
fn test_get_workspace_crates() {
    let metadata = create_test_metadata();
    let crates = get_workspace_crates(&metadata).unwrap();

    let expected_crates = HashSet::from([
        create_test_crate("crate1", "/workspace/crate1"),
        create_test_crate("crate2", "/workspace/crate2"),
        create_test_crate("crate3", "/workspace/crate3"),
    ]);

    assert_eq!(crates.0, expected_crates);
}

#[test]
fn test_find_crate_for_file() {
    let crates = Crates(HashSet::from([
        create_test_crate("crate1", "/workspace/crate1"),
        create_test_crate("crate2", "/workspace/crate2"),
        create_test_crate("nested", "/workspace/crate2/nested"),
    ]));

    // Test exact path match
    let result1 = find_crate_for_file(Path::new("/workspace/crate1"), &crates);
    assert!(result1.is_some());
    assert_eq!(result1.unwrap().name, "crate1");

    // Test file in crate
    let result2 = find_crate_for_file(Path::new("/workspace/crate1/src/main.rs"), &crates);
    assert!(result2.is_some());
    assert_eq!(result2.unwrap().name, "crate1");

    // Test nested crate (should match the most specific path)
    let result3 = find_crate_for_file(Path::new("/workspace/crate2/nested/src/lib.rs"), &crates);
    assert!(result3.is_some());
    assert_eq!(result3.unwrap().name, "nested");

    // Test non-matching path
    let result4 = find_crate_for_file(Path::new("/some/other/path"), &crates);
    assert!(result4.is_none());
}

#[test]
fn test_find_changed_crates() {
    let crates = Crates(HashSet::from([
        create_test_crate("crate1", "/workspace/crate1"),
        create_test_crate("crate2", "/workspace/crate2"),
        create_test_crate("crate3", "/workspace/crate3"),
        create_test_crate("crate4", "/workspace/crate4"),
    ]));

    let changed_files = vec![
        ChangedFile {
            old_path: None,
            current_path: PathBuf::from("/workspace/crate1/src/lib.rs"),
            file_type: FileType::File,
            change_type: ChangeType::Modified,
        },
        ChangedFile {
            old_path: Some(PathBuf::from("/workspace/crate3/Cargo.toml")),
            current_path: PathBuf::from("/workspace/crate2/Cargo.toml"),
            file_type: FileType::File,
            change_type: ChangeType::Modified,
        },
        ChangedFile {
            old_path: None,
            current_path: PathBuf::from("/workspace/README.md"),
            file_type: FileType::File,
            change_type: ChangeType::Modified,
        },
    ];

    let result = find_changed_crates(&changed_files, &crates).unwrap();

    assert_eq!(result.len(), 3);
    assert!(result.contains(&"crate1".to_string()));
    assert!(result.contains(&"crate2".to_string()));
    assert!(result.contains(&"crate3".to_string()));
}

#[test]
fn test_find_dependent_crates() {
    let metadata = create_test_metadata();
    let crate1_name = "crate1".to_string();
    let changed_crates = IndexSet::from([&crate1_name]);

    let result = find_dependent_crates(&changed_crates, &metadata).unwrap();

    assert_eq!(result.len(), 1);
    assert!(result.contains(&"crate2".to_string()));

    // Test with no dependencies
    let crate2_name = "crate2".to_string();
    let changed_crates2 = IndexSet::from([&crate2_name]);
    let result2 = find_dependent_crates(&changed_crates2, &metadata).unwrap();
    assert_eq!(result2.len(), 0);
}

#[test]
fn test_verify_crates_exist() {
    let metadata = create_test_metadata();

    // Test with existing crates
    let crates_exist = vec!["crate1".to_string(), "crate2".to_string()];
    let result = verify_crates_exist(&metadata, &crates_exist);
    assert!(result.is_ok());

    // Test with non-existing crate
    let crates_not_exist = vec!["crate1".to_string(), "nonexistent".to_string()];
    let result = verify_crates_exist(&metadata, &crates_not_exist);
    assert!(result.is_err());
    match result {
        Err(AppError::UnknownCrate { crate_name }) => {
            assert_eq!(crate_name, "nonexistent");
        }
        _ => panic!("Expected UnknownCrate error"),
    }
}
