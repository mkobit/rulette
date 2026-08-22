use assert_cmd::Command;
use predicates::prelude::*;

const CODEX_FIXTURE: &str = "tests/fixtures/v0_1/codex";

fn graph_from(command: &mut Command) -> serde_json::Value {
    let output = command.assert().success().get_output().stdout.clone();
    serde_json::from_slice(&output).expect("transform must write a compilation graph JSON value")
}

#[test]
fn transform_outputs_a_graph_and_selects_exact_package_ids() {
    let mut full_command = Command::cargo_bin("rulette").unwrap();
    full_command.arg("transform").arg(CODEX_FIXTURE);
    let full_graph = graph_from(&mut full_command);
    let ids: Vec<_> = full_graph["packages"]
        .as_object()
        .expect("graph packages are keyed by package ID")
        .keys()
        .cloned()
        .collect();
    assert_eq!(
        ids.len(),
        2,
        "the fixture has one rule and one skill package"
    );

    let mut selected_command = Command::cargo_bin("rulette").unwrap();
    selected_command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--select")
        .arg(&ids[1]);
    let selected_graph = graph_from(&mut selected_command);
    let selected_ids: Vec<_> = selected_graph["packages"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(selected_ids, vec![ids[1].clone()]);
}

#[test]
fn selection_union_is_deterministic_and_unknown_package_ids_fail() {
    let mut full_command = Command::cargo_bin("rulette").unwrap();
    full_command.arg("transform").arg(CODEX_FIXTURE);
    let full_graph = graph_from(&mut full_command);
    let ids: Vec<_> = full_graph["packages"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();

    let mut selected_command = Command::cargo_bin("rulette").unwrap();
    selected_command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--select")
        .arg(&ids[1])
        .arg("--select")
        .arg(&ids[0])
        .arg("--select")
        .arg(&ids[1]);
    let selected_graph = graph_from(&mut selected_command);
    let selected_ids: Vec<_> = selected_graph["packages"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(selected_ids, ids);

    let mut unknown_command = Command::cargo_bin("rulette").unwrap();
    unknown_command
        .arg("transform")
        .arg(CODEX_FIXTURE)
        .arg("--select")
        .arg("pkg_0000000000000000000000000000000000000000000000000000000000000000")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown package ID"));
}

#[test]
fn removed_mutation_flags_fail_during_argument_parsing() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("does-not-need-to-exist")
        .arg("--filter")
        .arg("kind == \"rule\"")
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument '--filter'"));
}

#[test]
fn legacy_entity_interchange_is_not_a_graph_frontend() {
    let mut command = Command::cargo_bin("rulette").unwrap();
    command
        .arg("transform")
        .arg("-")
        .arg("--from")
        .arg("ir-json")
        .write_stdin(r#"{"entities": []}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'ir-json'"));
}
