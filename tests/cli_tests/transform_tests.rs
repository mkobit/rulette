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

#[cfg(unix)]
#[test]
fn test_transform_unchanged_target_is_not_rewritten() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .success();

    let target = out_dir.join("ir.json");

    // Make the file and its parent read-only; a spurious write would fail,
    // proving the "unchanged" path really is skipped rather than rewritten.
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444)).unwrap();
    std::fs::set_permissions(&out_dir, std::fs::Permissions::from_mode(0o555)).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .success();

    std::fs::set_permissions(&out_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains(&format!("Unchanged {}", target.display())));
}

#[test]
fn test_transform_changed_target_is_rewritten_and_reported_as_updated() {
    let temp_dir = tempfile::tempdir().unwrap();
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let target = out_dir.join("ir.json");
    std::fs::write(&target, "{\"stale\": true}").unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains(&format!("Updated {}", target.display())));

    let content = std::fs::read_to_string(&target).unwrap();
    assert_ne!(content, "{\"stale\": true}");
    let json: Value = serde_json::from_str(&content).unwrap();
    assert!(json.get("entities").is_some());
}

#[test]
fn test_transform_new_target_is_created_and_reported() {
    let temp_dir = tempfile::tempdir().unwrap();
    let out_dir = temp_dir.path().join("out");
    let target = out_dir.join("ir.json");

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains(&format!("Created {}", target.display())));
    assert!(target.exists());
}

#[test]
fn test_transform_mixed_statuses_reported_independently_in_one_invocation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let unchanged_dir = temp_dir.path().join("unchanged");
    let updated_dir = temp_dir.path().join("updated");
    let created_dir = temp_dir.path().join("created");
    std::fs::create_dir_all(&unchanged_dir).unwrap();
    std::fs::create_dir_all(&updated_dir).unwrap();

    // Seed unchanged_dir with exactly what this invocation will render.
    let mut seed_cmd = Command::cargo_bin("rulette").unwrap();
    seed_cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", unchanged_dir.display()))
        .assert()
        .success();

    // Seed updated_dir with stale content that differs from what will be rendered.
    std::fs::write(updated_dir.join("ir.json"), "{\"stale\": true}").unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", unchanged_dir.display()))
        .arg("-o")
        .arg(format!("ir-json:{}", updated_dir.display()))
        .arg("-o")
        .arg(format!("ir-json:{}", created_dir.display()))
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(stdout.contains(&format!(
        "Unchanged {}",
        unchanged_dir.join("ir.json").display()
    )));
    assert!(stdout.contains(&format!(
        "Updated {}",
        updated_dir.join("ir.json").display()
    )));
    assert!(stdout.contains(&format!(
        "Created {}",
        created_dir.join("ir.json").display()
    )));

    assert!(created_dir.join("ir.json").exists());
    let updated_content = std::fs::read_to_string(updated_dir.join("ir.json")).unwrap();
    assert_ne!(updated_content, "{\"stale\": true}");
}

#[cfg(unix)]
#[test]
fn test_transform_rollback_restores_updated_targets_original_content() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let ok_target = temp_dir.path().join("ok_target");
    let readonly_target = temp_dir.path().join("readonly_target");
    std::fs::create_dir_all(&ok_target).unwrap();
    std::fs::create_dir_all(&readonly_target).unwrap();

    let ok_file = ok_target.join("ir.json");
    std::fs::write(&ok_file, "{\"original\": true}").unwrap();

    std::fs::set_permissions(&readonly_target, std::fs::Permissions::from_mode(0o555)).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", ok_target.display()))
        .arg("-o")
        .arg(format!("ir-json:{}/sub", readonly_target.display()))
        .assert()
        .failure();

    std::fs::set_permissions(&readonly_target, std::fs::Permissions::from_mode(0o755)).unwrap();

    let content = std::fs::read_to_string(&ok_file).unwrap();
    assert_eq!(
        content, "{\"original\": true}",
        "rollback should restore the overwritten target's original content, not delete it"
    );
}

#[cfg(unix)]
#[test]
fn test_transform_rollback_telescopes_to_true_pre_invocation_content_for_shared_path() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let shared_file = temp_dir.path().join("shared.json");
    std::fs::write(&shared_file, "{\"pre_invocation\": true}").unwrap();

    let readonly_target = temp_dir.path().join("readonly_target");
    std::fs::create_dir_all(&readonly_target).unwrap();
    std::fs::set_permissions(&readonly_target, std::fs::Permissions::from_mode(0o555)).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", shared_file.display()))
        .arg("-o")
        .arg(format!("ir-json:{}", shared_file.display()))
        .arg("-o")
        .arg(format!("ir-json:{}/sub", readonly_target.display()))
        .assert()
        .failure();

    std::fs::set_permissions(&readonly_target, std::fs::Permissions::from_mode(0o755)).unwrap();

    let content = std::fs::read_to_string(&shared_file).unwrap();
    assert_eq!(
        content, "{\"pre_invocation\": true}",
        "rollback must telescope back to the true pre-invocation content, not an intermediate write"
    );
}

#[cfg(unix)]
#[test]
fn test_transform_symlink_existing_target_aborts_before_any_writes() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().unwrap();
    let ok_dir = temp_dir.path().join("ok");
    std::fs::create_dir_all(&ok_dir).unwrap();

    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let elsewhere = temp_dir.path().join("elsewhere.json");
    std::fs::write(&elsewhere, "{}").unwrap();
    symlink(&elsewhere, out_dir.join("ir.json")).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", ok_dir.display()))
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .failure()
        .stderr(predicates::str::contains("not a regular file"));

    let entries: Vec<_> = std::fs::read_dir(&ok_dir).unwrap().collect();
    assert!(
        entries.is_empty(),
        "no writes should happen anywhere in the invocation, found: {:?}",
        entries
    );
}

#[test]
fn test_transform_directory_existing_target_aborts_before_any_writes() {
    let temp_dir = tempfile::tempdir().unwrap();
    let ok_dir = temp_dir.path().join("ok");
    std::fs::create_dir_all(&ok_dir).unwrap();

    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    // A directory sitting exactly where the rendered "ir.json" file would go.
    std::fs::create_dir_all(out_dir.join("ir.json")).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", ok_dir.display()))
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .failure()
        .stderr(predicates::str::contains("not a regular file"));

    let entries: Vec<_> = std::fs::read_dir(&ok_dir).unwrap().collect();
    assert!(
        entries.is_empty(),
        "no writes should happen anywhere in the invocation, found: {:?}",
        entries
    );
}

#[test]
fn test_transform_non_utf8_existing_target_aborts_before_any_writes() {
    let temp_dir = tempfile::tempdir().unwrap();
    let ok_dir = temp_dir.path().join("ok");
    std::fs::create_dir_all(&ok_dir).unwrap();

    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    std::fs::write(out_dir.join("ir.json"), [0xFF, 0xFE, 0x00, 0xFF]).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", ok_dir.display()))
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .failure()
        .stderr(predicates::str::contains("Cannot read existing target"));

    let entries: Vec<_> = std::fs::read_dir(&ok_dir).unwrap().collect();
    assert!(
        entries.is_empty(),
        "no writes should happen anywhere in the invocation, found: {:?}",
        entries
    );
}

#[test]
fn test_transform_check_with_no_drift_exits_zero_and_writes_nothing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut seed_cmd = Command::cargo_bin("rulette").unwrap();
    seed_cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .success();

    let target = out_dir.join("ir.json");
    let content_before = std::fs::read_to_string(&target).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .arg("--check")
        .assert()
        .success();

    let content_after = std::fs::read_to_string(&target).unwrap();
    assert_eq!(content_before, content_after);

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains(&format!("Unchanged {}", target.display())));
}

#[test]
fn test_transform_check_with_drift_exits_nonzero_and_creates_nothing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let out_dir = temp_dir.path().join("out"); // does not exist yet

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .arg("--check")
        .assert()
        .failure();

    assert!(
        !out_dir.exists(),
        "check mode must not create parent directories that didn't already exist"
    );
}

#[test]
fn test_transform_check_with_only_stdout_targets_fails() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("--check")
        .assert()
        .failure()
        .stderr(predicates::str::contains("at least one output file target"));
}

#[test]
fn test_transform_quiet_check_produces_no_stdout_output() {
    let temp_dir = tempfile::tempdir().unwrap();
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut seed_cmd = Command::cargo_bin("rulette").unwrap();
    seed_cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("-q")
        .arg("transform")
        .arg(get_fixture_path())
        .arg("-o")
        .arg(format!("ir-json:{}", out_dir.display()))
        .arg("--check")
        .assert()
        .success();

    let stdout = assert.get_output().stdout.clone();
    assert!(
        stdout.is_empty(),
        "expected no stdout under -q --check, got: {:?}",
        String::from_utf8_lossy(&stdout)
    );
}
