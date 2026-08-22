use rulette::publication::fs::{
    open_root, validate_distinct_paths, validate_relative_path, RegularFileMetadata,
};
use rulette::ResourcePath;
use std::fs;

fn resource_path(path: &str) -> ResourcePath {
    ResourcePath::parse(path).unwrap()
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn root_relative_helpers_create_read_and_describe_regular_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = open_root(temporary.path()).unwrap();
    let path = resource_path("rules/example.md");

    root.create_parent_directories(&path).unwrap();
    root.write_new_regular(&path, b"follow these rules", false)
        .unwrap();

    assert_eq!(
        root.read_regular(&path).unwrap(),
        Some(b"follow these rules".to_vec())
    );
    assert_eq!(
        root.regular_metadata(&path).unwrap(),
        Some(RegularFileMetadata {
            byte_length: 18,
            executable: false,
        })
    );
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn root_relative_helpers_do_not_replace_existing_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = open_root(temporary.path()).unwrap();
    let path = resource_path("AGENTS.md");

    root.write_new_regular(&path, b"first", false).unwrap();
    let error = root.write_new_regular(&path, b"second", false).unwrap_err();

    assert!(error.to_string().contains("already exists"));
    assert_eq!(root.read_regular(&path).unwrap(), Some(b"first".to_vec()));
}

#[test]
fn validation_rejects_repository_control_reserved_and_nonportable_names() {
    for path in [
        ".git/config",
        ".GIT/config",
        "rules/CON.md",
        "rules/AUX",
        "rules/name.",
        "rules/name ",
        "rules/name:stream.md",
        "rules/résumé.md",
    ] {
        let error = validate_relative_path(&resource_path(path)).unwrap_err();
        assert!(
            error.to_string().contains("publication path"),
            "{path}: {error:#}"
        );
    }
}

#[test]
fn validation_rechecks_malformed_resource_paths_at_the_filesystem_boundary() {
    let malformed: ResourcePath = serde_json::from_str(r#""../escape""#).unwrap();

    let error = validate_relative_path(&malformed).unwrap_err();

    assert!(error.to_string().contains("resource path"));
}

#[test]
fn validation_rejects_case_normalization_collisions() {
    let paths = [
        resource_path("rules/Review.md"),
        resource_path("rules/review.md"),
    ];

    let error = validate_distinct_paths(paths.iter()).unwrap_err();

    assert!(error.to_string().contains("normalization collision"));
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn root_relative_metadata_rejects_directories() {
    let temporary = tempfile::tempdir().unwrap();
    fs::create_dir(temporary.path().join("rules")).unwrap();
    let root = open_root(temporary.path()).unwrap();

    let error = root.regular_metadata(&resource_path("rules")).unwrap_err();

    assert!(error.to_string().contains("not a regular file"));
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn root_relative_helpers_reject_symlinked_roots_leaves_and_parent_directories() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().unwrap();
    let root_path = temporary.path().join("root");
    let outside = temporary.path().join("outside");
    fs::create_dir(&root_path).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("target.md"), "outside").unwrap();
    symlink(&root_path, temporary.path().join("root-link")).unwrap();
    symlink(outside.join("target.md"), root_path.join("link.md")).unwrap();
    symlink(&outside, root_path.join("linked-dir")).unwrap();

    let root_error = match open_root(temporary.path().join("root-link")) {
        Ok(_) => panic!("a symlinked root must be rejected"),
        Err(error) => error,
    };
    assert!(root_error.to_string().contains("symlink"));

    let root = open_root(&root_path).unwrap();
    let leaf_error = root.read_regular(&resource_path("link.md")).unwrap_err();
    assert!(leaf_error.to_string().contains("symlink"));

    let parent_error = root
        .create_parent_directories(&resource_path("linked-dir/new.md"))
        .unwrap_err();
    assert!(parent_error.to_string().contains("symlink"));
    assert!(!outside.join("new.md").exists());
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
#[test]
fn root_opening_refuses_platforms_without_safe_descriptor_operations() {
    let temporary = tempfile::tempdir().unwrap();

    let error = match open_root(temporary.path()) {
        Ok(_) => panic!("an unsupported platform must refuse publication"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("unsupported on this platform"));
}
