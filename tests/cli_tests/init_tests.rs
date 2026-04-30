use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_init_creates_files() {
    let temp_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("rulette").unwrap();

    cmd.arg("init").arg(temp_dir.path()).assert().success();

    assert!(temp_dir.path().join("rules").exists());
    assert!(temp_dir.path().join("rules/example.md").exists());
    assert!(temp_dir.path().join("RULETTE.toml").exists());

    let config_content = fs::read_to_string(temp_dir.path().join("RULETTE.toml")).unwrap();
    assert!(config_content.contains("[transform]"));
}

#[test]
fn test_init_force_overwrites() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("RULETTE.toml");
    fs::create_dir_all(temp_dir.path()).unwrap();
    fs::write(&config_path, "original content").unwrap();

    // Without force, should not overwrite (actually my current implementation doesn't check content, just existence)
    // Let's verify it doesn't overwrite by default (existing behavior check)
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("init").arg(temp_dir.path()).assert().success();

    let content = fs::read_to_string(&config_path).unwrap();
    assert_eq!(content, "original content");

    // With force, should overwrite
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("init")
        .arg(temp_dir.path())
        .arg("--force")
        .assert()
        .success();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[transform]"));
}
