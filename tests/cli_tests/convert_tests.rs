use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_multiple_target_outputs_with_claude_fixture() {
    let fixture_dir = env!("FIXTURE_CLAUDE_CODE_DIR");
    let temp_dir = tempdir().unwrap();

    let json_output_path = temp_dir.path().join("rules.json");
    let claude_output_dir = temp_dir.path().join(".claude").join("skills");

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("convert")
        .arg(fixture_dir)
        .arg("-o")
        .arg(format!("ir-json:{}", json_output_path.display()))
        .arg("-o")
        .arg(format!("claude:{}", claude_output_dir.display()))
        .assert()
        .success();

    assert!(json_output_path.exists(), "JSON output was not created");
    let json_content = fs::read_to_string(&json_output_path).unwrap();
    assert!(
        json_content.contains("\"kind\": \"rule\"") || json_content.contains("\"kind\": \"skill\"")
    );

    assert!(
        claude_output_dir.exists(),
        "Claude skills directory was not created"
    );
    assert!(claude_output_dir.is_dir());

    let claude_files: Vec<_> = fs::read_dir(&claude_output_dir)
        .unwrap()
        .map(|r| r.unwrap().path())
        .collect();

    assert!(!claude_files.is_empty(), "Claude skills directory is empty");
}
