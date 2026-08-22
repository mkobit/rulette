use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

#[test]
fn explicit_selection_only_config_compiles_a_graph() {
    let temporary = tempfile::Builder::new().suffix(".toml").tempfile().unwrap();
    let fixture = std::fs::canonicalize("tests/fixtures/v0_1/codex").unwrap();
    writeln!(
        &mut temporary.reopen().unwrap(),
        "inputs = [\"{}\"]\ntargets = [{{ target = \"codex\", scope = \"project\" }}]\nselect = []",
        fixture.display()
    )
    .unwrap();

    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("--config")
        .arg(temporary.path())
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
