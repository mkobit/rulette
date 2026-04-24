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
fn test_transform_dedup() {
    let dup_file = "tests/fixtures/transform/dup_entity.json";
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(dup_file)
        .arg("--dedup")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();

    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 1);
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
fn test_transform_dedup_conflict_error() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg("--dedup")
        .arg("--on-conflict")
        .arg("error")
        .arg("-");

    // Two skills with the same name but different descriptions
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
            "Conflict detected for entity 'pdf-processing'",
        ));
}

#[test]
fn test_transform_dedup_conflict_take_first() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg("--dedup")
        .arg("--on-conflict")
        .arg("take-first")
        .arg("-");

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

    let output = cmd
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out_str = String::from_utf8(output).unwrap();
    assert!(out_str.contains("Desc 1"));
    assert!(!out_str.contains("Desc 2"));
}

#[test]
fn test_transform_dedup_conflict_take_last() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg("--dedup")
        .arg("--on-conflict")
        .arg("take-last")
        .arg("-");

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

    let output = cmd
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out_str = String::from_utf8(output).unwrap();
    assert!(!out_str.contains("Desc 1"));
    assert!(out_str.contains("Desc 2"));
}
