use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;

fn get_fixture_path() -> String {
    "tests/fixtures/transform/multi_entity.json".to_string()
}

#[test]
fn test_config_inputs_and_cli_positional_inputs_conflict() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("rulette.transform.jsonc");
    fs::write(&config_path, r#"{"inputs": ["./rules/"]}"#).unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(get_fixture_path())
        .arg("--config")
        .arg(&config_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("inputs"));
}

#[test]
fn test_config_inputs_used_when_cli_positionals_empty() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("rulette.transform.jsonc");
    let fixture = std::env::current_dir().unwrap().join(get_fixture_path());
    fs::write(
        &config_path,
        format!(r#"{{"inputs": [{:?}]}}"#, fixture.to_str().unwrap()),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg("--config")
        .arg(&config_path)
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();
    let entities = json.get("entities").unwrap().as_array().unwrap();
    assert_eq!(entities.len(), 2);
}

#[test]
fn test_config_pipeline_composes_with_cli_filter() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("rulette.transform.jsonc");
    fs::write(
        &config_path,
        r#"{"pipeline": [{"rename": "rename_me=renamed_key"}]}"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("--config")
        .arg(&config_path)
        .arg("--filter")
        .arg("license == \"MIT\"")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();
    let entities = json.get("entities").unwrap().as_array().unwrap();

    // The config's rename step ran, then the CLI's filter step narrowed to
    // one entity -- proving both applied, config first, in that order.
    assert_eq!(entities.len(), 1);
    assert_eq!(
        entities[0]["metadata"]["name"].as_str().unwrap(),
        "test-skill-1"
    );
    assert!(entities[0]["metadata"].get("rename_me").is_none());
    assert_eq!(
        entities[0]["metadata"]["renamed_key"].as_str().unwrap(),
        "old_value"
    );
}

#[test]
fn test_cli_output_flag_replaces_config_outputs_entirely() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("rulette.transform.jsonc");
    let never_written = temp_dir.path().join("should_not_exist");
    fs::write(
        &config_path,
        format!(
            r#"{{"outputs": [{{"target": "claude", "path": {:?}}}]}}"#,
            never_written.to_str().unwrap()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("transform")
        .arg(get_fixture_path())
        .arg("--config")
        .arg(&config_path)
        .arg("-o")
        .arg("ir-json:-")
        .assert()
        .success();

    let output = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    let json: Value = serde_json::from_str(output).unwrap();
    assert!(json.get("entities").is_some());
    assert!(
        !never_written.exists(),
        "config's outputs must not be used once a CLI -o flag is present"
    );
}

fn write_lossy_rule_fixture(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("rule.md");
    fs::write(&path, "---\nname: just-a-rule\n---\nRule body").unwrap();
    path
}

#[test]
fn test_per_output_strict_escalates_independently_of_sibling_output() {
    let temp_dir = tempfile::tempdir().unwrap();
    let rule_path = write_lossy_rule_fixture(temp_dir.path());
    let config_path = temp_dir.path().join("rulette.transform.jsonc");
    fs::write(
        &config_path,
        r#"{
            "outputs": [
                {"target": "agent-skills", "path": "-", "strict": true},
                {"target": "ir-json", "path": "-"}
            ]
        }"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("transform")
        .arg(&rule_path)
        .arg("--config")
        .arg(&config_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error: Lossy conversion"));
}

#[test]
fn test_per_output_strict_relaxes_below_global_strict() {
    let temp_dir = tempfile::tempdir().unwrap();
    let rule_path = write_lossy_rule_fixture(temp_dir.path());
    let config_path = temp_dir.path().join("rulette.transform.jsonc");
    fs::write(
        &config_path,
        r#"{
            "outputs": [
                {"target": "agent-skills", "path": "-", "strict": false}
            ]
        }"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("--strict")
        .arg("transform")
        .arg(&rule_path)
        .arg("--config")
        .arg(&config_path)
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning: Lossy conversion"));
}

fn scaffold(input_dir: &std::path::Path, inputs: &[&str], out_path: &std::path::Path) -> Value {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.current_dir(input_dir).arg("transform");
    for input in inputs {
        cmd.arg(input);
    }
    cmd.arg("--to")
        .arg("transform-config")
        .arg("--out")
        .arg(out_path)
        .assert()
        .success();

    let content = fs::read_to_string(out_path).unwrap();
    serde_json::from_str(&content).unwrap()
}

#[test]
fn test_scaffold_distinguishes_cursor_mdc_and_cursor_mcp() {
    let input_dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(input_dir.path().join(".cursor/rules")).unwrap();
    fs::write(
        input_dir.path().join(".cursor/rules/typescript.mdc"),
        "---\ndescription: ts rule\n---\nBody.",
    )
    .unwrap();
    fs::write(
        input_dir.path().join(".cursor/mcp.json"),
        r#"{"mcpServers": {"test": {"command": "echo", "args": [], "env": {}}}}"#,
    )
    .unwrap();

    let out_path = input_dir.path().join("rulette.transform.jsonc");
    let manifest = scaffold(
        input_dir.path(),
        &[".cursor/rules/typescript.mdc", ".cursor/mcp.json"],
        &out_path,
    );

    let outputs = manifest.get("outputs").unwrap().as_array().unwrap();
    let targets: Vec<&str> = outputs
        .iter()
        .map(|o| o["target"].as_str().unwrap())
        .collect();
    assert!(targets.contains(&"cursor-mdc"));
    assert!(targets.contains(&"cursor-mcp"));
    assert_eq!(targets.len(), 2);
}

#[test]
fn test_scaffold_bare_claude_md_file() {
    let input_dir = tempfile::tempdir().unwrap();
    fs::write(input_dir.path().join("CLAUDE.md"), "Project rules.").unwrap();

    let out_path = input_dir.path().join("rulette.transform.jsonc");
    let manifest = scaffold(input_dir.path(), &["CLAUDE.md"], &out_path);

    let outputs = manifest.get("outputs").unwrap().as_array().unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0]["target"].as_str().unwrap(), "claude");
}

#[test]
fn test_scaffold_antigravity_directory() {
    let input_dir = tempfile::tempdir().unwrap();
    let agy_dir = input_dir.path().join(".antigravity");
    fs::create_dir_all(&agy_dir).unwrap();
    fs::write(agy_dir.join("rules.md"), "Antigravity rules.").unwrap();

    let out_path = input_dir.path().join("rulette.transform.jsonc");
    let manifest = scaffold(
        input_dir.path(),
        &[".antigravity/rules.md"],
        &out_path,
    );

    let outputs = manifest.get("outputs").unwrap().as_array().unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0]["target"].as_str().unwrap(), "antigravity");
    assert_eq!(outputs[0]["path"].as_str().unwrap(), ".antigravity/");
}

#[test]
fn test_scaffold_unmatched_path_preserved_with_warning() {
    let input_dir = tempfile::tempdir().unwrap();
    fs::write(
        input_dir.path().join("misc.md"),
        "Not a recognized tool path.",
    )
    .unwrap();

    let out_path = input_dir.path().join("rulette.transform.jsonc");
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.current_dir(input_dir.path())
        .arg("transform")
        .arg("misc.md")
        .arg("--to")
        .arg("transform-config")
        .arg("--out")
        .arg(&out_path)
        .assert()
        .success()
        .stderr(predicate::str::contains("no known tool convention matched"));

    let manifest: Value = serde_json::from_str(&fs::read_to_string(&out_path).unwrap()).unwrap();
    let inputs = manifest.get("inputs").unwrap().as_array().unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].as_str().unwrap(), "misc.md");
    assert!(manifest
        .get("outputs")
        .unwrap()
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn test_scaffold_dedups_nested_codex_agents_md_and_round_trips() {
    let input_dir = tempfile::tempdir().unwrap();
    fs::write(input_dir.path().join("AGENTS.md"), "Always write tests.").unwrap();
    fs::create_dir_all(input_dir.path().join("src/backend")).unwrap();
    fs::write(
        input_dir.path().join("src/backend/AGENTS.md"),
        "Use Go idioms.",
    )
    .unwrap();

    let out_path = input_dir.path().join("rulette.transform.jsonc");
    let manifest = scaffold(
        input_dir.path(),
        &["AGENTS.md", "src/backend/AGENTS.md"],
        &out_path,
    );

    let inputs = manifest.get("inputs").unwrap().as_array().unwrap();
    assert_eq!(
        inputs
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["AGENTS.md", "src/backend/AGENTS.md"]
    );
    let outputs = manifest.get("outputs").unwrap().as_array().unwrap();
    assert_eq!(
        outputs.len(),
        1,
        "nested AGENTS.md inputs must dedup to one codex entry"
    );
    assert_eq!(outputs[0]["target"].as_str().unwrap(), "codex");

    // Round-trip acceptance criterion: re-running against the unmodified
    // fixture reports every target unchanged.
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .current_dir(input_dir.path())
        .arg("transform")
        .arg("--config")
        .arg(&out_path)
        .arg("--check")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    assert!(stdout.contains("Unchanged AGENTS.md"));
    assert!(stdout.contains(&format!(
        "Unchanged {}",
        std::path::Path::new("src")
            .join("backend")
            .join("AGENTS.md")
            .display()
    )));
}

#[test]
fn test_inspect_rejects_transform_config_target() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("inspect")
        .arg(get_fixture_path())
        .arg("--to")
        .arg("transform-config")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "only a valid target for the `transform` command",
        ));
}
