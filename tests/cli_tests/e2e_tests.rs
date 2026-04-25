use assert_cmd::Command;
use insta::assert_snapshot;
use std::str;

#[test]
fn test_e2e_parse_conductor_fixture() {
    let fixture_dir = std::env::var("FIXTURE_CONDUCTOR_DIR")
        .unwrap_or_else(|_| env!("FIXTURE_CONDUCTOR_DIR").to_string());
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("parse")
        .arg(fixture_dir)
        .arg("--from")
        .arg("auto")
        .assert()
        .success();

    let output = str::from_utf8(&assert.get_output().stdout).unwrap();

    // Sort entities to ensure deterministic snapshot
    let json: serde_json::Value = serde_json::from_str(output).unwrap();
    let mut entities = json.get("entities").unwrap().as_array().unwrap().clone();
    entities.sort_by(|a, b| {
        let name_a = a
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        let name_b = b
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        name_a.cmp(name_b)
    });

    let sorted_json = serde_json::json!({ "entities": entities });
    let normalized_output = serde_json::to_string_pretty(&sorted_json)
        .unwrap()
        .replace("\r\n", "\n")
        .replace("\\r\\n", "\\n");

    // The fixture is real and large, so we just snapshot a portion or the whole thing.
    // For deterministic output, let's snapshot it.
    assert_snapshot!(normalized_output);
}

#[test]
fn test_e2e_parse_and_transform_agency_agents_fixture() {
    let fixture_dir = std::env::var("FIXTURE_AGENCY_AGENTS_DIR")
        .unwrap_or_else(|_| env!("FIXTURE_AGENCY_AGENTS_DIR").to_string());
    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    let mut parse_cmd = std::process::Command::new(&cargo_bin);
    parse_cmd
        .arg("parse")
        .arg(fixture_dir)
        .arg("--from")
        .arg("auto")
        .stdout(std::process::Stdio::piped());

    let mut parse_child = parse_cmd.spawn().expect("Failed to spawn parse command");

    let mut transform_cmd = std::process::Command::new(&cargo_bin);
    transform_cmd
        .arg("transform")
        .arg("-")
        .arg("--set")
        .arg("e2e_tested=true")
        .stdin(parse_child.stdout.take().unwrap())
        .stdout(std::process::Stdio::piped());

    let mut transform_child = transform_cmd
        .spawn()
        .expect("Failed to spawn transform command");

    let mut emit_cmd = std::process::Command::new(&cargo_bin);
    let output = emit_cmd
        .arg("emit")
        .arg("-")
        .arg("-o")
        .arg("ir-json:-")
        .stdin(transform_child.stdout.take().unwrap())
        .output()
        .expect("Failed to execute emit command");

    assert!(
        output.status.success(),
        "Pipeline failed: {}",
        str::from_utf8(&output.stderr).unwrap()
    );
    let _ = parse_child.wait();
    let _ = transform_child.wait();

    let output_str = str::from_utf8(&output.stdout).unwrap();
    let json: serde_json::Value = serde_json::from_str(output_str).unwrap();
    let mut entities = json.get("entities").unwrap().as_array().unwrap().clone();

    entities.sort_by(|a, b| {
        let name_a = a
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        let name_b = b
            .get("metadata")
            .and_then(|m| m.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");
        name_a.cmp(name_b)
    });

    let sorted_json = serde_json::json!({ "entities": entities });
    let normalized_output = serde_json::to_string_pretty(&sorted_json)
        .unwrap()
        .replace("\r\n", "\n")
        .replace("\\r\\n", "\\n");

    assert_snapshot!(normalized_output);
}
