use assert_cmd::Command;

fn coverage_json(ir: &str) -> serde_json::Value {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let output = cmd
        .arg("-q")
        .arg("inspect")
        .arg("-")
        .arg("--coverage")
        .arg("--json")
        .write_stdin(ir)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).expect("--coverage --json output should be valid JSON")
}

#[test]
fn test_coverage_multiple_entities_same_kind_roll_up_to_worst_status() {
    // Two mcp-server entities targeting cursor-mcp: one fully representable
    // (no extra metadata), one only partially (populated extra). The
    // aggregated cell must report the worst status seen, not the best.
    let ir = r#"{
      "entities": [
        {
          "kind": "mcp-server",
          "metadata": { "name": "clean-server" },
          "config": { "command": "echo", "args": [], "env": {} }
        },
        {
          "kind": "mcp-server",
          "metadata": { "name": "lossy-server", "some_extension_key": "value" },
          "config": { "command": "echo", "args": [], "env": {} }
        }
      ]
    }"#;

    let entries = coverage_json(ir);
    let cursor_mcp_entry = entries
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["target"] == "cursor-mcp" && e["entity_kind"] == "mcp-server")
        .expect("expected a cursor-mcp/mcp-server entry");

    assert_eq!(cursor_mcp_entry["status"], "lossy");
}

#[test]
fn test_coverage_matrix_reflects_actual_input() {
    // Only rule and skill entities in the input -- the matrix must not
    // include rows for hook/agent/mcp-server/permissions.
    let ir = r#"{
      "entities": [
        { "kind": "rule", "metadata": {}, "body": "A rule." },
        { "kind": "skill", "metadata": { "name": "s", "description": "d" }, "body": "A skill." }
      ]
    }"#;

    let entries = coverage_json(ir);
    let kinds: std::collections::BTreeSet<String> = entries
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["entity_kind"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(
        kinds,
        ["rule", "skill"].into_iter().map(String::from).collect()
    );
}

#[test]
fn test_coverage_and_to_are_mutually_exclusive() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("inspect")
        .arg("-")
        .arg("--coverage")
        .arg("--to")
        .arg("claude")
        .write_stdin(r#"{"entities": []}"#)
        .assert()
        .failure()
        .stderr(predicates::str::contains("cannot be used with"));
}

#[test]
fn test_coverage_strict_exits_nonzero_when_lossy_or_dropped_present() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("-q")
        .arg("inspect")
        .arg("-")
        .arg("--coverage")
        .arg("--strict")
        .write_stdin(r#"{"entities": [{"kind": "rule", "metadata": {}, "body": "A rule."}]}"#)
        .assert()
        .failure();
}

#[test]
fn test_coverage_strict_exits_zero_when_matrix_is_empty() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("-q")
        .arg("inspect")
        .arg("-")
        .arg("--coverage")
        .arg("--strict")
        .write_stdin(r#"{"entities": []}"#)
        .assert()
        .success();
}

#[test]
fn test_coverage_without_strict_always_exits_zero() {
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("-q")
        .arg("inspect")
        .arg("-")
        .arg("--coverage")
        .write_stdin(r#"{"entities": [{"kind": "rule", "metadata": {}, "body": "A rule."}]}"#)
        .assert()
        .success();
}

#[test]
fn test_coverage_json_shape_matches_spec() {
    let ir = r#"{
      "entities": [
        { "kind": "rule", "metadata": {}, "body": "A rule." }
      ]
    }"#;

    let entries = coverage_json(ir);
    let entries = entries.as_array().unwrap();
    assert!(!entries.is_empty());

    for entry in entries {
        let obj = entry.as_object().unwrap();
        assert!(obj.contains_key("target"));
        assert!(obj.contains_key("entity_kind"));
        assert!(obj.contains_key("status"));
        let status = obj["status"].as_str().unwrap();
        assert!(["supported", "lossy", "dropped"].contains(&status));

        if status == "supported" {
            assert!(
                !obj.contains_key("reason"),
                "supported entries should omit reason, got: {entry}"
            );
        } else {
            assert!(
                obj["reason"].is_string() && !obj["reason"].as_str().unwrap().is_empty(),
                "lossy/dropped entries must carry a non-null reason, got: {entry}"
            );
        }
    }

    // Rule -> Cursor MCP is a known Dropped case; assert it directly.
    let cursor_mcp_rule = entries
        .iter()
        .find(|e| e["target"] == "cursor-mcp" && e["entity_kind"] == "rule")
        .unwrap();
    assert_eq!(cursor_mcp_rule["status"], "dropped");
    assert!(cursor_mcp_rule["reason"].as_str().unwrap().contains("Rule"));
}

#[test]
fn test_inspect_to_strict_warning_text_unchanged_by_coverage_change() {
    // Regression guard: the pre-existing single-target `inspect --to
    // <format> --strict` behavior (exact warning wording, exit code) must
    // be byte-identical to before the coverage-reporting refactor.
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("-q")
        .arg("--strict")
        .arg("inspect")
        .arg("-")
        .arg("--to")
        .arg("cursor-mdc")
        .write_stdin(r#"{"entities": [{"kind": "hook", "metadata": {"name": "PreToolUse"}}]}"#)
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "Lossy conversion: Hook to Cursor MDC drops metadata",
        ));

    // Non-strict: the exact named warning text must still appear, and the
    // command must still succeed (existing behavior, unchanged).
    let mut cmd = Command::cargo_bin("rulette").unwrap();
    cmd.arg("-q")
        .arg("inspect")
        .arg("-")
        .arg("--to")
        .arg("cursor-mdc")
        .write_stdin(r#"{"entities": [{"kind": "hook", "metadata": {"name": "PreToolUse"}}]}"#)
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "Warning: Lossy conversion: Hook 'PreToolUse' to Cursor MDC drops metadata",
        ));
}
