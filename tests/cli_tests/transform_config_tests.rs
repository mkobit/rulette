use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

#[test]
#[cfg(any(target_os = "linux", target_os = "android"))]
fn explicit_selection_only_config_compiles_a_graph() {
    let temporary = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
    let publication = tempfile::tempdir().unwrap();
    let project_root = publication.path().join("project");
    std::fs::create_dir(&project_root).unwrap();
    let fixture = std::fs::canonicalize("tests/fixtures/v0_1/codex").unwrap();
    let config = toml::to_string(&serde_json::json!({
        "inputs": [fixture.to_str().unwrap()],
        "targets": [{ "target": "codex", "scope": "project" }],
        "select": []
    }))
    .unwrap();
    std::fs::write(temporary.path(), config).unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("--config")
        .arg(temporary.path())
        .arg("--project-root")
        .arg(project_root)
        .arg("--stage")
        .arg(publication.path().join("stage"))
        .assert()
        .success();
}

#[test]
fn transform_config_rejects_mutation_and_authority_fields_before_compilation() {
    for config in [
        "pipeline = []",
        "strict = true",
        "stage = \"outside-the-plan\"",
        "targets = [{ target = \"codex\", scope = \"project\", path = \"AGENTS.md\" }]",
    ] {
        let mut temporary = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
        temporary.write_all(config.as_bytes()).unwrap();
        let mut command = Command::cargo_bin("rulette").unwrap();
        command
            .arg("transform")
            .arg("--config")
            .arg(temporary.path())
            .assert()
            .failure()
            .stderr(predicate::str::contains("unknown field"));
    }
}

#[test]
fn transform_config_requires_a_strictly_sorted_select_array() {
    let mut temporary = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
    temporary
        .write_all(b"inputs = []\nselect = [\"pkg_b\", \"pkg_a\"]\n")
        .unwrap();
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("--config")
        .arg(temporary.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("strictly sorted"));
}
