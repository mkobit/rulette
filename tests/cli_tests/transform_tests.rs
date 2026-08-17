use assert_cmd::Command;
use serde_json::Value;

fn get_fixture_path() -> String {
    "tests/fixtures/transform/multi_entity.json".to_string()
}

#[test]
fn test_transform_filter() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("--filter")
        .arg("license == \"MIT\"")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();

    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(
        entities[0]["metadata"]["name"].as_str().unwrap(),
        "test-skill-1"
    );
}

#[test]
fn test_transform_exclude() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("--exclude")
        .arg("license == \"MIT\"")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();

    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 1);
    assert_eq!(
        entities[0]["metadata"]["name"].as_str().unwrap(),
        "test-skill-2"
    );
}

#[test]
fn test_transform_rename() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("--rename")
        .arg("rename_me=renamed_key")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();

    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 2);

    let skill_1 = &entities[0];
    assert!(skill_1["metadata"].get("rename_me").is_none());
    assert_eq!(
        skill_1["metadata"]["renamed_key"].as_str().unwrap(),
        "old_value"
    );
}

#[test]
fn test_transform_set() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("--set")
        .arg("new_key=new_value")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();

    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 2);

    for entity in entities {
        assert_eq!(entity["metadata"]["new_key"].as_str().unwrap(), "new_value");
    }
}

#[test]
fn test_transform_chained_filter_set() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("--filter")
        .arg("license == \"Apache-2.0\"")
        .arg("--set")
        .arg("injected=true")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();

    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 1);

    let entity = &entities[0];
    assert_eq!(entity["metadata"]["name"].as_str().unwrap(), "test-skill-2");
    assert_eq!(entity["metadata"]["injected"].as_str().unwrap(), "true");
}

#[test]
fn test_transform_filter_by_kind() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("--filter")
        .arg("kind == \"skill\"")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();

    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 2);
    for entity in entities {
        assert_eq!(entity["kind"].as_str().unwrap(), "skill");
    }
}

#[test]
fn test_transform_filter_does_not_match_body_substring() {
    // Regression test: a body containing the filter expression as literal
    // text must not cause a false match now that the raw-JSON-substring
    // fallback has been removed. The field genuinely doesn't exist here,
    // so the filter must exclude everything, not accidentally include it.
    let input = r#"{
      "entities": [
        {
          "kind": "rule",
          "metadata": { "name": "r1" },
          "body": "the string status == \"stable\" appears in this body"
        }
      ]
    }"#;

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("-")
        .arg("--filter")
        .arg("status == \"stable\"")
        .write_stdin(input)
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();
    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 0);
}

#[test]
fn test_transform_filter_rejects_malformed_expression() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("--filter")
        .arg("not-a-valid-expr")
        .assert()
        .failure()
        .stderr(predicates::str::contains("Invalid filter expression"));
}

#[cfg(unix)]
#[test]
fn test_transform_multi_target_rollback_on_write_failure() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let ok_target = temp_dir.path().join("ok_target");
    let readonly_target = temp_dir.path().join("readonly_target");
    fs::create_dir_all(&ok_target).unwrap();
    fs::create_dir_all(&readonly_target).unwrap();
    fs::set_permissions(&readonly_target, fs::Permissions::from_mode(0o555)).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", ok_target.display()))
        .arg("-o")
        .arg(format!("ir-json:{}/sub", readonly_target.display()))
        .assert()
        .failure();

    // Restore write permission so the tempdir can clean itself up.
    fs::set_permissions(&readonly_target, fs::Permissions::from_mode(0o755)).unwrap();

    let entries: Vec<_> = fs::read_dir(&ok_target).unwrap().collect();
    assert!(
        entries.is_empty(),
        "expected no partial writes in ok_target after multi-target failure, found: {:?}",
        entries
    );
}

#[test]
fn test_transform_quiet_suppresses_emitted_message_but_writes_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("-q")
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .success();

    let stdout = assert.get_output().stdout.clone();
    assert!(
        stdout.is_empty(),
        "expected no stdout output under -q, got: {:?}",
        String::from_utf8_lossy(&stdout)
    );
    assert!(out_dir.join("ir.json").exists());
}

#[test]
fn test_transform_strict_collision_detection() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform").arg("-");

    // Two skills with the same name
    let input = r#"
    {
      "entities": [
        {
          "kind": "skill",
          "metadata": {
            "name": "pdf-processing",
            "description": "Desc 1"
          },
          "body": "Body 1"
        },
        {
          "kind": "skill",
          "metadata": {
            "name": "pdf-processing",
            "description": "Desc 2"
          },
          "body": "Body 2"
        }
      ]
    }
    "#;

    cmd.write_stdin(input)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Identity collision detected: entity 'pdf-processing' already exists",
        ));
}
